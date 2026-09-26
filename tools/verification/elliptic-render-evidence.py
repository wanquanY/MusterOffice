"""Seal actual raster pixels, bounded failure, caller regressions and timings."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/elliptic-render')
previous=Path('docs/reviews/evidence/2026-09-25-elliptic-scalar-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-elliptic-render-verification.json')
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='c9d2e4298453ceb35fc0d35d7d8e1b02cef6ba4597870222b4195934f9d0ef08'
old=json.loads(previous.read_text());prior={v['path']:v for v in old['sourceFiles']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
components/skia/mo_gradient.cpp
components/skia/mo_gradient.h
components/skia/mo_gradient_plane.cpp
components/skia/mo_gradient_plane_stage.h
components/skia/mo_skia.cpp
components/skia/mo_skia.h
packages/raster-component/src/index.ts
tools/components/build-skia.py
tools/verification/skia-probe.cpp'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,changed
added=set('''components/skia/mo_elliptic_prepared.h
components/skia/mo_elliptic_prepared.cpp
components/skia/elliptic-gradient.patch
docs/implementation/elliptic-gradient-raster.md
tools/verification/elliptic-render-components.mjs
tools/verification/elliptic_polynomial_reference.py
tools/verification/elliptic-render-reference.py
tools/verification/elliptic-prepared-probe.cpp
tools/verification/elliptic-prepared-checks.py
tools/verification/elliptic-prepared-parity.mjs
tools/verification/elliptic-render-workspace.py
tools/verification/elliptic_frames.mjs
tools/verification/elliptic-render-benchmark.mjs
tools/verification/elliptic-render-evidence.py'''.splitlines())
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
for p in prior:
    if p not in allowed:assert entry(p)==prior[p],p
reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['components','reference','benchmark','workspace-checks']}
c,r,b,w=[reports[n] for n in reports];p=json.loads((root/'prepared/verification.json').read_text())
assert (c['triples'],c['legacyFrames'],c['sourceFrames'],len(c['negatives']))==(551,490,18,19)
assert len(c['cases'])==23 and sum(v['expectedStatus']==0 for v in c['cases'])==20
assert c['oldCapabilityRejections']==2 and c['failureRecovery']
assert p['cases']==2646 and p['statuses']=={'0':2638,'1':5,'2':1,'3':2}
assert len(p['commands'])==4 and all(x['exitCode']==0 for x in p['commands'])
assert r['pixels']==48233 and r['multipleRootPixels']==32 and len(r['cases'])==19 and len(r['excluded'])==1
assert max(x['maxGrayError'] for x in r['cases'])<.5
assert len(w)==6 and all(x['exitCode']==0 for x in w)
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==611
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==82
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==82
for x in [*w,*p['commands']]:
    s=Path(x['log']).read_text()
    assert not any(v in s for v in ['error:','FAILED','Traceback','AssertionError','runtime error:','AddressSanitizer']),x['name']
assert len(b['results'])==6
for result in b['results']:
    for target in ['native','wasmAdapter']:
        times=result[target]['samplesMs'];assert len(times)==25
        assert result[target]['medianMs']==sorted(times)[12] and result[target]['p95Ms']==sorted(times)[23]

records=0;skipped=set();mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(value,historical=False):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            path=value['path']
            if historical and path in allowed|mutable:skipped.add(path)
            else:assert all(value[k]==v for k,v in entry(path).items()),path;records+=1
        for x in value.values():audit(x,historical)
    elif isinstance(value,list):
        for x in value:audit(x,historical)
audit(reports)
for rec in old['reports'].values():audit(rec);audit(json.loads(Path(rec['path']).read_text()),True)
audit(old['artifacts'],True);audit(old['baselineSnapshots']);audit(old['baselineManifest'])
audit(old['existingProductionArtifacts'],True)
for rec in old['componentBuilds'].values():
    audit(rec);v=json.loads(Path(rec['path']).read_text())
    for key in ['componentSources','artifacts','imageCodecs']:audit(v[key],True)
builds={}
for name in ['native','wasm','native-asan']:
    path=root/'component'/(name+'-build.json');rec=entry(path);audit(rec)
    build=json.loads(path.read_text());assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    assert build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(build[key])
    builds[name]=rec
current={k:entry(v) for k,v in {
    'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
    'cppWasm':str(root/'component/mo-skia.wasm'),'typescriptRaster':str(root/'ts-raster/index.js'),
    'rustWasm':'.codex-work/radial-observation/wasm-node/mo_wasm_bg.wasm'}.items()}
assert current['cppWasm']['byteLength']==2343781
assert current['rustWasm']==old['existingProductionArtifacts']['rustWasm']

# Pure fixture generator was factored after execution; replay its output and
# require every tested frame byte, including both budget cases, to be unchanged.
script="""import fs from 'node:fs';import assert from 'node:assert/strict';import {ellipticFrame} from './tools/verification/elliptic_frames.mjs';
for(const c of JSON.parse(fs.readFileSync('.codex-work/elliptic-render/components.json')).cases)
assert.deepEqual(Buffer.from(ellipticFrame(c.options).buffer),fs.readFileSync(c.frame.path));"""
subprocess.run(['node','--input-type=module','-e',script],check=True)
markdown=links=0
for name in sorted(sources):
    path=Path(name)
    if path.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:assert (path.parent/target).exists() or (path.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[entry(path) for path in sorted(root.rglob('*')) if path.is_file() and
    'component' not in path.relative_to(root).parts and 'attempt1' not in path.relative_to(root).parts and
    path.name not in ['next.md','seal.log','seal-check.log']]
report=dict(format='musteroffice.elliptic-render-verification/1',previousEvidence=entry(previous),
    scope='Actual V12 elliptic raster component and TS adapter, prepared concentric math, bounded failure, legacy pixels and performance observations. Rust/PPTX production lowering is not implemented.',
    sourceFiles=[entry(path) for path in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,commandChecks=w,checks=dict(rustTests=611,strictClippy=True,rustfmt=True,schemas=82,typescript=True,
        preparedCases=2646,preparedSuccesses=2638,componentTriples=551,legacyFrames=490,sourceFrames=18,
        newRasterCases=23,successfulImages=20,resourceOrPrecisionFailures=3,invalidFrames=19,independentPixels=48233,
        independentlyVerifiedMultipleRootPixels=32,maxGrayError=max(x['maxGrayError'] for x in r['cases']),
        addressSanitizer=True,undefinedBehaviorSanitizer=True,oldCapabilityRejections=2,numericalFailureRecovery=True,
        unchangedDependencies=True,unchangedRustAndSchemas=True,markdownFiles=markdown,localLinks=links),
    reports={**{n:entry(root/(n+'.json')) for n in reports},'prepared':entry(root/'prepared/verification.json')},
    artifacts=artifacts,currentArtifacts=current,componentBuilds=builds,previousComponentBuilds=old['componentBuilds'],
    benchmark=b,size=dict(cppWasmBytes=2343781,previousCppWasmBytes=2327852,cppWasmIncrease=15929,rustWasmBytes=6398266,
        note='Raw standalone modules, not an installer or complete dependency closure.'),
    historicalSourceChangesExplicitlySkipped=sorted(skipped),verifiedArtifactRecords=records,
    limitations=['Rust generic field contracts and original PPTX page lowering still reject circle/path semantics; no complete product API integration claimed.',
        'General elliptic raster work remains too slow for interactive pages. Batch SIMD and further verified specializations remain open.',
        'Scalar error bounds are relative to wire binary32 inputs; source parameter conversion and pixel-color bounds are separate work.',
        'Independent reference samples general ellipses every fourth pixel, fully checks centered circles, and excludes the Office gamma case.',
        'No new WPS/PowerPoint observation in this stage; earlier near-equal-extent discrepancy remains unresolved.',
        'Advanced presentation content, Agent packages and Musterwork replacement remain incomplete.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),componentTriples=551,referencePixels=48233,rustTests=611,auditedRecords=records)))
