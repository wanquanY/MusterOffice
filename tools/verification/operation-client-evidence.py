"""Bind the thin TS client to its native integration and independent checks."""
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlparse

root = Path('.codex-work/operation-client')
output = Path('docs/reviews/evidence/2026-09-26-operation-client-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-mcp-recovery-verification.json')


def entry(path):
    path = Path(path)
    raw = path.read_bytes()
    return dict(path=str(path), byteLength=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path):
    return json.loads(Path(path).read_text())


def local(value):
    parsed = urlparse(value)
    path = Path(unquote(parsed.path)) if parsed.scheme == 'file' else Path(value)
    return path.resolve().relative_to(Path.cwd())


assert entry(parent_path)['sha256'] == '1070c4f6d6b7368ae54e0ee0f3c28384798b7bf986c2e684543d073311b8de02'
parent = read(parent_path)
prior = {e['path']: e for e in parent['sourceFiles']}
changed = {name for name, item in prior.items() if entry(name) != item}
assert changed == {'package.json', 'docs/implementation/progress.md',
    'docs/architecture/agent-integration.md', 'docs/design/agent-interfaces.md'}, changed
sources = set(prior) | {'docs/implementation/operation-client.md',
    'tools/verification/operation-client-evidence.py',
    'tools/verification/operation-client-reference.py',
    'tools/verification/operation-client-native.mjs',
    'tools/verification/operation-client-tests.mjs'}
for directory in ['packages/operation-client', 'tools/verification/operation-client']:
    sources |= {str(p) for p in Path(directory).rglob('*') if p.is_file()}
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
            destination = (path.parent / target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

report = read(root/'native-2/report.json')
reference = read(root/'reference-2.json')
assert report['node'] == 'v23.5.0' and report['platform'] == 'darwin' and report['arch'] == 'arm64'
assert report['controlCalls'] == reference['controlCallsSchemaChecked'] == 37
assert report['binaryCalls'] == 22
assert report['assets'] == reference['assetsVerified'] == 12
assert report['resumedAppendOffsets'] == ['0', '262144', '524288', '786432']
assert all(report[k] for k in ['recoveredAcknowledgement', 'retriesAndReconnectUnchanged',
    'crossScopeRejectedBeforePrivateStore'])
assert reference['persistedJobs'] == 3 and reference['persistedRevisions'] == 2
assert reference['largeUploadBytes'] == 786469
assert len(reference['checkedParts']) == 10 and reference['nativeObjects'] == 15 and reference['textRuns'] == 8
assert len(reference['pages']) == 2 and reference['samePreviewPixelsAsMcp']
assert reference['pages'] == read('.codex-work/mcp-recovery/file-checks-final.json')['files'][0]['pages']
assert reference['pptxSha256'] == '49213f1e4cf95b4c60010f210e5e27445edf1e7ecc431573faa0d88cb5f6d7f8'
runtime = {}
for key, expected in [('binary', '219c22620e71b7dedf0e9ce515b0d9aee36c8b0a4c8dd5fe0139e71b1afbf2b0'),
    ('worker', 'b5de830a2733cfdc6747534f63ba22cf2170240645db6bfb437fc32e946e1be4')]:
    actual = entry(local(report[key]['path']))
    assert actual['sha256'] == report[key]['sha256'] == expected
    assert actual['byteLength'] == report[key]['byteLength']
    runtime[key] = actual
for asset in read(root/'native-2/assets.json'):
    actual = entry(local(asset['candidate']['path']))
    assert actual['sha256'] == asset['candidate']['sha256'] == asset['asset']['sha256']
    assert actual['byteLength'] == asset['candidate']['byteLength'] == int(asset['asset']['byteLength'])

test_log = (root/'tests-3.log').read_text()
tests = re.findall(r'^✔ (.+) \([0-9.]+ms\)$', test_log, re.M)
assert len(tests) == len(set(tests)) == 17
assert 'ℹ pass 17' in test_log and 'ℹ fail 0' in test_log
checks = []
for name in ['tests-3', 'types', 'native-2', 'reference-2']:
    path = root/f'{name}.log'
    text = path.read_text()
    assert not any(bad in text for bad in ['error TS', 'ERR_PNPM', 'Traceback', 'AssertionError', '✖']), name
    checks.append(dict(name=name, exitCode=0, log=entry(path)))
for name in ['initial', 'reconnected', 'denied']:
    assert not (root/f'native-2/{name}.stderr').read_bytes()
build = root/'build/operation-client/src'
js = [entry(p) for p in sorted(build.glob('*.js'))]
declarations = [entry(p) for p in sorted(build.glob('*.d.ts'))]
assert len(js) == len(declarations) == 7
assert sum(p['byteLength'] for p in js) == 16371
assert sum(p['byteLength'] for p in declarations) == 8108
for item in js:
    text = Path(item['path']).read_text()
    for module in re.findall(r'\bfrom [\'\"]([^\'\"]+)[\'\"]', text):
        assert module.startswith('./') and (build/module).exists(), module
    assert not re.search(r'\b(?:require|import)\s*\(', text)

evidence = dict(format='musteroffice.operation-client-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(sources-set(prior)), sourceLinksChecked=links,
    checks=checks, testNames=tests, runtime=runtime,
    native=entry(root/'native-2/report.json'), independentChecks=entry(root/'reference-2.json'),
    controlCalls=37, binaryCalls=22, assets=12,
    emittedJavaScript=js, emittedDeclarations=declarations,
    unminifiedJavaScriptBytes=16371, ownDeclarationBytes=8108,
    artifacts=[entry(p) for p in sorted(root.rglob('*')) if p.is_file()],
    limitations=[
        'This is a host-injected control/resource SDK layer, not a shipping transport or full SDK.',
        'OperationPort must validate wire schemas and bind trusted host authority; TS types do not validate wire values.',
        'Native integration is a Standalone test driver, not a Musterwork Embedded adapter or second product owner.',
        'No new Rust, contract, dependency-lock, full Native/WASM, Office/WPS or product acceptance.',
        'Final stored byte integrity does not imply layout, native editing, playback or target application acceptance.',
        'Huge integer offsets use a mock source; no huge-file throughput, memory, FPS or installer-size claim.',
        'Private callbacks must settle before cleanup; host implementations own bounded/cooperative I/O.',
        'Browser owner, production transports, Viewer/Player, Skill/Plugin, full advanced content and Musterwork replacement remain incomplete.'])
encoded = json.dumps(evidence, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == encoded
elif '--dry-run' not in sys.argv:
    assert not output.exists(), 'never overwrite historical evidence'
    output.write_text(encoded)
print(json.dumps(dict(sourceFiles=len(sources), sourceLinks=links, tests=17, controlCalls=37,
    binaryCalls=22, assets=12, unminifiedJavaScriptBytes=16371,
    output=str(output), check='--check' in sys.argv)))
