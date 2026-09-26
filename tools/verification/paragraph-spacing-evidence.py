"""Bind real spacing execution and independent references to a new source seal."""
import hashlib
import json
from pathlib import Path
import re
import sys
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/paragraph-spacing')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-native-baseline-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-paragraph-spacing-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == '41cd690774f4a0d3335a6e981bbffadceda43e692442b1ff2e3708537b9c833a'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'README.md', 'docs/README.md', 'contracts/README.md',
    *['docs/implementation/' + n + '.md' for n in ['development', 'progress', 'native-baseline', 'source-frame']],
    'crates/mo-harfbuzz-sys/examples/manifest_paths.rs',
    *['crates/mo-presentation-compile/src/' + n + '.rs' for n in [
        'lib', 'source_frame', 'source_frame/properties', 'source_frame/types',
        'source_text/number', 'source_text/types', 'source_text_page/precision']],
    *['crates/mo-text/src/' + n + '.rs' for n in ['flow/mod', 'flow/types', 'geometry/mod', 'geometry/types', 'manifest/prepared']],
    *['contracts/generated/' + n + '-request.schema.json' for n in ['line-geometry', 'paragraph-layout', 'paragraph-paths']],
    *['packages/contracts/src/generated/' + n + '-request.ts' for n in ['line-geometry', 'paragraph-layout', 'paragraph-paths']],
    'tools/verification/line_geometry_math.py',
}
changed = [r['path'] for r in old['sourceFiles'] if entry(r['path']) != r]
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = [
    'crates/mo-presentation-compile/src/source_number.rs',
    'crates/mo-presentation-compile/src/source_frame/spacing.rs',
    'crates/mo-text/src/geometry/spacing.rs',
    'crates/mo-harfbuzz-sys/tests/paragraph_spacing.rs',
    'docs/implementation/paragraph-spacing.md',
    *['tools/verification/paragraph-spacing-' + n for n in ['reference.py', 'parity.mjs', 'geometry.mjs', 'numerics.py', 'evidence.py']],
]
sources = {r['path'] for r in old['sourceFiles']} | set(added)
for p in sources:
    if Path(p).suffix in ['.rs', '.ts', '.py', '.mjs', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
for r in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(), *old['standardInputs']]:
    assert entry(r['path']) == r
logs = ['workspace-tests.log', 'clippy.log', 'fmt.log', 'workers-build.log', 'wasm-build.log',
        'bindgen.log', 'schema-check.log', 'types-check.log', 'contracts.log', 'parity.log',
        'reference.log', 'regressions.log', 'geometry.log', 'numerics.log']
