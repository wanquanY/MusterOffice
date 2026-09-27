"""Independent portable/compatibility manifest launcher for package verification.

This is a test client, not a plugin manager or evidence of Codex activation.
No shell, network, global client configuration or user installation is used.
"""
import base64
import copy
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'mo-mcp'))
from compute_support import Session, create, export, read, sha, write


def extract_owned_archive(archive, destination):
    destination.mkdir(parents=True, exist_ok=False)
    with zipfile.ZipFile(archive) as source:
        for info in source.infolist():
            # This test still checks paths before extracting its own archive.
            relative = Path(info.filename)
            assert not relative.is_absolute() and '..' not in relative.parts
            assert not info.is_dir() and info.external_attr >> 16 & 0o170000 == 0o100000
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            with source.open(info) as incoming, target.open('xb') as outgoing:
                shutil.copyfileobj(incoming, outgoing, 1 << 20)
            target.chmod(info.external_attr >> 16 & 0o777)


def launch_spec(package, data, compatibility=False):
    tokens = {'PLUGIN_ROOT': str(package), 'PLUGIN_DATA': str(data)}

    def expand(value):
        # One pass. A token-looking substring in a real path is literal.
        return re.sub(r'\$\{(PLUGIN_ROOT|PLUGIN_DATA)\}', lambda m: tokens[m[1]], value)

    config = read(package / ('.mcp.json' if compatibility else 'mcp.json'))
    assert list(config['mcpServers']) == ['musteroffice']
    server = config['mcpServers']['musteroffice']
    assert server['type'] == 'stdio'
    command = server['command']
    assert command.startswith('./bin/') and '${' not in command and '..' not in Path(command).parts
    cwd = Path(expand(server.get('cwd', '${PLUGIN_ROOT}')))
    assert cwd == package
    argv = [str(package / command), *(expand(arg) for arg in server['args'])]
    # The installed program must not rely on repository cwd, Python, Node,
    # a Cargo target directory or component library environment overrides.
    env = dict(tokens, PATH=str(data / 'empty-executable-path'))
    for key in ('SYSTEMROOT', 'WINDIR'):
        if key in os.environ:
            env[key] = os.environ[key]
    return argv, cwd, env


class PackagedSession(Session):
    def __init__(self, package, data, files, era, name, compatibility=False):
        self.files, self.era, self.seq = files, era, 0
        self.directory = files.root / name
        self.directory.mkdir()
        self.stderr = self.directory / 'stderr.log'
        self.log = self.stderr.open('xb')
        argv, cwd, env = launch_spec(package, data, compatibility)
        write(self.directory / 'launch.json', dict(argv=argv, cwd=str(cwd), env=env))
        try:
            self.p = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, stderr=self.log)
        except BaseException:
            self.log.close()
            raise
        self.pending, self.tools = bytearray(), {}


def workflow(client, files, worker_sha):
    caps = client.tool('mo_capabilities', {})
    assert caps['businessJobs'] is False and caps['completePresentationCapability'] is False
    assert caps['exportRenderer']['implementationSha256'] == worker_sha
    for name in caps['schemas']:
        assert client.tool('mo_schema', {'id': name})['schema'] == read('contracts/generated/' + name + '.schema.json')
    fixture = read('fixtures/presentations/delivery/input.json')
    original = client.computed(files.invocation('created', create(fixture['document'])))['result']['snapshot']
    text = copy.deepcopy(original['document']['objects']['title:1']['content']['text'])
    text['paragraphs'][0]['runs'][0]['content']['text'] = 'A A A'
    request = dict(contractVersion=caps['contractVersion'], requestId='edited',
                   profileId='presentations-author-model-v01-draft', action=dict(kind='apply',
                   documentId=original['document']['id'], baseRevision=original['revision'],
                   operations=[dict(operationId='title', operation=dict(kind='setText', object='title:1', text=text))]))
    edited = client.computed(files.invocation('edited', dict(request=request, snapshot=original)))['result']['snapshot']
    assert edited['revision'] != original['revision']
    assert edited['document']['objects']['title:1']['content']['text'] == text
    observations = []
    for name, snapshot in [('exported', original), ('exported-edited', edited)]:
        invocation, inputs = export(files, snapshot, caps['exportRenderer'])
        client.computed(files.invocation(name, invocation, inputs))
        folder = files.outputs / name
        index = read(folder / 'files.json'); assert len(index) == 12
        for asset in index:
            data = (folder / asset['file']).read_bytes()
            assert sha(data) == asset['asset']['sha256'] and len(data) == int(asset['asset']['byteLength'])
            resource = client.call('resources/read', {'uri': f"musteroffice://output/{name}/{asset['file']}"})['result']['contents'][0]
            assert base64.b64decode(resource['blob'], validate=True) == data
            assert resource['_meta']['io.musteroffice/range']['sha256'] == sha(data)
        pptx = next(asset for asset in index if asset['asset']['role'] == 'pptx')
        data = (folder / pptx['file']).read_bytes()
        (files.inputs / 'source.pptx').write_bytes(data)
        binding = dict(file='source.pptx', info=dict(id='source', descriptor=dict(sha256=sha(data),
                       byteLength=str(len(data)), mediaType=pptx['asset']['mediaType']), verification='bytesSha256'))
        imp = dict(request=dict(contractVersion=caps['contractVersion'], requestId='import:' + name,
                   profileId='presentations-author-model-v01-draft', action=dict(kind='import',
                   documentId='import:' + name, source=dict(resourceId='source', assetId='source'))))
        imported = client.computed(files.invocation('imported-' + name, imp, [binding]))['result']['snapshot']
        assert imported['document']['sourceBindings'] and len(imported['document']['slideOrder']) == 2
        observations.append(dict(name=name, pptxSha256=sha(data), assets=len(index)))
    return observations


