"""Bind current source/build/tests and native/WASM/reference outputs; never overwrite a seal."""
import hashlib
import json
from pathlib import Path
import re
import sys
from jsonschema import Draft202012Validator

ROOT=Path('.codex-work/text-decorations')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-text-resource-diagnostics-verification.json')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-text-decorations-verification.json')
def entry(path):
    p=Path(path);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256']=='56d7fabb5590a8520f0fa6cd0cf4688ed284cc6fd8de76aacdab740787c5703b'
old=json.loads(PREVIOUS.read_text())
allowed={
    'README.md','docs/README.md','fixtures/fonts/README.md','contracts/README.md','tools/verification/contracts.py',
    *['docs/implementation/'+n+'.md' for n in ['development','progress','text-page-runtime']],
    'contracts/generated/pptx-text-page-raster-response.schema.json',
    'packages/contracts/src/generated/pptx-text-page-raster-response.ts',
    'crates/mo-harfbuzz-sys/tests/source_text_page.rs','crates/mo-kernel-api/tests/pptx_text_page.rs',
    'crates/mo-kernel-api/src/pptx_page.rs','crates/mo-kernel-api/src/pptx_text_page/diagnostic.rs',
    'crates/mo-pptx/src/source/text/paint.rs',
    *['crates/mo-presentation-compile/src/'+n+'.rs' for n in ['source_frame','source_frame/backend','source_page/types','source_text_page','source_text_page/glyphs','source_text_page/types']],
    'crates/mo-text/src/manifest/prepared.rs','crates/mo-text/src/manifest/prepared_tests.rs',
}
changed=[p['path'] for p in old['sourceFiles'] if entry(p['path'])!=p]
assert set(changed)==allowed,(set(changed)-allowed,allowed-set(changed))
added=[
    'crates/mo-presentation-compile/src/source_text_page/decorations.rs',
    'crates/mo-harfbuzz-sys/tests/text_decorations.rs',
    'fixtures/fonts/owned-decorations.ttf','fixtures/fonts/decoration-manifest.json',
    'docs/implementation/text-decorations.md','tools/verification/decoration-font-fixtures.py',
    *['tools/verification/text-decorations-'+n for n in ['failures.py','reference.py','parity.mjs','evidence.py']],
]
sources={p['path'] for p in old['sourceFiles']}|set(added)
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.py','.mjs','.cpp','.h']:
        assert len(Path(p).read_text().splitlines())<=2000,p
for r in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(),*old['standardInputs']]:assert entry(r['path'])==r
logs=['workspace-tests.log','clippy.log','fmt.log','workers-build.log','wasm-build.log','bindgen.log','schema-check.log','types-check.log','contracts.log','parity.log','reference.log','regressions.log','fixture-rebuild.log']
for name in logs:
    s=(ROOT/name).read_text()
    assert not any(v in s for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),name
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M)
new=[t for t in tests if t not in old['rustTestNames']]
assert len(tests)==474 and len(new)==5
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M))==70
assert len(re.findall(r'^check .+\.ts$',(ROOT/'types-check.log').read_text(),re.M))==70
parity=json.loads((ROOT/'parity.json').read_text())
reference=json.loads((ROOT/'reference.json').read_text())
regressions=json.loads((ROOT/'regressions.json').read_text())
assert parity['counts']==dict(priorRequests=39,changedDiagnostics=2,newRequests=16,cliRequests=16)
assert reference['counts']==dict(packages=14,metricValues=48,decorationRectangles=23,interiorPixels=1589995,excludedEdgePixels=90005)
assert regressions['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
records=0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            assert entry(value['path'])==value,value['path'];records+=1
        for v in value.values():audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
for value in [parity,reference,regressions,old['nativeComponents']]:audit(value)
request=Draft202012Validator(json.loads(Path('contracts/generated/pptx-text-page-request.schema.json').read_text()))
response=Draft202012Validator(json.loads(Path('contracts/generated/pptx-text-page-raster-response.schema.json').read_text()))
valid=invalid=0
def unique(pairs):
    result={}
    for key,value in pairs:
        if key in result:raise ValueError('duplicate member')
        result[key]=value
    return result
for c in parity['cases']:
    try:q=json.loads(Path(c['request']['path']).read_text(),object_pairs_hook=unique)
    except ValueError:invalid+=1
    else:
        errors=list(request.iter_errors(q))
        if errors:invalid+=1
        else:valid+=1
    response.validate(json.loads(Path(c['response']['path']).read_text()))
assert (valid,invalid)==(51,4)
report=dict(
    format='musteroffice.text-decorations-verification/1',previousEvidence=entry(PREVIOUS),
    scope='Native source single underline/strike, selected-instance optional metrics, shared placement/budgets, Native/WASM/CLI raster execution and independent owned-fixture coordinate/pixel reference.',
    checks=dict(rustTests=len(tests),newRustTests=len(new),strictClippy=True,rustfmt=True,schemasChecked=70,
                generatedTypeScriptAndTypeCheck=True,newNativeWasmRequests=16,priorTextPageRequests=39,oldNativeWasmRequests=307,
                totalCurrentParityRequests=362,newSuccessfulPages=14,newCliRequests=16,intendedOldDiagnosticChanges=2,
                validRequests=valid,invalidRequests=invalid,responses=55,**reference['counts']),
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new,validationLogs=[entry(ROOT/n) for n in logs],
    parity=entry(ROOT/'parity.json'),parityEvidence=parity,reference=entry(ROOT/'reference.json'),referenceEvidence=reference,
    regressions=entry(ROOT/'regressions.json'),regressionEvidence=regressions,
    nativeComponents=old['nativeComponents'],rawRustWasm=entry('target/wasm32-unknown-unknown/debug/mo_wasm.wasm'),
    wasmBindgen=old['wasmBindgen'],wasmDeclarations=[entry(ROOT/'wasm-node'/n) for n in ['mo_wasm.d.ts','mo_wasm_bg.wasm.d.ts','package.json']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],standardInputs=old['standardInputs'],
    dependenciesChanged=False,
    limitations=[
        'Single underline/strike only, follow-text solid/noFill, draft font metric policy. Double/heavy/dashed/wavy/words-only and independent underline fill/stroke remain explicit unsupported cases and full phase-one goals.',
        'Original font/outline fixtures and synthetic Hebrew mappings are not real-font quality or Office/WPS compatibility evidence. Baseline superscript/subscript, vertical text, columns and autofit remain unsupported.',
        'Independent reference validates 48 line metric values through FontTools and recomputes decoration rectangles from native declarations and selected glyph integer outputs. Selected shaping and line topology are inputs, not independently certified.',
        'Pixel oracle excludes a two-pixel edge band; no antialiasing, text skip-ink, effect, complete transparency or target-application fidelity claim.',
        'Old single-underline fixtures now reach actual font metric failures (zero thickness). All other prior replies and all prior successful pixel outputs are unchanged.',
        'Current development artifacts were tested; 11 prior release artifacts are unchanged. No product latency/RSS/installer measurement or production Musterwork/Artifact/Agent integration and E0-E3 acceptance.',
    ])
raw=json.dumps(report,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
print(json.dumps(dict(sources=len(sources),boundRecords=records,checks=report['checks'],evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
