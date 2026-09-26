"""Bind source/build/tests and real execution outputs without replacing an old seal."""
import hashlib
import json
from pathlib import Path
import re
import sys
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/native-baseline')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-underline-paint-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-native-baseline-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == '04ef7cf952c1c5ea5cbad326c2163b022371cca8176f4ffbad7f92246d5f2acd'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'README.md', 'docs/README.md', 'contracts/README.md',
    *['docs/implementation/' + n + '.md' for n in ['development', 'progress', 'underline-paint', 'line-geometry']],
    'crates/mo-harfbuzz-sys/examples/manifest_paths.rs', 'crates/mo-harfbuzz-sys/tests/text_decorations.rs',
    *['crates/mo-presentation-compile/src/' + n + '.rs' for n in ['source_text', 'source_text/assemble', 'source_text/style', 'source_text/types', 'source_text_page/precision']],
    'crates/mo-presentation-compile/tests/source_glyphs.rs',
    *['crates/mo-text/src/' + n + '.rs' for n in ['flow/tests', 'geometry/mod', 'geometry/test_support', 'geometry/tests', 'geometry/types', 'manifest/prepared_tests', 'scene/tests']],
    *['contracts/generated/' + n + '-request.schema.json' for n in ['line-geometry', 'paragraph-layout', 'paragraph-paths']],
    *['packages/contracts/src/generated/' + n + '-request.ts' for n in ['line-geometry', 'paragraph-layout', 'paragraph-paths']],
    'tools/verification/line_geometry_math.py',
}
changed = [r['path'] for r in old['sourceFiles'] if entry(r['path']) != r]
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = [
    'crates/mo-presentation-compile/src/source_text/number.rs',
    'crates/mo-harfbuzz-sys/tests/native_baseline.rs',
    'docs/implementation/native-baseline.md',
    *['tools/verification/native-baseline-' + n for n in ['reference.py', 'parity.mjs', 'geometry.mjs', 'numerics.py', 'evidence.py']],
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
assert len(tests) == 487 and len(new) == 7
assert len(re.findall(r'^check contracts/generated/', (ROOT / 'schema-check.log').read_text(), re.M)) == 70
assert len(re.findall(r'^check .+\.ts$', (ROOT / 'types-check.log').read_text(), re.M)) == 70
contracts = json.loads((ROOT / 'contracts.log').read_text())
assert (contracts['schemas'], contracts['positiveInputs'], contracts['negativeMutations']) == (70, 1, 9)
parity = json.loads((ROOT / 'parity.json').read_text())
reference = json.loads((ROOT / 'reference.json').read_text())
regressions = json.loads((ROOT / 'regressions.json').read_text())
geometry = json.loads((ROOT / 'geometry.json').read_text())
numerics = json.loads((ROOT / 'numerics.json').read_text())
assert parity['counts'] == dict(priorRequests=75, changedDiagnostics=0, newRequests=14, cliRequests=14)
assert reference['counts'] == dict(packages=11, metricValues=33, paintDeclarations=21,
    baselineDeclarations=16, baselineValues=32, glyphCoordinates=58, lines=12,
    decorationRectangles=6, interiorPixels=1266594, excludedEdgePixels=53406)
assert regressions['counts'] == dict(requests=307, pageRequests=100, textRequests=207, pixelOutputs=21)
assert geometry['counts'] == dict(requests=12, success=7, errors=5)
assert numerics == dict(format='musteroffice.native-baseline-numerics/1', layouts=7, lines=7,
    glyphs=14, coordinates=91, maximumWireErrorEmu='1/2', q32ProfileExact=True, rationalErrorBoundVerified=True)
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
request = Draft202012Validator(json.loads(Path('contracts/generated/pptx-text-page-request.schema.json').read_text()))
response = Draft202012Validator(json.loads(Path('contracts/generated/pptx-text-page-raster-response.schema.json').read_text()))
valid = invalid = 0


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate member')
        result[key] = value
    return result


cases = {}
for c in parity['cases']:
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
assert len(cases) == 89 and (valid, invalid) == (85, 4)
success = sum('pixels' in c for name, c in cases.items() if not name.startswith('prior-'))
assert success == 11
geometry_request = Draft202012Validator(json.loads(Path('contracts/generated/line-geometry-request.schema.json').read_text()))
geometry_response = Draft202012Validator(json.loads(Path('contracts/generated/line-geometry-response.schema.json').read_text()))
geometry_valid = geometry_invalid = 0
for c in geometry['cases']:
    r = json.loads(Path(c['response']['path']).read_text())
    geometry_response.validate(r)
    try:
        q = json.loads(Path(c['request']['path']).read_text(), object_pairs_hook=unique)
    except ValueError:
        assert c['name'] == 'duplicate'
        geometry_invalid += 1
        continue
    errors = list(geometry_request.iter_errors(q))
    if errors:
        geometry_invalid += 1
        assert not c['success']
    else:
        geometry_valid += 1
        if not c['success']:
            # x-integer-maximum is a semantic constraint: the base JSON Schema
            # accepts this lexical form, but actual Rust must reject its range.
            assert c['name'] == 'range' and int(q['styles'][1]['baselineShift']['q32']) > 2**127 - 1
            assert r['error']['code'] == 'INPUT_INVALID'
assert (geometry_valid, geometry_invalid) == (8, 4)

report = dict(
    format='musteroffice.native-baseline-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Source baseline percentages, precise shared layout input, origin/coalescing uncertainty, actual Native/WASM/CLI execution and independent rational/pixel reference.',
    checks=dict(rustTests=len(tests), newRustTests=len(new), strictClippy=True, rustfmt=True, schemasChecked=70,
                generatedTypeScriptAndTypeCheck=True, contractNegativeMutations=9, newNativeWasmPageRequests=14,
                priorTextPageRequests=75, oldNativeWasmRequests=307, newGeometryRequests=12, totalCurrentParityRequests=408,
                newSuccessfulPages=success, newCliRequests=14, intendedOldDiagnosticChanges=0,
                validPageRequests=valid, invalidPageRequests=invalid, pageResponses=89,
                geometrySchemaValid=geometry_valid, geometrySchemaInvalid=geometry_invalid, geometrySemanticRangeFailure=1,
                geometryResponses=12, **reference['counts']),
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
    dependenciesChanged=False, wireContractsChanged=True, wireCompatibility='Existing integer EMU form and results preserved; additive Q32 object in 3 request schemas/TS types.',
    limitations=[
        'Baseline uses authored font size and current draft metric/line policy. Office/WPS automatic size, superscript/subscript UI and full layout rules are not certified.',
        'Owned synthetic outlines and Hebrew mappings are not real-font or target-application quality evidence. No additional runtime/font dependencies.',
        'Independent page reference derives native size/baseline/provenance and checks FontTools metrics, coordinates and pixels; selected glyph integers and line topology remain inputs.',
        'Opaque pixel oracle excludes a two-pixel edge band; no new alpha/antialiasing, complete shaping or target-application fidelity claim.',
        'All 75 prior text page responses and their successful pixels are unchanged; 307 older requests also replayed. Historical totals are separate.',
        'Current development binaries tested; 11 prior release artifacts unchanged. Complete text/high-level content, production Agent/Artifact/Worker, Musterwork/E0-E3 and product performance/package measurement remain incomplete.',
    ])

raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
print(json.dumps(dict(sources=len(sources), boundRecords=records, checks=report['checks'],
                      evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
