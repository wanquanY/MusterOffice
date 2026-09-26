"""Freeze typed font requirements, native run locations and real recovery runs."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT=Path('.codex-work/text-resource-diagnostics')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-text-page-runtime-verification.json')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-text-resource-diagnostics-verification.json')
def entry(path):
    p=Path(path);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256']=='4ce18b01e729d4a28b5d53f6e165c419e868427637ac71b35791d4798e787a0e'
old=json.loads(PREVIOUS.read_text())
response_names=['cascade-response','font-metrics-response','font-outlines-response','itemization-response',
                'line-geometry-response','line-shape-response','paragraph-layout-response','paragraph-paths-response',
                'paragraph-shape-response','pptx-text-page-raster-response','shape-response']
allowed={
    *['contracts/generated/'+n+'.schema.json' for n in response_names],
    *['packages/contracts/src/generated/'+n+'.ts' for n in response_names],
    *['crates/mo-kernel-api/src/'+n+'.rs' for n in ['cascade','metrics','outlines','paragraph','pptx_page','pptx_text_page/diagnostic','text']],
    'crates/mo-kernel-api/tests/pptx_text_page.rs','crates/mo-pptx/src/source/text/paint.rs',
    *['crates/mo-presentation-compile/src/'+n+'.rs' for n in ['source_frame','source_text','source_text/types','source_text_page','source_text_page/glyphs']],
    *['crates/mo-text/src/'+n+'.rs' for n in ['lib','manifest/binding','manifest/mod','manifest/tests']],
    'packages/contracts/tests/wire.ts','tools/verification/text-page-runtime-regressions.mjs',
    'README.md','docs/README.md','docs/implementation/development.md','docs/implementation/progress.md','docs/implementation/text-page-runtime.md',
}
changed=[p['path'] for p in old['sourceFiles'] if entry(p['path'])!=p]
assert set(changed)==allowed,(set(changed)-allowed,allowed-set(changed))
added=['crates/mo-text/src/manifest/selection.rs','docs/implementation/text-resource-diagnostics.md',
       *['tools/verification/text-resource-'+n for n in ['fixtures.py','parity.mjs','reference.py','evidence.py']]]
sources={p['path'] for p in old['sourceFiles']}|set(added)
assert len(sources)==len(old['sourceFiles'])+len(added)
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.py','.mjs','.h','.cpp']:
        assert len(Path(p).read_text().splitlines())<=2000,p
for record in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(),*old['standardInputs']]:assert entry(record['path'])==record
logs=['manifest-tests.log','api-tests.log','workspace-tests.log','clippy.log','fmt.log','workers-build.log',
      'wasm-build.log','bindgen.log','schema-check.log','types-check.log','parity.log','reference.log','regressions.log']
for name in logs:
    s=(ROOT/name).read_text();assert not any(v in s for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),name
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M)
new=[t for t in tests if t not in old['rustTestNames']]
assert len(tests)==469 and len(new)==6
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M))==70
assert len(re.findall(r'^check .+\.ts$',(ROOT/'types-check.log').read_text(),re.M))==70
parity=json.loads((ROOT/'parity.json').read_text());reference=json.loads((ROOT/'reference.json').read_text());regressions=json.loads((ROOT/'regressions.json').read_text())
assert parity['counts']==dict(priorRequests=27,additiveDiagnostics=2,newRequests=12,resourceRecoveries=5,cliRequests=12)
assert reference['counts']==dict(physicalPaintRuns=4,fontUses=9,validRequests=35,invalidRequests=4,responses=39)
assert regressions['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
def audit(value):
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():assert entry(value['path'])==value,value['path']
        for v in value.values():audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
for value in [parity,reference,regressions,old['nativeComponents']]:audit(value)
report=dict(
    format='musteroffice.text-resource-diagnostics-verification/1',previousEvidence=entry(PREVIOUS),
    scope='Typed root manifest prerequisites, all source uses of a failed paragraph computation style, physical run paint locations, and actual explicit-resource recovery through Native CLI/worker and Rust WASM.',
    checks=dict(rustTests=469,newRustTests=6,strictClippy=True,rustfmt=True,schemasChecked=70,generatedTypeScriptAndTypeCheck=True,
                newNativeWasmRequests=12,priorTextPageRequests=27,oldNativeWasmRequests=307,totalCurrentParityRequests=346,
                intendedAdditiveOldDiagnostics=2,successfulExplicitRecoveries=5,newCliRequests=12,physicalPaintRuns=4,fontUses=9),
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new,validationLogs=[entry(ROOT/n) for n in logs],
    parity=entry(ROOT/'parity.json'),parityEvidence=parity,reference=entry(ROOT/'reference.json'),referenceEvidence=reference,
    regressions=entry(ROOT/'regressions.json'),regressionEvidence=regressions,
    nativeComponents=old['nativeComponents'],rawRustWasm=entry('target/wasm32-unknown-unknown/debug/mo_wasm.wasm'),
    wasmBindgen=old['wasmBindgen'],wasmDeclarations=[entry(ROOT/'wasm-node'/n) for n in ['mo_wasm.d.ts','mo_wasm_bg.wasm.d.ts','package.json']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],standardInputs=old['standardInputs'],
    dependenciesChanged=False,
    limitations=[
        'First encountered missing paragraph style plus all its source bindings, not a whole-document font inventory or complete fallback/resource-acquisition system.',
        'Explicit host-provided mappings/instances are revalidated. No implicit system discovery, family replacement or synthetic style. Owned italic prerequisite remains unresolved because the test catalog has no italic instance.',
        'Five recoveries validate actual resource/context routing and rendered pixels, including explicit wght selection. These synthetic fonts do not establish real-font visual quality or Office/WPS fidelity.',
        'Only two prior text-page error replies add paintLocation; the remaining prior fields/responses and all successful images are unchanged. Other 307 Native/WASM requests and 21 old page pixels are unchanged.',
        'Independent XML reference covers recorded owned direct-font fixtures, not full native theme, font-slot, language or target-application behavior.',
        'Source text page remains a draft solid text operation; decorations, complex layout and complete phase-one advanced content still require implementation.',
        'Current development Native/WASM artifacts were tested. The 11 prior release artifacts are unchanged. No new product latency/RSS/installer measurements, production host/Artifact/Musterwork integration or E0-E3 acceptance.',
    ])
raw=json.dumps(report,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as file:file.write(raw)
print(json.dumps(dict(sources=len(sources),checks=report['checks'],evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