def compare_exports(left, right):
    for name in ('exported', 'exported-edited'):
        a, b = left / name, right / name
        for metadata in ('result.json', 'files.json', 'inspection.json'):
            assert (a / metadata).read_bytes() == (b / metadata).read_bytes(), (name, metadata)
        for asset in read(a / 'files.json'):
            assert (a / asset['file']).read_bytes() == (b / asset['file']).read_bytes(), (name, asset['file'])


def negative_startups(root, package, worker, caller):
    scratch = root / 'invalid-runtime'; scratch.mkdir()
    (scratch / 'bin').mkdir()
    runtime = read(package / 'runtime.json')
    manifest = scratch / 'runtime.json'
    target_worker = scratch / 'bin' / worker.name
    shutil.copy2(worker, target_worker)
    binary = next((package / 'bin').glob('mo-mcp*'))
    observations = []

    def reject(name, manifest_path=manifest, caller_path=caller):
        result = subprocess.run([str(binary), '--package', str(manifest_path), str(caller_path)],
                                stdin=subprocess.DEVNULL, capture_output=True, cwd=scratch, timeout=10,
                                env={'PATH': str(root / 'missing-path')})
        assert result.returncode != 0 and result.stderr and not result.stdout, (name, result)
        (root / f'rejected-{name}.log').write_bytes(result.stderr)
        observations.append(dict(case=name, exitCode=result.returncode, stderrSha256=sha(result.stderr)))

    reject('missing-manifest')
    for name, field, value in [('wrong-format', 'format', 'unsupported'), ('wrong-os', 'targetOs', 'unsupported'),
                               ('wrong-arch', 'targetArch', 'unsupported'), ('wrong-hash', 'workerSha256', '0' * 64),
                               ('unknown-field', 'workerPath', '/not-authorized')]:
        write(manifest, dict(runtime, **{field: value})); reject(name)
    manifest.write_text('{"format":"a","format":"b"}'); reject('duplicate-key')
    manifest.write_bytes(b' ' * 65537); reject('oversized-manifest')
    manifest.write_bytes(b'\xff'); reject('invalid-utf8')
    write(manifest, runtime)
    reject('relative-manifest', Path('runtime.json'))
    reject('relative-caller', manifest, Path('caller-files.json'))
    reject('missing-caller', manifest, scratch / 'missing-caller.json')
    override = read(caller)
    override['exportWorker'] = dict(path=str(worker), sha256=sha(worker.read_bytes()))
    override_path = scratch / 'override.json'; write(override_path, override)
    reject('worker-override', manifest, override_path)
    target_worker.write_bytes(b'damaged worker'); reject('damaged-worker')
    target_worker.unlink(); reject('missing-worker')
    target_worker.mkdir(); reject('directory-worker'); target_worker.rmdir()
    if os.name != 'nt':
        target_worker.symlink_to(worker); reject('symlink-worker'); target_worker.unlink()
        manifest.unlink(); manifest.symlink_to(package / 'runtime.json')
        reject('symlink-manifest')
    return observations
