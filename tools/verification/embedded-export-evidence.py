"""Seal the stateless native export stage after actual verification."""
import gzip
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/embedded-export')
output = Path('docs/reviews/evidence/2026-09-26-embedded-export-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-package-read-verification.json')

def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

def read(path):
    return json.loads(Path(path).read_text())

def checked(record):
    assert entry(record['path']) == record, record['path']
    return read(record['path'])

assert entry(parent_path)['sha256'] == '8c5d666836a1d7c1250a70392e515d8131e2ea22a0ad3a8ecfed0e8ff65e966e'
parent = read(parent_path)
prior = {r['path']:r for r in parent['sourceFiles']}
changed = {p for p,r in prior.items() if entry(p) != r}
assert changed == {
    'Cargo.toml', 'Cargo.lock', 'crates/mo-native-io/src/lib.rs',
    'crates/mo-native-io/tests/spool.rs',
    'crates/mo-kernel-api/src/text.rs',
    'crates/mo-kernel-api/src/pptx_resource_page/diagnostic.rs',
    'docs/implementation/progress.md', 'docs/implementation/dependencies.md',
}, changed
added = {str(p) for directory in ['crates/mo-native-export', 'tools/mo-export-worker']
    for p in Path(directory).rglob('*') if p.is_file()}
added |= {'crates/mo-native-io/src/directory.rs', 'docs/implementation/embedded-export.md'}
added |= {str(p) for p in Path('tools/verification').glob('embedded-export-*.py')}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:
        assert len(path.read_text().splitlines()) <= 2000, name
    if path.suffix != '.md':
        continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):
            continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            resolved = (path.parent/target).resolve()
            assert resolved.exists() or resolved == output.resolve(), (name,target)
            links += 1
for name, record in prior.items():
    if name.startswith(('contracts/generated/', 'packages/contracts/src/generated/')):
        assert entry(name) == record

tests_log = (root/'workspace-tests-1.log').read_text()
tests = re.findall(r'^test (.+) \.\.\. ok$', tests_log, re.M)
assert len(tests) == 873, len(tests)
assert 'Doc-tests mo_xml' in tests_log and 'FAILED' not in tests_log
for name in ['clippy-1','types-1','schemas-1','fmt-1','release-build-1','wasm-build-1']:
    text = (root/f'{name}.log').read_text()
    assert not any(marker in text for marker in ['error:', 'FAILED', 'Traceback', 'error TS']), name
assert not (root/'fmt-1.log').read_bytes()
native = checked(entry(root/'native.json'))
assert native['pptxAndPagePixelsUnchanged']
for record in native['checks']:
    assert record['exitCode'] == 0
    assert entry(record['log']['path']) == record['log']
for record in native['inputFiles']:
    assert entry(record['path']) == record
reference = checked(native['independentReference'])
assert len(reference['schemasChecked']) == 10 and reference['nativeObjects'] == 15
assert reference['nativeTextRuns'] == 8 and len(reference['pages']) == 2
assert len(reference['artifacts']) == 12
for record in reference['artifacts']:
    assert entry(record['path']) == record
assert len(re.findall(r'^test .+ \.\.\. ok$', (root/'release-integration.log').read_text(), re.M)) == 8

parent_deps = (root/'parent-dependencies.txt').read_text()
worker_deps = (root/'worker-dependencies.txt').read_text()
for forbidden in ['mo-standard-host','rusqlite','libsqlite3-sys','tokio']:
    assert forbidden not in parent_deps and forbidden not in worker_deps
for component in ['mo-skia-sys','mo-harfbuzz-sys']:
    assert component not in parent_deps and component in worker_deps
environment = read(root/'environment.json')
assert environment['rustLock'] == entry('Cargo.lock')
for record in environment['components']:
    assert entry(record['path']) == record
wasm = read(root/'wasm-analysis.json')
assert not wasm['moduleUnchanged'] and wasm['glueUnchanged']
assert wasm['unchangedCodeImportsExportsAndAllOtherSections'] and wasm['changedBytes'] == 2
assert len(wasm['panicSourceLocations']) == 2
for record in [wasm['module'], wasm['glue']]:
    assert entry(record['path']) == record
for record in wasm['checks']:
    assert record['exitCode'] == 0 and entry(record['log']['path']) == record['log']
reception = checked(wasm['reception'])
assert len(reception['cases']) == 36
regression = checked(wasm['regression'])
assert {k:v['cases'] for k,v in regression['reports'].items()} == {'authored':17,'source':33,'editor':22}
for record in regression['reports'].values():
    assert record['allPreviousResultsIdentical']
    checked(record['report'])
for record in regression['generatedArtifacts']:
    assert entry(record['path']) == record
worker_bytes = Path(native['worker']['path']).read_bytes()
assert entry(native['worker']['path']) == native['worker']
assert len(worker_bytes) == 10027936
assert len(gzip.compress(worker_bytes,compresslevel=9,mtime=0)) == 4233513
linked_libraries = (root/'native-linked-libraries.txt').read_text().splitlines()[1:]
assert len(linked_libraries) == 2 and all(line.strip().startswith('/usr/lib/') for line in linked_libraries)
observation = read(root/'musterwork-observation.json')
assert len(observation['files']) == 5
artifacts = [entry(p) for p in sorted(root.rglob('*')) if p.is_file()]
report = dict(format='musteroffice.embedded-export-verification/1',
    parent=entry(parent_path), sourceFiles=[entry(p) for p in sorted(sources)],
    changedPriorSources=sorted(changed), addedSources=sorted(added),
    localLinksChecked=links, workspaceTests=len(tests), rustTestNames=tests,
    schemaCount=111, schemasAndTypesUnchanged=True,
    native=entry(root/'native.json'), environment=entry(root/'environment.json'),
    wasm=entry(root/'wasm-analysis.json'), musterworkObservation=entry(root/'musterwork-observation.json'),
    worker=native['worker'], workerGzip9Bytes=4233513, artifacts=artifacts,
    externalGeneratedArtifacts=regression['generatedArtifacts'],
    limitations=[
        'Stateless computation inside an existing host task; no database, queue or product publication authority.',
        'Real native export, retained-file verification and process faults; no actual Musterwork Invocation/ContentStore/Artifact integration.',
        'Owned synthetic two-page fixture, ten XSD parts, fifteen native objects and eight text runs; not full visual, Office/WPS editing or advanced-content acceptance.',
        'Parent adapter callbacks must be bounded; the host must reserve aggregate disk/RSS and implement parent-crash orphan recovery.',
        'Unix fault processes and macOS arm64 native components tested here; other native platforms remain unaccepted.',
        'WASM code and all sections except two panic source line numbers are byte-identical; JS/contracts unchanged and 108 public paired calls passed. No new complete browser export owner or Viewer/Player is implemented.',
        'Single native executable size is not an installer, end-to-end performance or peak memory budget.',
        'Complete advanced content, Skill/Plugin/HTTP, historical migration and E0-E3 replacement acceptance remain open.',
    ])
serialized = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(sources=len(sources), links=links, tests=len(tests), evidence=entry(output))))
