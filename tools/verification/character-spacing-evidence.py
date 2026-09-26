"""Bind real spacing execution and independent references to a new source seal."""
import hashlib
import json
from pathlib import Path
import re
import sys
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/character-spacing')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-paragraph-spacing-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-character-spacing-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == '834cfbbbb98b3a064d93159f70e973ef8db86a0fa88d91c63266a3e3fa880ccc'
old = json.loads(PREVIOUS.read_text())
allowed = {'README.md',
 'contracts/README.md',
 'contracts/generated/line-geometry-request.schema.json',
 'contracts/generated/paragraph-layout-request.schema.json',
 'contracts/generated/paragraph-paths-request.schema.json',
 'crates/mo-harfbuzz-sys/examples/manifest_paths.rs',
 'crates/mo-presentation-compile/src/source_number.rs',
 'crates/mo-presentation-compile/src/source_text/assemble.rs',
 'crates/mo-presentation-compile/src/source_text/number.rs',
 'crates/mo-presentation-compile/src/source_text/style.rs',
 'crates/mo-presentation-compile/src/source_text/types.rs',
 'crates/mo-presentation-compile/src/source_text_page/decorations.rs',
 'crates/mo-presentation-compile/src/source_text_page/precision.rs',
 'crates/mo-presentation-compile/tests/source_glyphs.rs',
 'crates/mo-text/src/flow/tests.rs',
 'crates/mo-text/src/geometry/mod.rs',
 'crates/mo-text/src/geometry/order.rs',
 'crates/mo-text/src/geometry/test_support.rs',
 'crates/mo-text/src/geometry/tests.rs',
 'crates/mo-text/src/geometry/types.rs',
 'crates/mo-text/src/manifest/prepared_tests.rs',
 'crates/mo-text/src/scene/tests.rs',
 'docs/README.md',
 'docs/implementation/development.md',
 'docs/implementation/paragraph-spacing.md',
 'docs/implementation/progress.md',
 'fixtures/fonts/README.md',
 'packages/contracts/src/generated/line-geometry-request.ts',
 'packages/contracts/src/generated/paragraph-layout-request.ts',
 'packages/contracts/src/generated/paragraph-paths-request.ts',
 'tools/verification/line_geometry_math.py'}
changed = [r['path'] for r in old['sourceFiles'] if entry(r['path']) != r]
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = ['crates/mo-text/src/geometry/pen.rs',
 'crates/mo-harfbuzz-sys/tests/character_spacing.rs',
 'fixtures/fonts/owned-tracking.ttf',
 'fixtures/fonts/tracking-manifest.json',
 'tools/verification/tracking-font-fixtures.py',
 'docs/implementation/character-spacing.md',
 'tools/verification/character-spacing-reference.py',
 'tools/verification/character-spacing-parity.mjs',
 'tools/verification/character-spacing-geometry.mjs',
 'tools/verification/character-spacing-numerics.py',
 'tools/verification/character-spacing-evidence.py']
sources = {r['path'] for r in old['sourceFiles']} | set(added)
for p in sources:
    if Path(p).suffix in ['.rs', '.ts', '.py', '.mjs', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
for r in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(), *old['standardInputs']]:
    assert entry(r['path']) == r
logs = ['workspace-tests.log', 'clippy.log', 'fmt.log', 'workers-build.log', 'wasm-build.log',
        'bindgen.log', 'schema-check.log', 'types-check.log', 'contracts.log', 'parity.log',
        'reference.log', 'regressions.log', 'geometry.log', 'numerics.log', 'font.log']
