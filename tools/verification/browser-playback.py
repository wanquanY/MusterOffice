"""Run the packaged playback SDK in an explicitly selected installed browser.

Uses a temporary empty profile, loopback-only immutable allowlisted bytes, and
owned fixtures. No browser automation dependency, user profile or installation.
"""
import argparse
import importlib.util
import json
import mimetypes
from pathlib import Path
import platform
import secrets
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from browser_playback.corpus import prepare, sha

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('playback_build', ROOT / 'tools/playback-sdk/build.py')
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


def run(args):
    args.output.mkdir(parents=True, exist_ok=False)
    bundle = build.verify(args.bundle, args.pin)
    owners, expected, assets, inputs = prepare(args.author, args.source, args.native_reference)
    routes = {}
    for file in bundle['files']:
        data = (args.bundle / file['path']).read_bytes()
        assert len(data) == file['byteLength'] and sha(data) == file['sha256']
        routes['sdk/' + file['path']] = data
    for name in ['run.mjs', 'lifecycle.mjs', 'fault-worker.mjs', 'assert.mjs']:
        path = ROOT / 'tools/verification/browser_playback' / name
        routes[name] = path.read_bytes()
        inputs[str(path.relative_to(ROOT))] = sha(routes[name])
    modules = {}
    for key, name in [('kernel', 'mo_wasm_bg.wasm'), ('raster', 'mo-skia.wasm'), ('text', 'mo-hb.wasm')]:
        url = 'sdk/runtime/' + name
        modules[key] = dict(url=url, byteLength=len(routes[url]), sha256=sha(routes[url]))
    routes.update(assets)
    routes['corpus.json'] = json.dumps(dict(owners=owners, modules=modules, stepped=args.stepped)).encode()
    routes[''] = b'<!doctype html><meta charset="utf-8"><title>MusterOffice verification</title><body>running<script type="module" src="./run.mjs"></script>'
    prefix = '/' + secrets.token_hex(16) + '/'
    complete, received, errors, requests = threading.Event(), set(), [], []
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def respond(self, status, body=b'', content_type='text/plain'):
            self.send_response(status)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(body)))
            self.send_header('Cache-Control', 'no-store')
            self.send_header('X-Content-Type-Options', 'nosniff')
            self.send_header('Content-Security-Policy', "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; connect-src 'self'")
            if args.isolated:
                self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
                self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
            self.end_headers()
            self.wfile.write(body)

        def route(self):
            if self.headers.get('Host') != f'127.0.0.1:{server.server_port}' or not self.path.startswith(prefix):
                return None
            return self.path[len(prefix):]

        def do_GET(self):
            key = self.route()
            if key not in routes:
                self.respond(404)
                return
            with lock:
                requests.append(key)
            mime = 'text/html' if key == '' else 'text/javascript' if key.endswith('.mjs') else mimetypes.guess_type(key)[0] or 'application/octet-stream'
            self.respond(200, routes[key], mime)

        def do_POST(self):
            key = self.route()
            try:
                if key is None or self.headers.get('Origin') != f'http://127.0.0.1:{server.server_port}':
                    raise ValueError('Unexpected report origin')
                count = int(self.headers['Content-Length'])
                if not 0 < count <= 64 * 1024 * 1024:
                    raise ValueError('Report limit')
                data = self.rfile.read(count)
                if len(data) != count:
                    raise ValueError('Incomplete body')
                if key.startswith('frames/') and key[7:].isdigit():
                    index = int(key[7:])
                    assert 0 <= index < len(expected) and data == expected[index]['pixels'], 'Frame differs from native bytes'
                    with lock:
                        assert index not in received, 'Duplicate frame'
                        with (args.output / f'{index:03}.rgba').open('xb') as f:
                            f.write(data)
                        received.add(index)
                elif key == 'report':
                    with (args.output / 'browser.json').open('xb') as f:
                        f.write(data)
                    complete.set()
                else:
                    raise ValueError('Unknown report path')
                self.respond(200, b'ok')
            except Exception as error:
                with lock:
                    errors.append(str(error))
                self.respond(400, str(error).encode())
                complete.set()

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    version = subprocess.check_output([str(args.browser), '--version'], text=True).strip()
    try:
        with tempfile.TemporaryDirectory(prefix='mo-browser-') as profile, (args.output / 'browser.log').open('x') as log:
            command = [str(args.browser), '--headless', '--no-first-run', '--no-default-browser-check',
                       '--disable-background-networking', '--disable-component-update',
                       '--user-data-dir=' + profile, f'http://127.0.0.1:{server.server_port}{prefix}']
            browser = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
            try:
                if not complete.wait(60):
                    raise TimeoutError('Browser verification deadline')
            finally:
                browser.terminate()
                try:
                    browser.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    browser.kill()
                    browser.wait(timeout=10)
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
    assert not errors, errors
    report = json.loads((args.output / 'browser.json').read_text())
    assert report['status'] == 'passed', report
    assert report['crossOriginIsolated'] == args.isolated
    assert report['stepped'] == args.stepped
    assert len(received) == report['frameCount'] == len(expected)
    for owner in report['observations']:
        for frame in owner['frames']:
            old = expected[frame['index']]
            assert frame['name'] == old['name'] and frame['info'] == old['info'], 'Native metadata differs'
            assert frame['pixelSha256'] == sha(old['pixels']) and frame['byteLength'] == len(old['pixels'])
            (args.output / f"{frame['index']:03}.json").write_text(json.dumps(frame['info']))
    for path, digest in inputs.items():
        assert sha(Path(path).read_bytes()) == digest, 'Input changed'
    build.verify(args.bundle, args.pin)
    value = dict(format='musteroffice.browser-playback/1', status='passed', browserVersion=version,
                 platform=platform.platform(), machine=platform.machine(), crossOriginIsolated=args.isolated,
                 bundleManifestSha256=args.pin, frameCount=len(received), ownerCount=len(owners), stepped=args.stepped,
                 cooperativeCancellations=report['cooperativeCancellations'],
                 nativeMetadataAndFullPixelBytesEqual=True, inputs=inputs, lifecycle=report['lifecycle'],
                 browserReportSha256=sha((args.output / 'browser.json').read_bytes()),
                 servedFiles=sorted(set(requests)), compiledModules=modules,
                 scope=report['scope'], performanceClaim=False, physicalMemoryReclamationClaim=False)
    (args.output / 'report.json').write_text(json.dumps(value, indent=2) + '\n')
    print(json.dumps({k: value[k] for k in ['status', 'browserVersion', 'crossOriginIsolated', 'frameCount', 'ownerCount']}))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ['output', 'bundle', 'author', 'source', 'native-reference', 'browser']:
        p.add_argument('--' + name, type=Path, required=True)
    p.add_argument('--pin', required=True)
    p.add_argument('--isolated', action='store_true')
    p.add_argument('--stepped', action='store_true', help='drive begin/step/take and cooperative cancellation through the real Worker')
    run(p.parse_args())


if __name__ == '__main__':
    main()
