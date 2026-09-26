"""Bind the storage-independent read interface to actual regressions and size."""
import gzip
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/delivery-size')
output = Path('docs/reviews/evidence/2026-09-26-package-read-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-delivery-receive-verification.json')

def entry(path):
    path = Path(path)
    raw = path.read_bytes()
    return dict(path=str(path), byteLength=len(raw), sha256=hashlib.sha256(raw).hexdigest())

def read(path):
    return json.loads(Path(path).read_text())

def checked(record):
    assert entry(record['path']) == record
    return read(record['path'])

assert entry(parent_path)['sha256'] == 'd5b8fefe0d75bbd7efc8749cf38d8c584bfd1993f99e554df87963695d89e446'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
baseline = read(root/'package-read-baseline/manifest.json')
changed = {p for p, r in prior.items() if entry(p) != r}
assert changed == {r['path'] for r in baseline} | {'docs/implementation/progress.md'}, changed
for record in baseline:
    assert record['sha256'] == prior[record['path']]['sha256']
    assert entry(root/'package-read-baseline'/record['path'])['sha256'] == record['sha256']
added = {'crates/mo-opc/src/read.rs', 'crates/mo-opc/tests/read_view.rs',
    'docs/implementation/package-read-sharing.md'}
added |= {str(p) for p in Path('tools/verification').glob('package-read-*') if p.is_file()}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs', '.py', '.ts', '.mjs', '.cpp', '.h']:
        assert len(path.read_text().splitlines()) <= 2000, name
    if path.suffix != '.md':
        continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (path.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

checks = read(root/'checks.json') + read(root/'regression-checks.json')
assert [r['name'] for r in checks] == ['clippy', 'native', 'schemas', 'types', 'fmt', 'reception', 'independent', 'regression']
for record in checks:
    assert record['exitCode'] == 0
    record['log'] = entry(record['log'])
tests = re.findall(r'^test (.+) \.\.\. ok$', (root/'workspace-tests-1.log').read_text(), re.M)
assert len(tests) == 858, len(tests)
for name in ['workspace-tests-1', 'check-1', 'wasm-package-read-1', 'clippy-1', 'native-1',
             'schemas-1', 'types-1', 'fmt-1', 'reception-1', 'independent-1', 'regression-1', 'playback-1', 'benchmark-1']:
    contents = (root/f'{name}.log').read_text()
    assert not any(s in contents for s in ['error:', 'FAILED', 'Traceback', 'AssertionError', 'error TS']), name
assert not (root/'fmt-1.log').read_bytes()
for p in prior:
    if p.startswith(('contracts/generated/', 'packages/contracts/src/generated/')):
        assert entry(p) == prior[p]

parity_path = root/'reception/report.json'
parity = read(parity_path)
assert len(parity['cases']) == 36
old = checked(parent['actualParity'])
for before, after in zip(old['cases'], parity['cases'], strict=True):
    assert before['name'] == after['name'] and before['expected'] == after['expected']
    for key in ['input', 'bytes', 'response']:
        for record in [before[key], after[key]]:
            assert entry(record['path']) == record
        assert Path(before[key]['path']).read_bytes() == Path(after[key]['path']).read_bytes()
for key in ['native', 'wasm', 'wasmGlue']:
    assert entry(parity[key]['path']) == parity[key]
reference = read(root/'independent.json')
assert reference == checked(parent['independentChecks'])
regression = read(root/'regression.json')
assert {name: r['cases'] for name, r in regression['reports'].items()} == {'authored': 17, 'source': 33, 'editor': 22}
for record in regression['reports'].values():
    assert record['allPreviousResultsIdentical']
    checked(record['report'])
for record in regression['generatedArtifacts']:
    assert entry(record['path']) == record

playback_root = Path('.codex-work/package-read-playback')
playback = read(playback_root/'comparison.json')
assert playback['pairedCalls'] == 654 and playback['allPreviousRequestsResponsesPixelsFramesAndComponentCallsIdentical']
product = checked(playback['after'])
previous = checked(playback['before'])
for before, after in zip(previous['cases'], product['cases'], strict=True):
    for key in ['request', 'response', 'pixels', 'frame', 'source', 'fonts']:
        if key in before:
            for record in [before[key], after[key]]:
                assert entry(record['path']) == record
            assert before[key]['sha256'] == after[key]['sha256']
for record in product['artifacts']:
    assert entry(record['path']) == record

def size(path):
    raw = Path(path).read_bytes()
    return dict(module=entry(path), gzip9Bytes=len(gzip.compress(raw, compresslevel=9, mtime=0)))

before = size(old['wasm']['path'])
after = size(parity['wasm']['path'])
assert (after['module']['byteLength'], after['gzip9Bytes']) == (9843551, 2693291)
raw_delta = after['module']['byteLength'] - before['module']['byteLength']
gzip_delta = after['gzip9Bytes'] - before['gzip9Bytes']
assert (raw_delta, gzip_delta) == (-205996, -28902)
benchmark = read(root/'benchmark.json')
assert len(benchmark['cases']) == 4
assert all(c['rounds'] == 41 and c['batch'] == 4 and len(c['beforeMs']) == len(c['afterMs']) == 41 for c in benchmark['cases'])
for record in benchmark['artifacts']:
    assert entry(record['path']) == record
for case in benchmark['cases']:
    for record in case['inputs']:
        assert entry(record['path']) == record

# The profiler is an isolated diagnostic experiment, not a workspace dependency.
# Keep its source/lock/binary, but do not enumerate its dependency build cache.
artifacts = [entry(p) for p in sorted(root.rglob('*')) if p.is_file() and
             ('profiler/target/' not in str(p) or str(p).endswith('/debug/delivery-wasm-profiler'))]
artifacts += [entry(p) for p in sorted(playback_root.rglob('*')) if p.is_file()]
report = dict(format='musteroffice.package-read-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), localLinksChecked=links, checks=checks, rustTestNames=tests,
    workspaceTests=858, schemaCount=111, schemasAndTypesUnchanged=True,
    reception=entry(parity_path), independentChecks=entry(root/'independent.json'),
    publicApiRegression=entry(root/'regression.json'), playback=entry(playback_root/'comparison.json'),
    performance=entry(root/'benchmark.json'), wasmModuleSize=dict(before=before, after=after,
        rawDelta=raw_delta, gzip9Delta=gzip_delta,
        scope='Identical release configuration and wasm-bindgen; Rust module only, including its existing name section. Not full components, fonts, media, JS or an installer.'),
    artifacts=artifacts, externalGeneratedArtifacts=regression['generatedArtifacts'],
    limitations=[
        'Sealed borrowed read interface; package opening, reader ownership and streaming rewrite remain generic and retain existing validation.',
        'No JSON schema, typed deserialization, content, quality or editability checks removed; production dependency lock unchanged.',
        'WASM cost samples cover four owned small inputs only. They do not establish general throughput, cold start, RSS, FPS or native storage performance.',
        'Diagnostic function-name groups omit generic type arguments and are attribution clues, not exact per-type accounting.',
        'No complete advanced-content, Office/WPS, production Viewer/Player, Musterwork integration or E0-E3 acceptance.',
    ])
serialized = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == serialized
else:
    with output.open('x') as f:
        f.write(serialized)
print(json.dumps(dict(sources=len(sources), links=links, rustTests=858, pairedCalls=36+72+654,
    rawDelta=raw_delta, gzip9Delta=gzip_delta, evidence=entry(output))))
