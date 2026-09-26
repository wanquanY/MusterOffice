"""Seal product streaming storage evidence; private source stays in the product."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import unquote

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
product = args.product_repo.resolve()
stage = Path('.codex-work/product-content-stream')
output = Path('docs/reviews/evidence/2026-09-26-product-content-stream-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-embedded-sdk-verification.json')


def read(path):
    return json.loads(Path(path).read_text())


def entry(path, base=Path('.')):
    path = Path(path)
    length = 0
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while data := stream.read(1024 * 1024):
            length += len(data)
            digest.update(data)
    return dict(path=path.relative_to(base).as_posix(), byteLength=length, sha256=digest.hexdigest())


assert entry(parent_path)['sha256'] == '744519236c506ffec64c2aa756af4716d2fe7b95091af0ab32fff07021272dcf'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {p for p, record in prior.items() if entry(p) != record}
assert changed == {'docs/README.md', 'docs/implementation/progress.md', 'docs/implementation/dependencies.md'}, changed
added = {'docs/implementation/product-content-stream.md',
         'tools/verification/product-content-stream-files.py',
         'tools/verification/product-content-stream-evidence.py'}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs', '.py', '.mjs', '.ts', '.cpp', '.h']:
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
baseline = read(stage / 'product-baseline.json')
for record in baseline['files']:
    data = subprocess.check_output(['git', 'show', f"{baseline['head']}:{record['path']}"], cwd=product)
    assert len(data) == record['byteLength'] and hashlib.sha256(data).hexdigest() == record['sha256']
private_names = {r['path'] for r in baseline['files']}
private_names |= {
    'apps/agent-runtime/crates/runtime/content/src/stream.rs',
    'apps/agent-runtime/crates/runtime/content/src/stream/tests.rs',
    'apps/agent-runtime/crates/infrastructure/sqlite/src/content/stream.rs',
    'apps/agent-runtime/crates/infrastructure/sqlite/src/content/stream_file.rs',
    'apps/agent-runtime/crates/infrastructure/sqlite/src/content/stream/tests.rs',
    'apps/agent-runtime/crates/infrastructure/sqlite/src/content/stream/tests/cases.rs',
}
for name in private_names:
    if Path(name).suffix == '.rs':
        assert len((product / name).read_text().splitlines()) <= 2000, name
for report in [parent, read('docs/reviews/evidence/2026-09-26-product-office-manifest-verification.json')]:
    for record in report['productPrivateFiles']:
        assert entry(product / record['path'], product) == record, record['path']
environment = read(stage / 'environment.json')
for record in environment['productLocks']:
    assert entry(product / record['path'], product) == record
for record in environment['kernelLocks']:
    assert entry(record['path']) == record
old_lock = tomllib.loads(subprocess.check_output(
    ['git', 'show', baseline['head'] + ':apps/agent-runtime/Cargo.lock'], cwd=product, text=True))
new_lock = tomllib.loads((product / 'apps/agent-runtime/Cargo.lock').read_text())
registry = lambda lock: {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                         for p in lock['package'] if p.get('source')}
assert registry(old_lock) == registry(new_lock)
old_internal = {p['name']: p for p in old_lock['package'] if not p.get('source')}
new_internal = {p['name']: p for p in new_lock['package'] if not p.get('source')}
assert {k for k in old_internal if old_internal[k] != new_internal[k]} == {'musterwork-agent-runtime-content'}

test_names = {}
executables = []
for name, count in [('runtime-tests-3', 20), ('sqlite-tests-3', 25)]:
    log = (stage / f'{name}.log').read_text()
    passed = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
    assert len(passed) == count and 'FAILED' not in log
    assert f'{count} passed; 0 failed' in log
    test_names[name] = passed
    matches = re.findall(r'Running unittests src/lib.rs \((target/debug/deps/[^)]+)\)', log)
    assert len(matches) == 1
    executables.append(entry(product / 'apps/agent-runtime' / matches[0], product))
assert len([n for n in test_names['sqlite-tests-3'] if n.startswith('content::')]) == 18
clippy = (stage / 'clippy-1.log').read_text()
assert 'Finished `dev` profile' in clippy and 'error:' not in clippy and 'warning:' not in clippy
assert not (stage / 'fmt-final.log').read_bytes()
assert (stage / 'check-1.log').read_text().count('error[E0631]') == 4
file_report = read(stage / 'file-verification.json')
assert file_report['actualFiles'] == 13 and file_report['assets'] == 12 and file_report['pages'] == 2
for key in ['allBytesUnchanged', 'finalStoredInspectionUnchanged', 'corruptFinalBytesRejected', 'wrongExternalRendererRejected']:
    assert file_report[key]
assert entry(file_report['cli']['path']) == file_report['cli']
for record in file_report['files']:
    assert entry(record['path']) == record
    assert Path(record['path']).read_bytes() == (stage / 'actual-final' / Path(record['path']).name).read_bytes()
assert read(stage / 'actual/files.json') == read(stage / 'actual-final/files.json')
assert read(stage / 'inspected.json')['report'] == read('.codex-work/embedded-export/actual/inspection.json')
assert read(stage / 'rejected-corrupt.json')['status'] == 'error'
assert read(stage / 'rejected-renderer.json')['status'] == 'error'

report = dict(
    format='musteroffice.product-content-stream-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), localLinksChecked=links, kernelComputingSourcesUnchanged=True,
    productBaseline=entry(stage / 'product-baseline.json'),
    productHeadObserved=environment['productHeadObserved'],
    productPrivateFiles=[entry(product / p, product) for p in sorted(private_names)],
    priorProductSdkAndReaderHashesUnchanged=True,
    productTestExecutables=executables, environment=entry(stage / 'environment.json'),
    externalRegistryVersionsUnchanged=True,
    dependencies=dict(existingSha2Direct='0.11.0', existingTempfilePromotedToProduction='3.27.0'),
    tests=dict(runtimeContent=20, sqliteSelected=25, sqliteContent=18, names=test_names,
               strictProductionClippy=True, sqliteWholeSuite=False),
    transferChunkBytes=65536, generatedLargeInputBytes=8 * 1024 * 1024 + 17,
    actualFiles=13, actualAssets=12, actualPages=2, sharedInspectionUnchanged=True,
    sharedInspectionNegatives=2,
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'Shared ports and real SQLite filesystem only. PostgreSQL/object storage streaming ports are not implemented.',
        'Stream sinks are private staging until complete success; no partial bytes can be treated as verified.',
        'RAII cancellation is not hard process-exit recovery. Orphan temporary/CAS collection and job-wide disk quotas remain open.',
        'Native producer, immutable range-read integration, fenced Artifact commit and Viewer/Player are still unconnected.',
        '64 KiB transfer blocks are not a whole-process RSS, throughput, package-size or cross-platform acceptance result.',
        'Full advanced content, Office/WPS editing, history migration, distribution and E0-E3 remain open.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(private_names), links=links,
                     tests=45, actualFiles=13, evidence=entry(output))))
