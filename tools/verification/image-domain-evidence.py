"""Bind completed source-domain implementation to real execution artifacts."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/image-domain')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-resolution-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-image-domain-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == '0ef13483fdfebc3ece6ae0a7a53390d8a52c5cf18e72fbdd69e59e0800c873be'
old = json.loads(PREVIOUS.read_text())
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
allowed = {
    'README.md', 'docs/README.md', 'contracts/README.md',
    'docs/implementation/development.md', 'docs/implementation/progress.md',
    'docs/implementation/image-raster.md',
    'components/skia/mo_image.cpp', 'components/skia/mo_image.h',
    'components/skia/mo_skia.cpp', 'components/skia/mo_skia.h',
    'crates/mo-raster/src/compile.rs', 'crates/mo-raster/src/image.rs',
    'crates/mo-raster/src/image/compile.rs', 'crates/mo-raster/src/image_tests.rs',
    'crates/mo-raster/src/lib.rs', 'crates/mo-render/src/image_tests.rs',
    'crates/mo-skia-sys/src/ffi.rs', 'packages/raster-component/src/index.ts',
    'tools/components/build-skia.py', 'tools/verification/skia-probe.cpp',
}
for name in ['image-raster-request', 'image-raster-response', 'image-scene-request',
             'image-scene-response', 'page-compile-response', 'path-raster-request',
             'pptx-page-compile-response', 'scene-raster-request']:
    allowed.add('contracts/generated/'+name+'.schema.json')
    allowed.add('packages/contracts/src/generated/'+name+'.ts')
assert changed == allowed, (changed-allowed, allowed-changed)
added = {
    'components/skia/mo_image_domain.cpp', 'components/skia/mo_image_domain_stage.h',
    'components/skia/image-domain.patch', 'docs/implementation/image-domain.md',
    'tools/verification/image-domain-fixtures.py', 'tools/verification/image-domain-parity.mjs',
    'tools/verification/image-domain-components.mjs', 'tools/verification/image-domain-bench.cpp',
    'tools/verification/image-domain-benchmark.mjs', 'tools/verification/image-domain-evidence.py',
}
sources = {r['path'] for r in old['sourceFiles']} | added
for name in sources:
    p = Path(name)
    if p.suffix in ['.rs', '.cpp', '.h', '.ts', '.py', '.mjs']:
        assert len(p.read_text().splitlines()) <= 2000, name

logs = ['tests', 'clippy', 'fmt', 'native-rust-build', 'wasm-rust-build',
        'schema-check', 'types-check', 'contracts', 'fixtures', 'parity', 'components',
        'regressions', 'recent-regressions', 'scene-regressions', 'source-resources',
        'codec-regressions', 'resolution-regressions', 'component-native',
        'component-wasm', 'component-asan', 'benchmark']
for name in logs:
    value = (ROOT/(name+'.log')).read_text()
    assert not any(v in value for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
    if name in ['clippy', 'native-rust-build', 'wasm-rust-build']:
        assert 'Finished' in value, name
    if name.startswith('component-'):
        marker = json.loads(value.splitlines()[-1])
        assert marker['target'] == ('wasm' if name == 'component-wasm' else 'native')
test_log = (ROOT/'tests.log').read_text()
assert 'Doc-tests mo_xml' in test_log and test_log.rstrip().endswith('finished in 0.00s')
tests = re.findall(r'^test (.+) \.\.\. ok$', test_log, re.M)
new_tests = [n for n in tests if n not in old['rustTestNames']]
assert (len(tests), len(new_tests)) == (538, 4)
assert len(re.findall(r'^check contracts/generated/', (ROOT/'schema-check.log').read_text(), re.M)) == 78
assert len(re.findall(r'^check .+\.ts$', (ROOT/'types-check.log').read_text(), re.M)) == 78
contracts = json.loads((ROOT/'contracts.log').read_text())
assert (contracts['schemas'], contracts['negativeMutations']) == (78, 9)
assert json.loads((ROOT/'fixtures.log').read_text()) == dict(cases=313, success=304, referencePixels=145920)

markers = {
    'parity': dict(pairedCalls=626, success=304, preflightFailures=9, maxChannelDeviation=1),
    'components': dict(successfulFrames=304, negativeFrames=15, triples=321, noDiagnostics=True),
    'regressions': dict(requests=307, pixelOutputs=21),
    'recent-regressions': dict(requests=211, priorStableResponses=204, textPages=124,
                               textGeometry=41, imageRequests=39, jsonRejections=7),
    'scene-regressions': dict(sceneRequests=140, success=124, preflightFailures=16,
                              jsonRejections=7, pairedCalls=148, worldReferenceCalls=1, wasmHostRejections=1),
    'source-resources': dict(catalogs=19, invalidRequests=6, pairedCalls=50, noOverwriteChecks=19),
    'codec-regressions': dict(pairedCalls=195, decodeCalls=154, sceneCalls=41, badHostReplies=4),
    'resolution-regressions': dict(pairedCalls=184, decodeCalls=106, sceneCalls=78, badHostReplies=4),
}
reports = {}
report_entries = {}
for name, marker in markers.items():
    assert json.loads((ROOT/(name+'.log')).read_text()) == marker, name
    p = ROOT/(name+'/parity.json' if name in ['codec-regressions', 'resolution-regressions'] else name+'.json')
    report_entries[name] = entry(p)
    reports[name] = json.loads(p.read_text())
for name in ['codec-regressions', 'resolution-regressions']:
    assert reports[name]['priorFieldsAndPixelsUnchanged']
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
for key in ['unchangedComponents', 'previousReleaseArtifactsVerifiedUnchanged',
            'standardInputs', 'componentBuilds', 'codecInputLock']:
    audit(old[key])
component_builds = {}
for name, prior in old['componentBuilds'].items():
    # Frozen old component binaries/dependencies must not change. Their owned
    # source hashes are historical; this stage intentionally changes those files.
    original = json.loads(Path(prior['path']).read_text())
    audit(original['artifacts'])
    audit(original['imageCodecs'])
    p = ROOT/'component'/(name+'-build.json')
    current = json.loads(p.read_text())
    assert current['imageCodecs'] == original['imageCodecs'], name
    assert current['lock'] == json.loads(Path('components/skia/lock.json').read_text())
    # SDK lock paths are explicitly relative to its declared installation root.
    for tool in current['lock']['emsdkTools']:
        audit({**tool, 'path': str(Path('.codex-work/emsdk')/tool['path'])})
    for key in ['componentSources', 'artifacts', 'imageCodecs', 'profile']:
        audit(current[key])
    component_builds[name] = entry(p)

schemas = {}


def validate(name, value):
    if name not in schemas:
        schemas[name] = Draft202012Validator(json.loads(Path('contracts/generated/'+name+'.schema.json').read_text()))
    schemas[name].validate(value)


for case in reports['parity']['cases']:
    for k, name in [('pathResponse', 'image-raster-response'), ('sceneResponse', 'image-scene-response')]:
        validate(name, json.loads(Path(case[k]['path']).read_text()))
    if not case['success']:
        continue
    for k, name in [('request', 'image-raster-request'), ('sceneRequest', 'image-scene-request')]:
        validate(name, json.loads(Path(case[k]['path']).read_text()))
    expected = Path(case['expectedPixels']['path']).read_bytes()
    actual = Path(case['pixels']['path']).read_bytes()
    assert len(expected) == len(actual) == 24*20*4
    assert max(abs(a-b) for a, b in zip(expected, actual)) == case['maxChannelDeviation'] <= 1
    if case['name'].startswith('constant-corner-'):
        assert actual == bytes([80, 30, 100, 128])*(24*20)
for name in ['codec-regressions', 'resolution-regressions']:
    for case in reports[name]['cases']:
        response = json.loads(Path(case['response']['path']).read_text()) if 'name' in case else case['response']
        validate('image-decode-response', response)
    for case in reports[name]['rendered']:
        for key, schema in [('request', 'image-scene-request'), ('response', 'image-scene-response')]:
            validate(schema, json.loads(Path(case[key]['path']).read_text()))

benchmark = json.loads((ROOT/'benchmark.json').read_text())
audit(benchmark)
assert len(benchmark['results']) == 4
for result in benchmark['results']:
    assert result['maxChannelDeviation'] == 0
    for state in result['states']:
        for name in ['native', 'wasmAdapter']:
            assert len(state[name]['samplesMs']) == 25
            assert state[name]['medianMs'] == sorted(state[name]['samplesMs'])[12]
        validate('image-raster-request', json.loads(Path(state['request']['path']).read_text()))

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
    format='musteroffice.image-domain-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Explicit source-pixel domains in shared image paths/scenes and fixed Skia CPU stages. Draft sampling semantics, not source PPTX page integration or Office/WPS acceptance.',
    checks=dict(rustTests=538, newRustTests=4, strictClippy=True, rustfmt=True,
                schemasChecked=78, generatedTypeScriptAndTypeCheck=True, genericNegativeMutations=9,
                successfulNewFixtures=304, newPreflightFailures=9, independentPixels=145920,
                constantFieldCases=24, maxChannelDeviation=1, newPairedCalls=626,
                regressionPairedCalls=1095, currentPairedCalls=1721, wasmBadHostReplies=9,
                nativeWasmAsanUbsanTriples=321, malformedComponentFrames=15,
                legacyCapabilityAndWholeImageChecks=3, noSanitizerDiagnostics=True, leakSanitizer=False,
                sourceMarkdown=markdown, localLinks=links),
    sourceFiles=[entry(n) for n in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=tests, newTestNames=new_tests,
    validationLogs={n: entry(ROOT/(n+'.log')) for n in logs}, reports=report_entries,
    fixtureManifest=entry(ROOT/'cases/manifest.json'), benchmark=entry(ROOT/'benchmark.json'),
    componentBuilds=component_builds, previousComponentBuildsVerifiedUnchanged=old['componentBuilds'],
    codecInputLock=old['codecInputLock'], unchangedComponents=old['unchangedComponents'],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],
    artifacts={n: entry(p) for n, p in {
        'nativeDebugWorker': 'target/debug/mo-raster-worker', 'nativeDebugCli': 'target/debug/mo-cli',
        'nativeDebugTextWorker': 'target/debug/mo-text-worker',
        'rustDebugWasm': str(ROOT/'wasm-node/mo_wasm_bg.wasm'), 'rustWasmGlue': str(ROOT/'wasm-node/mo_wasm.js'),
        'cppWasm': str(ROOT/'component/mo-skia.wasm'), 'cppWasmGlue': str(ROOT/'component/mo-skia.mjs'),
        'typescriptAdapter': str(ROOT/'ts-raster/index.js'),
    }.items()},
    limitations=['Explicit domain sampling profile is draft, not measured Office/WPS crop and tile semantics.',
                 'No complete source PPTX image page compilation, clipping or native picture editing acceptance.',
                 'Coordinate bound covers input parameter quantization, not all inverse/shader arithmetic or filtering.',
                 'Synthetic warm component benchmark is scoped; no installer/RSS/product latency conclusion.',
                 'Complete presentation features, Agent packages and Musterwork acceptance remain open.'],
)
if '--seal' in sys.argv:
    assert not OUTPUT.exists(), 'evidence is immutable after sealing'
    OUTPUT.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
else:
    assert json.loads(OUTPUT.read_text()) == report, 'completed evidence differs from current closure'
print(json.dumps(dict(evidence=entry(OUTPUT), sources=len(sources), auditedRecords=records)))
