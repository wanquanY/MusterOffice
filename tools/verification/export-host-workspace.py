"""Validate the export owner without overwriting any prior stage evidence."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path('.codex-work/export-host')
root.mkdir(exist_ok=True)
start = sys.argv[1] if len(sys.argv) > 1 else 'fmt'
env = dict(os.environ, CARGO_BUILD_JOBS='2', MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
commands = [
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('tests', ['cargo', 'test', '--workspace', '--locked', '--offline']),
    ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']),
    ('native-build', ['cargo', 'build', '--locked', '--offline', '-p', 'mo-cli', '-p', 'mo-host', '-p', 'mo-raster-worker']),
    ('schema-check', ['cargo', 'run', '--locked', '--offline', '-p', 'mo-contract-codegen', '--', 'check', 'contracts/generated']),
    ('types-check', ['pnpm', 'check:types']),
    ('pure-operation-wasm', ['cargo', 'check', '--locked', '--offline', '-p', 'mo-operation-service', '--target', 'wasm32-unknown-unknown']),
    ('rust-wasm', ['cargo', 'build', '--locked', '--offline', '--release', '-p', 'mo-wasm', '--target', 'wasm32-unknown-unknown']),
    ('bindgen', ['.codex-work/toolchain/bin/wasm-bindgen', 'target/wasm32-unknown-unknown/release/mo_wasm.wasm', '--target', 'nodejs', '--out-dir', str(root/'wasm-node')]),
    ('native-integration', ['cargo', 'test', '--locked', '--offline', '-p', 'mo-native-render', '--test', 'delivery', '--', '--ignored']),
    ('export-integration', ['cargo', 'test', '--locked', '--offline', '-p', 'mo-standard-host', '--test', 'exports', '--', '--ignored']),
    ('native-tree', ['cargo', 'tree', '-p', 'mo-host', '--edges', 'normal,build', '--locked', '--offline', '--format', '{p} | {l} | {f}']),
    ('wasm-tree', ['cargo', 'tree', '-p', 'mo-operation-service', '--target', 'wasm32-unknown-unknown', '--edges', 'normal,build', '--locked', '--offline', '--format', '{p} | {l} | {f}']),
]
names = [x[0] for x in commands]
assert start in names
report_path = root/'workspace.json'
records = json.loads(report_path.read_text()) if report_path.exists() else []
for name, argv in commands[names.index(start):]:
    attempt = 1
    while (root/f'{name}-{attempt}.log').exists(): attempt += 1
    path = root/f'{name}-{attempt}.log'
    worker = None
    if name.endswith('-integration'):
        p = Path('target/debug/mo-raster-worker').resolve()
        worker = dict(path=str(p), sha256=hashlib.sha256(p.read_bytes()).hexdigest())
        env.update(MO_DELIVERY_WORKER=worker['path'], MO_DELIVERY_WORKER_SHA256=worker['sha256'])
    with path.open('x') as log:
        run = subprocess.run(argv, env=env, stdout=log, stderr=subprocess.STDOUT)
    record = dict(name=name, argv=argv, exitCode=run.returncode, log=str(path))
    if worker: record['worker'] = worker
    records.append(record)
    report_path.write_text(json.dumps(records, indent=2)+'\n')
    print(json.dumps(record), flush=True)
    if run.returncode: raise SystemExit(run.returncode)
    if name == 'bindgen':
        (root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