for name in logs:
    s = (ROOT / name).read_text()
    assert not any(v in s for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
new = [t for t in tests if t not in old['rustTestNames']]
assert len(tests) == 499 and len(new) == 7
assert len(re.findall(r'^check contracts/generated/', (ROOT / 'schema-check.log').read_text(), re.M)) == 70
assert len(re.findall(r'^check .+\.ts$', (ROOT / 'types-check.log').read_text(), re.M)) == 70
contracts = json.loads((ROOT / 'contracts.log').read_text())
assert (contracts['schemas'], contracts['positiveInputs'], contracts['negativeMutations']) == (70, 1, 9)
parity = json.loads((ROOT / 'parity.json').read_text())
reference = json.loads((ROOT / 'reference.json').read_text())
regressions = json.loads((ROOT / 'regressions.json').read_text())
geometry = json.loads((ROOT / 'geometry.json').read_text())
numerics = json.loads((ROOT / 'numerics.json').read_text())
assert parity['counts'] == dict(priorRequests=106, changedDiagnostics=0, newRequests=18, cliRequests=18)
assert reference['counts'] == dict(packages=15, paragraphs=15, spacingDeclarations=0, trackingDeclarations=19,
    styleValues=33, metricValues=45, lines=16, glyphCoordinates=74, decorationRectangles=2,
    interiorPixels=1725584, excludedEdgePixels=74416)
assert regressions['counts'] == dict(requests=307, pageRequests=100, textRequests=207, pixelOutputs=21)
assert geometry['counts'] == dict(priorRequests=24, newRequests=17, requests=41, success=25, errors=16)
assert numerics == dict(format='musteroffice.character-spacing-numerics/1', layouts=25, lines=26,
    glyphs=44, coordinates=321, maximumWireErrorEmu='1/2', q32ProfileExact=True, rationalErrorBoundVerified=True)
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
assert len(cases) == 124 and (valid, invalid) == (120, 4)
success = sum('pixels' in c for name, c in cases.items() if not name.startswith('prior-'))
assert success == 15
for name, expected in [('lexical','native tracking lexical bytes'),('range','native tracking range'),('cluster-conflict','GraphemeStyleConflict')]:
    e=json.loads(Path(cases[name]['response']['path']).read_text())['error']
    assert e['error']['location']==dict(part='/ppt/slides/slide1.xml',object=42)
    assert expected in e['error']['message'] and cases[name]['textCalls']==0
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
        assert c['name'] in ['prior-prior-duplicate', 'prior-duplicate', 'duplicate']
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
            if c['name'] == 'overflow':
                assert r['error']['code']=='LIMIT_EXCEEDED' and c['componentCalls']>0
                assert 'line geometry numeric range' in r['error']['message']
                assert int(q['styles'][0]['clusterSpacing'])==2**127-1
                continue
            assert r['error']['code'] == 'INPUT_INVALID' and c['componentCalls'] == 0
            if c['name'] == 'prior-prior-range':
                assert int(q['styles'][1]['baselineShift']['q32']) > 2**127 - 1
            elif c['name'] == 'range':
                assert int(q['styles'][0]['clusterSpacing']) > 2**127 - 1
            elif c['name'] == 'prior-negative':
                assert min(map(int, q['spacing']['heights'])) < 0
            else:
                assert c['name'] in ['prior-missing', 'prior-extra']
                assert len(q['spacing']['heights']) != len(q['styles'])
assert (geometry_valid, geometry_invalid) == (31, 10)
assert semantic == {'prior-prior-range', 'prior-missing', 'prior-extra', 'prior-negative', 'range', 'overflow'}

report = dict(
    format='musteroffice.character-spacing-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Native signed character spacing, shared fitting/placement/decoration pens, cluster and ligature policies, actual Native/WASM/CLI execution and independent rational/pixel reference.',
    checks=dict(rustTests=len(tests), newRustTests=len(new), strictClippy=True, rustfmt=True, schemasChecked=70,
                generatedTypeScriptAndTypeCheck=True, contractNegativeMutations=9, newNativeWasmPageRequests=18,
                priorTextPageRequests=106, oldNativeWasmRequests=307, priorGeometryRequests=24,
                newGeometryRequests=17, totalCurrentParityRequests=472, newSuccessfulPages=success,
                newCliRequests=18, intendedOldDiagnosticChanges=0, validPageRequests=valid,
                invalidPageRequests=invalid, pageResponses=124, geometrySchemaValid=geometry_valid,
                geometrySchemaInvalid=geometry_invalid, geometrySemanticFailures=len(semantic),
                geometryResponses=41, **reference['counts']),
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
    wireCompatibility='Optional clusterSpacing Q32 field in 3 request schemas/TS types. Absent/zero value preserves prior geometry and pixels; Rust style constructors require the new field.',
    addedTestFont=entry('fixtures/fonts/owned-tracking.ttf'), testFontManifest=entry('fixtures/fonts/tracking-manifest.json'),
    limitations=[
        'Visual trailing spacing per shaped cluster includes the final cluster. Optional liga/clig are disabled by native nonzero tracking; required shaping remains. Office/WPS tracking/ligature/script policies require acceptance.',
        'Owned synthetic outlines are not real-font or target-application quality evidence. No additional runtime dependencies; one owned GSUB test font added, not a runtime font pack.',
        'Independent page reference derives native tracking/size/provenance and checks FontTools metrics, coordinates and pixels; selected glyph integers and line topology remain inputs.',
        'Opaque pixel oracle excludes a two-pixel edge band; no new alpha/antialiasing, complete shaping or target-application fidelity claim.',
        'All 106 prior text page and 24 prior geometry responses unchanged; 307 older requests also replayed. Historical totals are separate.',
        'Current development binaries tested; 11 prior release artifacts unchanged. Complete text/high-level content, production Agent/Artifact/Worker, Musterwork/E0-E3 and product performance/package measurement remain incomplete.',
    ])

raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
print(json.dumps(dict(sources=len(sources), boundRecords=records, checks=report['checks'],
                      evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
