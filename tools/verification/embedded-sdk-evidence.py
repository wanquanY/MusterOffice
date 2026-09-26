"""Bind the local SDK and product preparation evidence without copying product code."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tomllib
from urllib.parse import unquote

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
product = args.product_repo.resolve()
stage = Path('.codex-work/embedded-sdk')
output = Path('docs/reviews/evidence/2026-09-26-embedded-sdk-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-product-office-manifest-verification.json')


def read(path):
    return json.loads(Path(path).read_text())


def entry(path, base=Path('.')):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=path.relative_to(base).as_posix(), byteLength=len(data),
                sha256=hashlib.sha256(data).hexdigest())


assert entry(parent_path)['sha256'] == 'abb68225847e9b777a76e1f0e6479b94e94b66ff615bbd3d24ab849cd4a17c8c'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {name for name, record in prior.items() if entry(name) != record}
assert changed == {'Cargo.lock', 'Cargo.toml', 'crates/mo-native-export/src/client.rs',
                   'tools/mo-export-worker/tests/native_export.rs', 'docs/README.md',
                   'docs/implementation/progress.md', 'docs/implementation/dependencies.md'}, changed
added = {'docs/implementation/embedded-sdk.md', 'tools/verification/embedded-sdk-evidence.py'}
for directory in ['tools/sdk', 'crates/mo-embedded-sdk']:
    added |= {p.as_posix() for p in Path(directory).rglob('*')
              if p.is_file() and '__pycache__' not in p.parts}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(path.read_text().splitlines()) <= 2000, name
    if path.suffix == '.md':
        for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text()):
            if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
                continue
            target = unquote(target.split('#')[0].split('?')[0])
            if target:
                resolved = (path.parent / target).resolve()
                assert resolved.exists() or resolved == output.resolve(), (name, target)
                links += 1

spec = importlib.util.spec_from_file_location('sdk_verify', 'tools/sdk/verify.py')
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)
sdk_digest = '6eb5e7ff2e791e4ef7a42712c46b68c51eb2c0f1f9c356d5929b88c9852b082e'
sdk = verifier.verify(stage / 'sdk-final', sdk_digest)
assert verifier.verify(stage / 'sdk-final-repeat', sdk_digest) == sdk
assert (stage / 'sdk-final.tar.gz').read_bytes() == (stage / 'sdk-final-repeat.tar.gz').read_bytes()
assert len(sdk['libraries']) == 22 and len(sdk['registryPackages']) == 48 and len(sdk['files']) == 434
for record in sdk['sourceFiles']:
    assert entry(record['path']) == record, record['path']
old_sdk = read(stage / 'sdk-two/sdk-manifest.json')
before = {r['path']: r for r in old_sdk['files']}
after = {r['path']: r for r in sdk['files']}
assert {p for p in before.keys() | after.keys() if before.get(p) != after.get(p)} == {'verify.py'}
root_lock = tomllib.loads(Path('Cargo.lock').read_text())
old_lock = tomllib.loads(Path('.codex-work/embedded-sdk/sdk-one/Cargo.lock').read_text())
registry = lambda lock: {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                         for p in lock['package'] if p.get('source')}
assert registry(tomllib.loads((stage / 'sdk-final/Cargo.lock').read_text())) <= registry(root_lock)
assert registry(old_lock) <= registry(root_lock)

environment = read(stage / 'environment.json')
for record in environment['productLocks']:
    assert entry(product / record['path'], product) == record
    previous = next(p for p in read('.codex-work/product-native-manifest/environment.json')['productLocks']
                    if p['path'] == record['path'])
    assert previous == record
for record in environment['kernelLocks']:
    assert entry(record['path']) == record
for record in parent['productPrivateFiles']:
    assert entry(product / record['path'], product) == record, record['path']
product_names = [
    'components/musteroffice/.gitignore', 'components/musteroffice/README.md',
    'components/musteroffice/lock.json', 'scripts/prepare-musteroffice-development.mjs',
    'scripts/prepare-musteroffice-development.test.mjs',
]
for name in product_names:
    assert len((product / name).read_text().splitlines()) <= 2000
lock = read(product / 'components/musteroffice/lock.json')
assert lock['sdkManifestSha256'] == sdk_digest
prepared = read(stage / 'product-prepare-final-1.log')
reused = read(stage / 'product-reuse-final-1.log')
assert not prepared['sdkReused'] and prepared['workerReused']
assert reused['sdkReused'] and reused['workerReused']
assert Path(prepared['sdkDirectory']).resolve().is_relative_to(product)
assert verifier.verify(Path(prepared['sdkDirectory']), sdk_digest) == sdk
worker = Path(prepared['workerFile'])
assert worker.resolve().is_relative_to(product)
assert entry(worker, product)['sha256'] == lock['workers']['darwin-arm64']['sha256']
assert worker.read_bytes() == Path('.codex-work/embedded-export/frozen/mo-export-worker').read_bytes()

for name in ['consumer-build-2.log', 'consumer-product-build-1.log', 'facade-check-1.log', 'clippy-1.log']:
    log = (stage / name).read_text()
    assert 'Finished `dev` profile' in log and 'error:' not in log, name
assert 'error[E0599]' in (stage / 'consumer-build-1.log').read_text()
assert not (stage / 'fmt-1.log').read_bytes()
assert 'Ran 6 tests' in (stage / 'verifier-tests-1.log').read_text()
assert '\nOK\n' in (stage / 'verifier-tests-1.log').read_text()
assert 'ℹ pass 13' in (stage / 'product-prepare-tests-1.log').read_text()
assert 'ℹ fail 0' in (stage / 'product-prepare-tests-1.log').read_text()
native_log = (stage / 'native-tests-1.log').read_text()
assert '8 passed; 0 failed' in native_log
test_names = re.findall(r'^test (.+) \.\.\. ok$', native_log, re.M)
assert len(test_names) == 8
for name in ['actual-run-1.log', 'consumer-product-run-1.log']:
    assert read(stage / name) == dict(assets=12, pages=2, committed=False)
files = read(stage / 'actual/files.json')
assert len(files) == 12 and read(stage / 'actual-product/files.json') == files
for record in files:
    data = (stage / 'actual' / record['file']).read_bytes()
    assert data == (stage / 'actual-product' / record['file']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == record['asset']['sha256']
    assert len(data) == int(record['asset']['byteLength'])
    # Independent earlier worker output uses MIME extensions; identify it by SHA.
    matches = [p for p in Path('.codex-work/embedded-export/actual').iterdir()
               if p.is_file() and hashlib.sha256(p.read_bytes()).hexdigest() == record['asset']['sha256']]
    assert matches, record['file']
assert (stage / 'actual/inspection.json').read_bytes() == (stage / 'actual-product/inspection.json').read_bytes()

# The verifier's own output must not recursively bind the report being written.
artifacts = {p for p in stage.iterdir() if p.is_file() and not p.name.startswith('evidence-')}
for name in ['input', 'actual', 'actual-product', 'consumer-two', 'consumer-product']:
    artifacts |= {p for p in (stage / name).rglob('*') if p.is_file()}
for name in ['sdk-two', 'sdk-final', 'sdk-final-repeat']:
    artifacts.add(stage / name / 'sdk-manifest.json')
for name in ['consumer-target-two', 'consumer-target-product']:
    artifacts.add(stage / name / 'debug/musteroffice-embedding-example')
report = dict(
    format='musteroffice.embedded-sdk-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), localLinksChecked=links,
    productHeadObserved=environment['productHead'],
    productPriorReaderHashesUnchanged=True,
    productPrivateFiles=[entry(product / p, product) for p in product_names],
    environment=entry(stage / 'environment.json'),
    sdk=dict(libraries=22, registryPackages=48, files=434, manifestSha256=sdk_digest,
             archive=entry(stage / 'sdk-final.tar.gz'), repeatedArchiveIdentical=True,
             firstSuccessfulConsumerToFinalChangedFiles=['verify.py']),
    finalProductPreparedConsumerBuilt=True, actualPages=2, actualAssets=12,
    actualAssetBytesUnchanged=True, finalFileInspectionUnchanged=True,
    tests=dict(nativeProcess=8, nativeNames=test_names, pythonVerifier=6, productPreparation=13,
               strictClippyAffectedPackages=True, completeWorkspaceRerun=False),
    artifacts=[entry(p) for p in sorted(artifacts)],
    limitations=[
        'Local SDK and material preparation only; no product workspace dependency, Runtime producer, atomic commit or UI switch.',
        'Example uses real final-file validation but has no product permissions, Content Store, Invocation or commit authority.',
        'SDK source archive is not a linked runtime or installer. Native worker and fonts/media are separate.',
        'Only macOS ARM64 worker is pinned; no new WASM build or complete cross-platform distribution acceptance.',
        'Registry versions are inherited; metadata license declarations are not complete binary distribution notices.',
        'Streaming product storage, advanced content, Viewer/Player, Office/WPS roundtrip, migration and E0-E3 remain open.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(product_names), links=links,
                     actualAssets=12, evidence=entry(output))))