for name in logs:
    s = (ROOT / name).read_text()
    assert not any(v in s for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
new = [t for t in tests if t not in old['rustTestNames']]
assert len(tests) == 492 and len(new) == 5
assert len(re.findall(r'^check contracts/generated/', (ROOT / 'schema-check.log').read_text(), re.M)) == 70
assert len(re.findall(r'^check .+\.ts$', (ROOT / 'types-check.log').read_text(), re.M)) == 70
contracts = json.loads((ROOT / 'contracts.log').read_text())
assert (contracts['schemas'], contracts['positiveInputs'], contracts['negativeMutations']) == (70, 1, 9)
parity = json.loads((ROOT / 'parity.json').read_text())
reference = json.loads((ROOT / 'reference.json').read_text())
regressions = json.loads((ROOT / 'regressions.json').read_text())
geometry = json.loads((ROOT / 'geometry.json').read_text())
numerics = json.loads((ROOT / 'numerics.json').read_text())
assert parity['counts'] == dict(priorRequests=89, changedDiagnostics=0, newRequests=17, cliRequests=17)
assert reference['counts'] == dict(packages=14, paragraphs=19, spacingDeclarations=41,
    styleValues=59, metricValues=57, lines=30, glyphCoordinates=58, decorationRectangles=2,
    interiorPixels=1625744, excludedEdgePixels=54256)
assert regressions['counts'] == dict(requests=307, pageRequests=100, textRequests=207, pixelOutputs=21)
assert geometry['counts'] == dict(priorRequests=12, newRequests=12, requests=24, success=14, errors=10)
assert numerics == dict(format='musteroffice.paragraph-spacing-numerics/1', layouts=14, lines=15,
    glyphs=23, coordinates=180, maximumWireErrorEmu='1/2', q32ProfileExact=True, rationalErrorBoundVerified=True)
records = 0


def audit(value):
    global records
    if isinstance(value, dict):
        if {'path', 'byteLength', 'sha256'} <= value.keys():
            assert entry(value['path']) == value, value['path']
            records += 1
        for v in value.values():
            audit(v)
    elif isinstance(value, list):
        for v in value:
            audit(v)


for value in [parity, reference, regressions, geometry, old['nativeComponents'], old['wasmBindgen']]:
    audit(value)


def validator(name):
    return Draft202012Validator(json.loads(Path('contracts/generated/' + name + '.schema.json').read_text()))


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate member')
        result[key] = value
    return result


request = validator('pptx-text-page-request')
response = validator('pptx-text-page-raster-response')
valid = invalid = 0
cases = {}
for c in parity['cases']:
    assert c['name'] not in cases
    cases[c['name']] = c
    try:
        q = json.loads(Path(c['request']['path']).read_text(), object_pairs_hook=unique)
    except ValueError:
        invalid += 1
    else:
        if list(request.iter_errors(q)):
            invalid += 1
        else:
            valid += 1
    response.validate(json.loads(Path(c['response']['path']).read_text()))
assert len(cases) == 106 and (valid, invalid) == (102, 4)
success = sum('pixels' in c for name, c in cases.items() if not name.startswith('prior-'))
assert success == 14
# Both rejection layers must retain exact native locations; neither can silently
# discard the authored extension to make the page render successfully.
for name, ordinal in [('unknown', 36), ('unknown-wrapper', 35)]:
    r = json.loads(Path(cases[name]['response']['path']).read_text())
    e = r['error']
    assert e['error']['location'] == dict(part='/ppt/slides/slide1.xml', object=42)
    assert e['detail']['kind'] == 'frame' and cases[name]['textCalls'] == 0
    reason = e['detail']['reason']
    if name == 'unknown':
        assert reason['kind'] == 'paragraphDeclaration'
        assert reason['declaration']['element'] == 'spcPct'
        origin = reason['declaration']['origin']
    else:
        assert reason['kind'] == 'text' and reason['reason']['kind'] == 'cascade'
        assert reason['reason']['reason']['kind'] == 'retainedContent'
        origin = reason['reason']['reason']['origin']
    assert origin == dict(kind='object', object=dict(part='/ppt/slides/slide1.xml', nativeId=42), sourceOrdinal=ordinal)
geometry_request = validator('line-geometry-request')
geometry_response = validator('line-geometry-response')
geometry_valid = geometry_invalid = 0
semantic = set()
for c in geometry['cases']:
    r = json.loads(Path(c['response']['path']).read_text())
    geometry_response.validate(r)
    try:
        q = json.loads(Path(c['request']['path']).read_text(), object_pairs_hook=unique)
    except ValueError:
        assert c['name'] in ['prior-duplicate', 'duplicate']
        geometry_invalid += 1
        continue
    errors = list(geometry_request.iter_errors(q))
    if errors:
        geometry_invalid += 1
        assert not c['success']
    else:
        geometry_valid += 1
        if not c['success']:
            semantic.add(c['name'])
            assert r['error']['code'] == 'INPUT_INVALID' and c['componentCalls'] == 0
            if c['name'] == 'prior-range':
                assert int(q['styles'][1]['baselineShift']['q32']) > 2**127 - 1
            elif c['name'] == 'negative':
                assert min(map(int, q['spacing']['heights'])) < 0
            else:
                assert c['name'] in ['missing', 'extra']
                assert len(q['spacing']['heights']) != len(q['styles'])
assert (geometry_valid, geometry_invalid) == (18, 6)
assert semantic == {'prior-range', 'missing', 'extra', 'negative'}

report = dict(
    format='musteroffice.paragraph-spacing-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Native line and paragraph percentage spacing, per-line style selection, exact zero, boundary gaps, actual Native/WASM/CLI execution and independent rational/pixel reference.',
    checks=dict(rustTests=len(tests), newRustTests=len(new), strictClippy=True, rustfmt=True, schemasChecked=70,
                generatedTypeScriptAndTypeCheck=True, contractNegativeMutations=9, newNativeWasmPageRequests=17,
                priorTextPageRequests=89, oldNativeWasmRequests=307, priorGeometryRequests=12,
                newGeometryRequests=12, totalCurrentParityRequests=437, newSuccessfulPages=success,
                newCliRequests=17, intendedOldDiagnosticChanges=0, validPageRequests=valid,
                invalidPageRequests=invalid, pageResponses=106, geometrySchemaValid=geometry_valid,
                geometrySchemaInvalid=geometry_invalid, geometrySemanticFailures=len(semantic),
                geometryResponses=24, **reference['counts']),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=tests, newTestNames=new, validationLogs=[entry(ROOT / n) for n in logs],
    parity=entry(ROOT / 'parity.json'), parityEvidence=parity,
    reference=entry(ROOT / 'reference.json'), referenceEvidence=reference,
    regressions=entry(ROOT / 'regressions.json'), regressionEvidence=regressions,
    geometry=entry(ROOT / 'geometry.json'), geometryEvidence=geometry,
    numerics=entry(ROOT / 'numerics.json'), numericEvidence=numerics,
    nativeComponents=old['nativeComponents'], rawRustWasm=entry('target/wasm32-unknown-unknown/debug/mo_wasm.wasm'),
    wasmBindgen=old['wasmBindgen'], wasmDeclarations=[entry(ROOT / 'wasm-node' / n) for n in ['mo_wasm.d.ts', 'mo_wasm_bg.wasm.d.ts', 'package.json']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'], standardInputs=old['standardInputs'],
    dependenciesChanged=False, wireContractsChanged=True,
    wireCompatibility='Existing spacing inputs preserved; additive styleMaximum variant in 3 request schemas/TS types. Explicit native 100 percent now uses authored line font size rather than Natural metrics.',
    limitations=[
        'Percentage source values use authored font sizes. First/last line paragraph bases, empty-line strut and metric/baseline policy remain draft pending Office/WPS acceptance.',
        'Owned synthetic outlines are not real-font or target-application quality evidence. No additional runtime/font dependencies.',
        'Independent page reference derives native spacing/size/provenance and checks FontTools metrics, coordinates and pixels; selected glyph integers and line topology remain inputs.',
        'Opaque pixel oracle excludes a two-pixel edge band; no new alpha/antialiasing, complete shaping or target-application fidelity claim.',
        'All 89 prior text page and 12 precise-baseline geometry responses unchanged; 307 older requests also replayed. Historical totals are separate.',
        'Current development binaries tested; 11 prior release artifacts unchanged. Complete text/high-level content, production Agent/Artifact/Worker, Musterwork/E0-E3 and product performance/package measurement remain incomplete.',
    ])

raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
print(json.dumps(dict(sources=len(sources), boundRecords=records, checks=report['checks'],
                      evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
