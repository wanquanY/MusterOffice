"""Record optional HTTP development components from pinned local Cargo archives.

No extraction or network; full root license/notice texts are retained. This is
not distribution clearance for the complete transitive dependency closure.
"""
import hashlib
import json
import os
from pathlib import Path
import sys
import tarfile
import tomllib

selected = dict([
    ('async-trait', '0.1.92'), ('atomic-waker', '1.1.2'), ('axum', '0.8.9'), ('axum-core', '0.5.6'),
    ('base64', '0.23.1'), ('chacha20', '0.10.2'), ('cpufeatures', '0.3.1'), ('http', '1.5.0'),
    ('http-body', '1.1.0'), ('http-body-util', '0.1.4'), ('httparse', '1.10.1'), ('httpdate', '1.0.3'),
    ('hyper', '1.11.1'), ('hyper-util', '0.1.21'), ('matchit', '0.8.4'), ('mime', '0.3.17'),
    ('percent-encoding', '2.3.2'), ('rand', '0.10.3'), ('rand_core', '0.10.1'),
    ('signal-hook-registry', '1.4.8'), ('sse-stream', '0.2.6'), ('sync_wrapper', '1.0.2'),
    ('tokio-stream', '0.1.19'), ('tower', '0.5.3'), ('tower-layer', '0.3.3'), ('tower-service', '0.3.3'),
])
lock = tomllib.loads(Path('tools/mo-mcp/Cargo.lock').read_text())
packages = {p['name']: p for p in lock['package'] if selected.get(p['name']) == p['version']}
assert set(packages) == set(selected)
cache = Path(os.environ.get('CARGO_HOME', str(Path.home() / '.cargo'))) / 'registry/cache'
output = Path('components/http-runtime')
files = {}; records = []
for name, version in sorted(selected.items()):
    stem = name + '-' + version
    archives = list(cache.glob('*/' + stem + '.crate')); assert len(archives) == 1, stem
    raw = archives[0].read_bytes(); digest = hashlib.sha256(raw).hexdigest()
    assert digest == packages[name]['checksum']
    licenses = []
    with tarfile.open(archives[0], 'r:gz') as archive:
        manifest = tomllib.loads(archive.extractfile(stem + '/Cargo.toml').read().decode())['package']
        for member in sorted(archive.getmembers(), key=lambda m: m.name):
            path = Path(member.name)
            if len(path.parts) != 2 or not member.isfile():
                continue
            if not any(word in path.name.lower() for word in ('license', 'copyright', 'notice', 'copying')):
                continue
            data = archive.extractfile(member).read()
            target = 'licenses/' + stem + '-' + path.name + '.txt'
            files[target] = data
            licenses.append(dict(path=target, sha256=hashlib.sha256(data).hexdigest()))
    assert licenses, stem
    records.append(dict(component=name, version=version, declaredLicense=manifest['license'],
        repository=manifest['repository'], rustVersion=manifest.get('rust-version'),
        archive=dict(url='https://static.crates.io/crates/' + name + '/' + stem + '.crate',
                     byteLength=len(raw), sha256=digest), licenses=licenses))
report = dict(status='development-optional-http-not-release-cleared', components=records,
    protocolSdk=dict(component='../rmcp/versions/3.4.1/component.json', addedFeature='transport-streamable-http-server',
                     previousDefaultFeatures=['server', 'transport-io'], vendorPatches=False),
    scope='Optional mo-mcp/http only. Not dependencies of default stdio or the embedded SDK. Versions newly added to the independent MCP workspace lock; existing package versions unchanged.',
    evidence='../../docs/implementation/http-mcp.md',
    remaining=['Complete transitive distribution/license review.', 'Public product gateway and cross-platform native acceptance.'])
files['component.json'] = (json.dumps(report, indent=2) + '\n').encode()
for name, data in files.items():
    path = output / name
    if '--check' in sys.argv:
        assert path.read_bytes() == data, str(path)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        assert not path.exists(), 'preserve previous component record'
        path.write_bytes(data)
print(json.dumps(dict(components=len(records), recordedFiles=len(files), check='--check' in sys.argv)))
