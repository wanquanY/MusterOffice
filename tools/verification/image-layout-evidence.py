"""Seal local image layout with real native source/decoder and scoped regression evidence."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/image-layout')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-domain-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-image-layout-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == 'cd13e3f4cc9322ae4d6e5de1d6fa9c6868779b25b4ab8701073bfed0aeb30b51'
old = json.loads(PREVIOUS.read_text())
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
allowed = {
    'Cargo.lock', 'crates/mo-presentation-compile/Cargo.toml', 'crates/mo-skia-sys/Cargo.toml',
    'crates/mo-presentation-compile/src/lib.rs', 'crates/mo-presentation-compile/src/source_number.rs',
    'README.md', 'docs/README.md', 'docs/implementation/progress.md', 'docs/implementation/development.md',
}
assert changed == allowed, (changed-allowed, allowed-changed)
added = {
    'crates/mo-presentation-compile/src/source_image_layout.rs',
    'crates/mo-presentation-compile/src/source_image_layout/number.rs',
    'crates/mo-presentation-compile/src/source_image_layout/source.rs',
    'crates/mo-presentation-compile/src/source_image_layout/types.rs',
    'crates/mo-presentation-compile/src/source_image_layout/tests.rs',
    'crates/mo-skia-sys/examples/source_image_layout.rs',
    'tools/verification/image-layout-reference.py', 'tools/verification/image-layout-evidence.py',
    'docs/implementation/image-layout.md',
}
sources = {r['path'] for r in old['sourceFiles']} | added
for name in sources:
    p = Path(name)
    if p.suffix in ['.rs', '.cpp', '.h', '.ts', '.py', '.mjs']:
        assert len(p.read_text().splitlines()) <= 2000, name

# Reversing ONLY the five new local dependency edges must recover the exact
# previous lock. No unrelated resolution/version/dependency change is allowed.
lock = Path('Cargo.lock').read_text()
for name, dependencies in [
    ('mo-presentation-compile', ['mo-image']),
    ('mo-skia-sys', ['mo-common', 'mo-opc', 'mo-pptx', 'mo-presentation-compile']),
]:
    pattern = r'\[\[package\]\]\nname = "'+name+r'"\n.*?(?=\n\[\[package\]\]|\Z)'
    match = re.search(pattern, lock, re.S)
    block = match.group()
    for dependency in dependencies:
        line = ' "'+dependency+'",\n'
        assert block.count(line) == 1
        block = block.replace(line, '')
    lock = lock[:match.start()]+block+lock[match.end():]
old_lock = next(r for r in old['sourceFiles'] if r['path'] == 'Cargo.lock')
assert len(lock.encode()) == old_lock['byteLength']
assert hashlib.sha256(lock.encode()).hexdigest() == old_lock['sha256']

logs = ['tests', 'clippy', 'fmt', 'native-build', 'wasm-build', 'schema-check', 'types-check',
        'contracts', 'example-build', 'reference', 'regressions', 'recent-regressions']
for name in logs:
    value = (ROOT/(name+'.log')).read_text()
    assert not any(v in value for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
    if name in ['clippy', 'native-build', 'wasm-build', 'example-build']:
        assert 'Finished' in value, name
test_log = (ROOT/'tests.log').read_text()
assert 'Doc-tests mo_xml' in test_log and test_log.rstrip().endswith('finished in 0.00s')
tests = re.findall(r'^test (.+) \.\.\. ok$', test_log, re.M)
new_tests = [n for n in tests if n not in old['rustTestNames']]
assert (len(tests), len(new_tests)) == (545, 7)
assert len(re.findall(r'^check contracts/generated/', (ROOT/'schema-check.log').read_text(), re.M)) == 78
assert len(re.findall(r'^check .+\.ts$', (ROOT/'types-check.log').read_text(), re.M)) == 78
contracts = json.loads((ROOT/'contracts.log').read_text())
assert (contracts['schemas'], contracts['negativeMutations']) == (78, 9)
reference = json.loads((ROOT/'reference.json').read_text())
summary = {k: v for k, v in reference.items() if k not in ['format', 'example', 'cases']}
assert json.loads((ROOT/'reference.log').read_text()) == summary
for key, value in dict(nativeRequests=102, successfulRequests=96, sourceTargetLayouts=288,
                       independentBoundedValues=3456, officialXsdParts=204, resourceReuseReferences=306).items():
    assert reference[key] == value
assert len(reference['bindingChecks']['rejected']) == 9
assert reference['bindingChecks']['cancelledCheckpoints'] == 69
reports = {'reference': reference}
markers = {
    'regressions': dict(requests=307, pixelOutputs=21),
    'recent-regressions': dict(requests=211, priorStableResponses=204, textPages=124,
                               textGeometry=41, imageRequests=39, jsonRejections=7),
}
for name, marker in markers.items():
    assert json.loads((ROOT/(name+'.log')).read_text()) == marker
    reports[name] = json.loads((ROOT/(name+'.json')).read_text())
records = 0


def audit(value):
    global records
    if isinstance(value, dict):
        if {'path', 'byteLength', 'sha256'} <= value.keys():
            actual = entry(value['path'])
            assert all(actual[k] == value[k] for k in actual), value['path']
            records += 1
        for v in value.values():
            audit(v)
    elif isinstance(value, list):
        for v in value:
            audit(v)


audit(reports)
for key in ['unchangedComponents', 'previousReleaseArtifactsVerifiedUnchanged', 'standardInputs', 'componentBuilds', 'codecInputLock']:
    audit(old[key])
for build in old['componentBuilds'].values():
    value = json.loads(Path(build['path']).read_text())
    for key in ['componentSources', 'artifacts', 'imageCodecs']:
        audit(value[key])
schemas = {name: Draft202012Validator(json.loads(Path('contracts/generated/'+name+'.schema.json').read_text()))
           for name in ['pptx-image-query', 'pptx-image-response']}
for c in reference['cases']:
    request = json.loads(Path(c['request']['path']).read_text())
    output = json.loads(Path(c['output']['path']).read_text())
    schemas['pptx-image-query'].validate(request)
    schemas['pptx-image-response'].validate(dict(status='inspected', images=output['catalog']))
    assert output['catalog']['sourceSha256'] == request['fill']['expectedSourceSha256'] == c['source']['sha256']
    assert output['result']['status'] == ('laidOut' if c['success'] else 'error')
    if c['success']:
        assert len(output['result']['plans']) == 3
        assert [p['target'] for p in output['result']['plans']] == request['fill']['targets']
    else:
        assert 'plans' not in output['result']

markdown = links = 0
for name in sources:
    p = Path(name)
    if p.suffix != '.md':
        continue
    markdown += 1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        destination = unquote(target.split('#')[0].split('?')[0])
        if destination:
            resolved = p.parent/destination
            assert resolved.exists() or resolved.resolve() == OUTPUT.resolve(), (name, target)
            links += 1
subprocess.run(['git', 'diff', '--check'], check=True)
report = dict(
    format='musteroffice.image-layout-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Native local image layout and source/decoded-resource binding. Real PPTX, fixed PNG decoder and independent rational bounds. Not image page rendering, new WASM operation or Office/WPS acceptance.',
    checks=dict(rustTests=545, newRustTests=7, strictClippy=True, rustfmt=True,
                schemasChecked=78, generatedTypeScriptAndTypeCheck=True, genericNegativeMutations=9,
                nativeSourceLayoutRequests=102, successfulLayoutRequests=96, layoutTargets=288,
                independentBoundedValues=3456, officialXsdParts=204, sourceBindingFailures=9,
                sourceCancellationCheckpoints=69, currentRegressionPairedCalls=518,
                newLayoutWasmExecution=False, newLayoutPixelRendering=False,
                unchangedThirdPartyDependencyResolution=True, sourceMarkdown=markdown, localLinks=links),
    sourceFiles=[entry(n) for n in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=tests, newTestNames=new_tests,
    validationLogs={n: entry(ROOT/(n+'.log')) for n in logs},
    reports={n: entry(ROOT/(n+'.json')) for n in reports},
    componentBuilds=old['componentBuilds'], codecInputLock=old['codecInputLock'],
    unchangedComponents=old['unchangedComponents']+[old['artifacts'][n] for n in ['cppWasm', 'cppWasmGlue', 'typescriptAdapter']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'], standardInputs=old['standardInputs'],
    artifacts={n: entry(p) for n, p in {
        'nativeDebugWorker': 'target/debug/mo-raster-worker', 'nativeDebugCli': 'target/debug/mo-cli',
        'nativeDebugTextWorker': 'target/debug/mo-text-worker',
        'nativeLayoutExample': 'target/debug/examples/source_image_layout',
        'rustDebugWasm': str(ROOT/'wasm-node/mo_wasm_bg.wasm'), 'rustWasmGlue': str(ROOT/'wasm-node/mo_wasm.js'),
    }.items()},
    limitations=['Local normalized-image-axis semantics remain draft; no complete image page compilation.',
                 'Rotation binding, hard fill clipping and ordered image/text page composition still need integration.',
                 'Negative/zero tile scales and unresolved physical DPI are explicit failures, pending full policy.',
                 'New layout ran in Native only; existing 518 requests ran in actual current Native and WASM.',
                 'No new C++ change, sanitizer run, product latency/RSS, installer or Office/WPS acceptance claim.',
                 'Full advanced presentation, Agent packages and Musterwork replacement gates remain open.'],
)
if '--seal' in sys.argv:
    assert not OUTPUT.exists(), 'evidence is immutable after sealing'
    OUTPUT.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
else:
    assert json.loads(OUTPUT.read_text()) == report, 'completed evidence differs from current closure'
print(json.dumps(dict(evidence=entry(OUTPUT), sources=len(sources), auditedRecords=records)))
