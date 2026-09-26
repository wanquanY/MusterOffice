"""Bind source/build/tests and real execution outputs without replacing an old seal."""
import hashlib
import json
from pathlib import Path
import re
import sys
import zipfile
from jsonschema import Draft202012Validator
from mce_reference import project, A, P

ROOT = Path('.codex-work/underline-paint')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-text-decorations-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-underline-paint-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == 'ffdc6d2aed16f477a2eef1b33ce13421c1a20bf92baab4b7aea9936edecf2e1b'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'README.md', 'docs/README.md',
    *['docs/implementation/' + n + '.md' for n in ['development', 'progress', 'text-decorations']],
    'crates/mo-harfbuzz-sys/tests/text_decorations.rs',
    'crates/mo-pptx/src/source/text/paint.rs',
    *['crates/mo-presentation-compile/src/' + n + '.rs' for n in [
        'source_text_page', 'source_text_page/decorations', 'source_text_page/glyphs', 'source_text_page/types']],
}
changed = [r['path'] for r in old['sourceFiles'] if entry(r['path']) != r]
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = [
    'crates/mo-pptx/tests/text_paint.rs',
    'crates/mo-harfbuzz-sys/tests/underline_paint.rs',
    'docs/implementation/underline-paint.md',
    *['tools/verification/underline-paint-' + n for n in ['reference.py', 'parity.mjs', 'evidence.py']],
]
sources = {r['path'] for r in old['sourceFiles']} | set(added)
for p in sources:
    if Path(p).suffix in ['.rs', '.ts', '.py', '.mjs', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
for r in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(), *old['standardInputs']]:
    assert entry(r['path']) == r
logs = ['workspace-tests.log', 'clippy.log', 'fmt.log', 'workers-build.log', 'wasm-build.log',
        'bindgen.log', 'schema-check.log', 'types-check.log', 'contracts.log', 'parity.log',
        'reference.log', 'regressions.log']
for name in logs:
    s = (ROOT / name).read_text()
    assert not any(v in s for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
new = [t for t in tests if t not in old['rustTestNames']]
assert len(tests) == 480 and len(new) == 6
assert len(re.findall(r'^check contracts/generated/', (ROOT / 'schema-check.log').read_text(), re.M)) == 70
assert len(re.findall(r'^check .+\.ts$', (ROOT / 'types-check.log').read_text(), re.M)) == 70
contracts = json.loads((ROOT / 'contracts.log').read_text())
assert (contracts['schemas'], contracts['positiveInputs'], contracts['negativeMutations']) == (70, 1, 9)
parity = json.loads((ROOT / 'parity.json').read_text())
reference = json.loads((ROOT / 'reference.json').read_text())
regressions = json.loads((ROOT / 'regressions.json').read_text())
assert parity['counts'] == dict(priorRequests=55, changedDiagnostics=0, newRequests=20, cliRequests=20)
assert reference['counts'] == dict(packages=12, metricValues=36, paintDeclarations=42,
                                 decorationRectangles=11, interiorPixels=1381352, excludedEdgePixels=58648)
assert regressions['counts'] == dict(requests=307, pageRequests=100, textRequests=207, pixelOutputs=21)
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


for value in [parity, reference, regressions, old['nativeComponents'], old['wasmBindgen']]:
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
assert len(cases) == 75 and (valid, invalid) == (71, 4)
success = sum('pixels' in c for name, c in cases.items() if not name.startswith('prior-'))
assert success == 16
# Verify actionable failures against the raw XML's physical source ordinals,
# independently of the compiler's declaration and run tables.
diagnostics = []
for name in ['missing-system', 'missing-placeholder', 'pattern']:
    c = cases[name]
    with zipfile.ZipFile(c['source']['path']) as z:
        xml, _, ordinals = project(z.read('ppt/slides/slide1.xml'), True)
    runs = xml.findall('.//p:sp/p:txBody/a:p/a:r', dict(a=A, p=P))
    assert len(runs) == 2
    child = runs[1].find('a:rPr/a:uFill/*', dict(a=A))
    assert child is not None
    e = json.loads(Path(c['response']['path']).read_text())['error']
    assert e['paintLocation'] == dict(paragraph=0, run=1, sourceOrdinal=ordinals[runs[1]])
    declaration = e['detail']['declaration']
    expected_element = 'pattFill' if name == 'pattern' else 'solidFill'
    assert child.tag == '{' + A + '}' + expected_element
    assert declaration == dict(element=expected_element, origin=dict(kind='object',
        object=dict(part='/ppt/slides/slide1.xml', nativeId=42), sourceOrdinal=ordinals[child]))
    assert c['textCalls'] == c['rasterCalls'] == 0
    diagnostics.append(dict(name=name, runOrdinal=ordinals[runs[1]], fillOrdinal=ordinals[child]))

report = dict(
    format='musteroffice.underline-paint-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Independent source single-underline solid/noFill, inheritance/provenance/shared color budgets, actual Native/WASM/CLI execution and independent owned-fixture pixel reference.',
    checks=dict(rustTests=len(tests), newRustTests=len(new), strictClippy=True, rustfmt=True, schemasChecked=70,
                generatedTypeScriptAndTypeCheck=True, contractNegativeMutations=9, newNativeWasmRequests=20,
                priorTextPageRequests=55, oldNativeWasmRequests=307, totalCurrentParityRequests=382,
                newSuccessfulPages=success, newCliRequests=20, intendedOldDiagnosticChanges=0,
                validRequests=valid, invalidRequests=invalid, responses=75, diagnosticSourceBindings=3,
                **reference['counts']),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=tests, newTestNames=new, validationLogs=[entry(ROOT / n) for n in logs],
    parity=entry(ROOT / 'parity.json'), parityEvidence=parity,
    reference=entry(ROOT / 'reference.json'), referenceEvidence=reference,
    regressions=entry(ROOT / 'regressions.json'), regressionEvidence=regressions, diagnosticBindings=diagnostics,
    nativeComponents=old['nativeComponents'], rawRustWasm=entry('target/wasm32-unknown-unknown/debug/mo_wasm.wasm'),
    wasmBindgen=old['wasmBindgen'], wasmDeclarations=[entry(ROOT / 'wasm-node' / n) for n in ['mo_wasm.d.ts', 'mo_wasm_bg.wasm.d.ts', 'package.json']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'], standardInputs=old['standardInputs'],
    dependenciesChanged=False, wireContractsChanged=False,
    limitations=[
        'Independent solid/noFill for single underline only. Spatial brushes, independent underline strokes, other decoration types and complete text layout remain full phase-one work.',
        'Owned synthetic outlines and Hebrew mappings are not real-font or Office/WPS compatibility evidence. Existing draft layout/metric policy remains in effect.',
        'Independent reference uses selected integer glyphs and line topology as inputs. It verifies 42 paint declarations, 36 line metric values and recomputes decoration coordinates/pixels, not all shaping.',
        'Opaque pixel oracle excludes a two-pixel edge band; no new alpha/antialiasing, text skip-ink, effect or target-application fidelity claim.',
        'All 55 prior text page responses and their successful pixels are unchanged; 307 older requests also replayed. Historic totals are not reported as current runs.',
        'Current development binaries tested; 11 prior release artifacts unchanged. No product latency/RSS/installer measurement, complete Agent/Artifact/Worker integration or Musterwork/E0-E3 acceptance.',
    ])
raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
print(json.dumps(dict(sources=len(sources), boundRecords=records, checks=report['checks'],
                      evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
