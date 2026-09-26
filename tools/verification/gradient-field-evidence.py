"""Seal native linear gradients, V9 frames, old behavior and open app differences."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
root=Path('.codex-work/gradient-field')
previous=Path('docs/reviews/evidence/2026-09-25-background-compositing-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-native-linear-gradient-verification.json')
def entry(path):
    b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='dde2ec031bcb477f0e1518b73d35ce5f72f73b65d9dacc02d2f42f4838e5f6a8'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''README.md
components/skia/mo_gradient.cpp
components/skia/mo_gradient.h
components/skia/mo_skia.cpp
components/skia/mo_skia.h
crates/mo-presentation-compile/src/source_image_layout/number.rs
crates/mo-presentation-compile/src/source_number.rs
crates/mo-presentation-compile/src/source_page.rs
crates/mo-presentation-compile/src/source_page/emit.rs
crates/mo-presentation-compile/src/source_page/paint.rs
crates/mo-presentation-compile/src/source_page/prepared.rs
crates/mo-raster/src/brush.rs
crates/mo-raster/src/compile.rs
crates/mo-raster/src/gradient.rs
crates/mo-raster/src/gradient_tests.rs
crates/mo-raster/src/image/compile.rs
crates/mo-raster/src/image/precision.rs
crates/mo-raster/src/lib.rs
crates/mo-skia-sys/src/ffi.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
docs/implementation/source-resource-page.md
packages/raster-component/src/index.ts
tools/components/build-skia.py
tools/mo-cli/src/raster.rs
tools/verification/skia-probe.cpp'''.splitlines())
for name in ['image-raster-request','image-scene-request','page-compile-response','path-raster-request','pptx-page-compile-response','scene-raster-request']:
    allowed.add('contracts/generated/'+name+'.schema.json');allowed.add('packages/contracts/src/generated/'+name+'.ts')
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''components/skia/gradient-plane.patch
components/skia/mo_gradient_plane.cpp
components/skia/mo_gradient_plane_stage.h
crates/mo-harfbuzz-sys/tests/source_gradient_page.rs
crates/mo-presentation-compile/src/source_page/gradient.rs
crates/mo-raster/src/gradient_plane.rs
crates/mo-raster/src/gradient_plane_tests.rs
crates/mo-raster/src/paint_matrix.rs
crates/mo-raster/src/paint_precision.rs
docs/implementation/native-linear-gradients.md
tools/test-support/source_gradient_page.rs'''.splitlines())
for name in ['checks.py','components.mjs','invalid.py','observe.py','parity.mjs','reference.py','regressions.mjs','source-parity.mjs','evidence.py']:
    added.add('tools/verification/gradient-field-'+name)
assert not added&set(prior)
sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:assert len(Path(p).read_text().splitlines())<=2000,p
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:assert entry(p)==prior[p]
commands=json.loads((root/'checks.json').read_text());assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for c in commands:
    text=Path(c['log']).read_text();assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),c['name']
    if c['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in text,c['name']
for name in ['native-rebuild','wasm-rebuild','asan-rebuild','parity','source-parity','components','regressions','text-regressions','reference','contracts','invalid-source','observations']:
    path=root/(name+'.log');text=path.read_text();assert text.strip() and not any(v in text for v in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(path)
logs['application-conversion']=entry(root/'libreoffice-convert.log')
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
assert len(tests)==584 and set(old['rustTestNames'])<=set(tests)
assert len(set(tests)-set(old['rustTestNames']))==7
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==80
contracts=json.loads((root/'contracts.log').read_text());assert (contracts['schemas'],contracts['negativeMutations'])==(80,9)
reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['parity','source-parity','components','reference','image-regressions','regressions','observations']}
p,s,c,r,i,t,o=[reports[n] for n in reports]
assert (p['pairedCalls'],len(p['cases']),p['oraclePixels'],p['maxChannelDifference'],len(p['negatives']))==(66,33,20736,1,6)
assert p['cli']['create'] and p['cli']['overwriteRefused']
assert (s['pairedCalls'],len(s['cases']),s['previousUnchanged'],sum(v['status']=='rendered' for v in s['cases']))==(107,107,86,87)
assert s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
assert len([v for v in s['cases'] if v['name'].startswith('invalid/')])==4
for v in s['cases']:
    if v['name'].startswith('invalid/'):assert v['status']=='error' and v['decodes']==v['rasters']==0
assert (c['triples'],c['legacyFrames'],len(c['negatives']),c['oldCapabilityRejections'])==(250,168,22,2)
assert c['addressSanitizer'] and c['undefinedBehaviorSanitizer'] and not c['leakSanitizer'] and c['noDiagnostics']
assert (len(r['sourceFiles']),r['officialXsdParts'],r['verifiedPixels'],len(r['cases']))==(17,119,1680000,14)
assert max(v['maximumChannelDifference'] for v in r['cases'])==1
assert json.loads((root/'invalid-source.json').read_text())['officialSlideParts']==4
assert i['pairedCalls']==len(i['cases'])==794
assert t['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
assert len(o['cases'])==16 and all(v['pixelsDifferent']>0 for v in o['cases'])
records=0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            actual=entry(value['path']);assert all(value[k]==actual[k] for k in actual),value['path'];records+=1
        for v in value.values():audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
audit(reports)
previous_source=json.loads(Path(s['previous']['path']).read_text())
for v in previous_source['cases']:
    now=next(x for x in s['cases'] if x['name']=='prior/'+v['name'])
    for key in ['request','source','fonts','response']:
        assert now[key]['sha256']==v[key]['sha256']
    if 'pixels' in v:assert now['pixels']['sha256']==v['pixels']['sha256']
builds={name:entry(root/'component'/(name+'-build.json')) for name in ['native','wasm','native-asan']}
lock=json.loads(Path('components/skia/lock.json').read_text())
for name,record in builds.items():
    build=json.loads(Path(record['path']).read_text());assert build['lock']==lock and build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
    assert any(v['path']=='components/skia/mo_gradient_plane.cpp' for v in build['componentSources'])
for record in old['componentBuilds'].values():audit(record);audit(json.loads(Path(record['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,value in old['artifacts'].items():
    if not key.startswith('native'):audit(value)
schema_checks=0
schemas={p.stem.removesuffix('.schema'):Draft202012Validator(json.loads(p.read_text())) for p in Path('contracts/generated').glob('*.schema.json')}
def validate(schema,record):
    global schema_checks
    schemas[schema].validate(json.loads(Path(record['path']).read_text()));schema_checks+=1
for v in p['cases']:
    validate('path-raster-request',v['request']);validate('path-raster-response',v['response'])
    if 'sceneRequest' in v:validate('scene-raster-request',v['sceneRequest']);validate('scene-raster-response',v['sceneResponse'])
for v in p['negatives']:
    validate('path-raster-response',v['response'])
    if v['name'] not in ['unknown-field','invalid-tile']:validate('path-raster-request',v['request'])
for v in s['cases']:
    validate('pptx-resource-page-raster-response',v['response'])
    if not v['name'].endswith(('/duplicate-json','/unknown-profile')):validate('pptx-resource-page-request',v['request'])
assert schema_checks==342,schema_checks
markdown=links=0
for name in sources:
    path=Path(name)
    if path.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:assert (path.parent/target).exists() or (path.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts={n:entry(v) for n,v in {
    'nativeWorker':'target/debug/mo-raster-worker','nativeCli':'target/debug/mo-cli','nativeTextWorker':'target/debug/mo-text-worker',
    'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
    'cppWasm':str(root/'component/mo-skia.wasm'),'typescriptRaster':str(root/'ts-raster/index.js'),
    'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
assert artifacts['rustWasm']['byteLength']==6206176 and artifacts['cppWasm']['byteLength']==2325263
report=dict(format='musteroffice.native-linear-gradient-verification/1',previousEvidence=entry(previous),
    scope='Native source linear gradients with independently mapped and tiled unit-plane fields. Existing source page/CLI/Native/WASM integration; path/stationary gradients and target Office/WPS compatibility remain open. Not complete PPT or Musterwork replacement acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),rustTestNames=tests,newTestNames=sorted(set(tests)-set(old['rustTestNames'])),
    checks=dict(rustTests=584,netNewRustTests=7,strictClippy=True,rustfmt=True,schemas=80,runtimeSchemaChecks=schema_checks,
                sharedPairedCalls=66,sourcePairedCalls=107,previousSourceUnchanged=86,previousImageClipCompositePairs=794,previousPageTextPairs=307,
                independentSourcePixels=1680000,independentPlanePixels=20736,maximumReferenceChannelDifference=1,
                originalPptxFiles=17,officialXsdParts=119,additionalNegativeSlideXsdParts=4,componentTriples=250,legacyFrames=168,
                addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,applicationFiles=16,thirdPartyLocksUnchanged=True,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,reports={n:entry(root/(n+'.json')) for n in reports},artifacts=artifacts,componentBuilds=builds,
    previousComponentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    normativeReferences=[dict(url='https://learn.microsoft.com/en-us/answers/questions/2265121/error-in-description-of-attribute-scaled-of-linear',claim='Microsoft employee confirmed corrected scaled normal on 2025-05-28: (h cos angle, w sin angle).'),dict(url='https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/f43f30df-c829-41f3-ba6f-52e9ac3b4e20',claim='MS-OI29500 2.1.1297: Office uses xy flip regardless of gradFill.flip; arrowhead bounds require additional rules.')],
    artifactSizeChange=dict(rustWasmBytes=6206176,previousRustWasmBytes=6145116,cppWasmBytes=2325263,previousCppWasmBytes=2323627,note='Uncompressed component artifacts only; no installer, product performance or RSS measurement.'),
    limitations=['Linear native fields only; path gradients and stationary object orientation still return explicit diagnostics before decode.',
                 'Straight sRGB interpolation is a draft source policy. Native/WASM parity and independent mathematical oracle do not establish target-application behavior.',
                 'LibreOffice 26.2.0.3 observations retain all differences, including scaled gradient, tiled and grouped interior differences. Office/WPS target acceptance remains unverified.',
                 'Independent source-pixel oracle covers integer aligned rectangles, not rotated/grouped antialias edges. Input uncertainty bounds are not shader or final pixel error proofs.',
                 'Complete advanced content, production Agent packaging and Musterwork replacement remain incomplete.'],verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),runtimeSchemaChecks=schema_checks,verifiedArtifacts=records)))
