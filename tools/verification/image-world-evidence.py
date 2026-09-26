"""Freeze Native world image compilation and precisely scoped renderer checks."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT=Path('.codex-work/image-world')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-image-precision-verification.json')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-image-world-verification.json')
def entry(path):
    b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256']=='7b6b665af73a95ff411729342adb4511c2af862d7e9aee1ff05232ebcca8809b'
old=json.loads(PREVIOUS.read_text())
changed={r['path'] for r in old['sourceFiles'] if entry(r['path'])!=r}
allowed={'crates/mo-presentation-compile/src/lib.rs','crates/mo-skia-sys/examples/source_image_layout.rs',
         'README.md','docs/README.md','docs/implementation/progress.md','docs/implementation/development.md'}
assert changed==allowed,(changed-allowed,allowed-changed)
added={'crates/mo-presentation-compile/src/source_image_paint.rs','docs/implementation/image-world-paint.md'}
for name in ['number','types','tests']:added.add('crates/mo-presentation-compile/src/source_image_paint/'+name+'.rs')
for name in ['fixtures.py','reference.py','parity.mjs','evidence.py']:added.add('tools/verification/image-world-'+name)
sources={r['path'] for r in old['sourceFiles']}|added
for name in sources:
    p=Path(name)
    if p.suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:assert len(p.read_text().splitlines())<=2000,name
for name in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:
    assert entry(name)==next(r for r in old['sourceFiles'] if r['path']==name)
logs=['tests','build','clippy','fmt','rust-wasm','schema-check','fixtures','reference','parity','pixels-reference']
for name in logs:
    value=(ROOT/(name+'.log')).read_text()
    assert not any(v in value for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),name
    if name in ['build','clippy','rust-wasm']:assert 'Finished' in value,name
text=(ROOT/'tests.log').read_text();assert 'Doc-tests mo_xml' in text
rust_tests=re.findall(r'^test (.+) \.\.\. ok$',text,re.M)
new_tests=[n for n in rust_tests if n not in old['rustTestNames']]
assert (len(rust_tests),len(new_tests))==(560,4)
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M))==78
reports={n:json.loads((ROOT/(n+'.json')).read_text()) for n in ['fixtures','reference','parity','pixels-reference']}
f=reports['fixtures'];assert (f['nativeRequests'],f['successfulRequests'],f['officialXsdParts'])==(28,26,56)
r=reports['reference'];assert (r['paints'],r['exactComparisons'],len(r['cases']))==(74,4474,26)
p=reports['parity'];assert (p['pairedCalls'],p['oldNativeLayoutRequests'])==(26,102)
assert reports['pixels-reference']['interiorPixels']==40171
records=0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            actual=entry(value['path']);assert all(actual[k]==value[k] for k in actual),value['path'];records+=1
        for v in value.values():audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
audit(reports)
clip_record=old['previousEvidence'];audit(clip_record)
clip=json.loads(Path(clip_record['path']).read_text())
for record in clip['componentBuilds'].values():
    audit(record);b=json.loads(Path(record['path']).read_text())
    for key in ['componentSources','artifacts','imageCodecs']:audit(b[key])
for key in ['previousReleaseArtifactsVerifiedUnchanged','standardInputs','codecInputLock']:audit(old[key])
for key in ['cppWasm','cppWasmGlue','typescriptAdapter']:audit(old['artifacts'][key])
checks=0
for c in p['cases']:
    for suffix,key in [('request','request'),('response','response')]:
        Draft202012Validator(json.loads(Path('contracts/generated/image-scene-'+suffix+'.schema.json').read_text())).validate(json.loads(Path(c[key]['path']).read_text()));checks+=1
markdown=links=0
for name in sources:
    pth=Path(name)
    if pth.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',pth.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        destination=unquote(target.split('#')[0].split('?')[0])
        if destination:
            resolved=pth.parent/destination
            assert resolved.exists() or resolved.resolve()==OUTPUT.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
report=dict(format='musteroffice.image-world-verification/1',previousEvidence=entry(PREVIOUS),
    scope='Native source image world-paint library and fill-clip output, followed by shared Native/WASM raster verification. No complete source page or WASM product operation invokes the new compiler yet.',
    sourceFiles=[entry(n) for n in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=rust_tests,newTestNames=new_tests,
    checks=dict(rustTests=560,newRustTests=4,strictClippy=True,rustfmt=True,existingSchemas=78,
        schemasAndGeneratedTypeScriptUnchanged=True,runtimeSchemaChecks=checks,
        nativeSourceRequests=28,successfulSourceRequests=26,orientationPrerequisiteFailures=2,
        nativeWorldPaints=74,exactRationalComparisons=4474,sharedRendererPairedCalls=26,
        independentInteriorPixels=40171,oldNativeLayoutRegressions=102,
        fixedComponentsAndThirdPartyResolutionUnchanged=True,markdown=markdown,localLinks=links),
    reports={n:entry(ROOT/(n+'.json')) for n in reports},validationLogs={n:entry(ROOT/(n+'.log')) for n in logs},
    componentBuilds=clip['componentBuilds'],
    artifacts={n:entry(path) for n,path in {
        'nativeWorldHost':'target/debug/examples/source_image_layout','nativeWorker':'target/debug/mo-raster-worker',
        'nativeCli':'target/debug/mo-cli','nativeTextWorker':'target/debug/mo-text-worker',
        'rustWasm':str(ROOT/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(ROOT/'wasm-node/mo_wasm.js'),
        'cppWasm':'.codex-work/clips/component/mo-skia.wasm','cppWasmGlue':'.codex-work/clips/component/mo-skia.mjs',
        'typescriptAdapter':'.codex-work/clips/ts-raster/index.js'}.items()},
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],
    limitations=['New world compiler is Native library tested and cross-target built; actual WASM calls test shared rendering of its output, not the new compiler itself.',
        'Interior nearest-sampling pixel reference excludes AA/clip/texel boundaries and is conditional on the separately verified native placement/local layout.',
        'Stationary shape-image orientation, ordered production source page/resources, advanced contents and Agent/Musterwork acceptance remain incomplete.',
        'No Office/WPS or new sanitizer/performance/installer acceptance in this stage.'])
if '--seal' in sys.argv:
    assert not OUTPUT.exists(),'evidence is immutable after sealing'
    OUTPUT.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
else:assert json.loads(OUTPUT.read_text())==report,'completed evidence differs from current closure'
print(json.dumps(dict(evidence=entry(OUTPUT),sources=len(sources),auditedRecords=records)))
