"""Record the selected runtime/pipe dependencies from locked registry archives.

No network access, archive extraction, or machine paths in the component record.
Run from the repository root; --check validates without overwriting evidence.
"""
import hashlib
import json
import os
from pathlib import Path
import sys
import tarfile
import tomllib

selected = ['tokio', 'mio', 'socket2', 'wasi', 'windows-sys']
lock = tomllib.loads(Path('tools/mo-mcp/Cargo.lock').read_text())
packages = {p['name']: p for p in lock['package'] if p['name'] in selected}
assert set(packages) == set(selected)
base = Path(os.environ.get('CARGO_HOME', str(Path.home()/'.cargo')))/'registry/cache'
output = Path('components/io-runtime')
files = {}; records = []
for name in selected:
    package = packages[name]; version = package['version']; stem = name+'-'+version
    archives = list(base.glob('*/'+stem+'.crate')); assert len(archives) == 1, name
    raw = archives[0].read_bytes(); digest = hashlib.sha256(raw).hexdigest()
    assert digest == package['checksum']
    licenses = []
    with tarfile.open(archives[0], 'r:gz') as tar:
        manifest = tomllib.loads(tar.extractfile(stem+'/Cargo.toml').read().decode())['package']
        for member in tar.getmembers():
            path = Path(member.name)
            if len(path.parts) != 2 or not member.isfile(): continue
            if not any(word in path.name.lower() for word in ['license', 'copyright', 'notice']): continue
            data = tar.extractfile(member).read()
            target = 'licenses/'+stem+'-'+path.name+'.txt'
            files[target] = data
            licenses.append(dict(path=target, sha256=hashlib.sha256(data).hexdigest()))
    assert licenses, name
    records.append(dict(component=name, version=version, declaredLicense=manifest['license'],
        repository=manifest['repository'], archive=dict(url='https://static.crates.io/crates/'+name+'/'+stem+'.crate',
        byteLength=len(raw), sha256=digest), licenses=licenses))
report = dict(status='development-adapter-dependencies-not-release-accepted', components=records,
    scope='Tokio Unix pipe readiness in mo-mcp only. net feature enables OS readiness; no socket/listener is opened. WASI/Windows entries are target-dependent lockfile branches, not verified native binaries.',
    evidence='../../docs/implementation/mcp-recovery.md',
    remaining=['Full transitive distribution/license closure.', 'Windows cancellable I/O and actual cross-platform lifecycle acceptance.'])
files['component.json'] = (json.dumps(report, indent=2)+'\n').encode()
for name, data in files.items():
    path = output/name
    if '--check' in sys.argv: assert path.read_bytes() == data, str(path)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        assert not path.exists(), 'do not overwrite recorded component files'
        path.write_bytes(data)
print(json.dumps(dict(components=len(records), recordedFiles=len(files), check='--check' in sys.argv)))
