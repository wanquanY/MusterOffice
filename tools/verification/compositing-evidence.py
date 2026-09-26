"""Seal prefix compositing, source policy, actual regressions and open app differences."""
import hashlib, json, re, subprocess, sys
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
root=Path('.codex-work/compositing')
previous=Path('docs/reviews/evidence/2026-09-25-group-image-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-background-compositing-verification.json')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='3ec0d28648587c1f8e8a62aed0eb220babc57c15caba0de3f30e861ead6af81b'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
docs/implementation/source-resource-page.md
docs/implementation/group-image-inheritance.md
components/skia/mo_skia.cpp
components/skia/mo_skia.h
crates/mo-presentation-compile/src/path_scene.rs
crates/mo-presentation-compile/src/source_page/emit.rs
crates/mo-presentation-compile/src/source_page/paint.rs
crates/mo-presentation-compile/tests/source_resource_page.rs
crates/mo-skia-sys/src/ffi.rs
packages/raster-component/src/index.ts
tools/components/build-skia.py
tools/mo-cli/src/raster.rs
tools/verification/skia-probe.cpp'''.splitlines())
for n in ['brush','clip_tests','compile','gradient_tests','image_tests','lib','stroke_tests','tests','types']:
    allowed.add('crates/mo-raster/src/'+n+'.rs')
for n in ['compile','tests','types']:allowed.add('crates/mo-render/src/'+n+'.rs')
stems=['image-raster-request','image-raster-response','image-scene-request','image-scene-response',
       'page-compile-response','page-raster-response','path-raster-request','path-raster-response',
       'pptx-page-compile-response','pptx-page-raster-response','pptx-resource-page-raster-response',
       'pptx-text-page-raster-response','scene-raster-request','scene-raster-response']
for stem in stems:
    allowed.add('contracts/generated/'+stem+'.schema.json')
    allowed.add('packages/contracts/src/generated/'+stem+'.ts')
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''components/skia/mo_composite.h
crates/mo-raster/src/composite.rs
crates/mo-raster/src/composite_tests.rs
crates/mo-harfbuzz-sys/tests/source_background_page.rs
tools/test-support/source_background_page.rs
docs/implementation/background-compositing.md'''.splitlines())
for name in ['checks.py','fixtures.mjs','parity.mjs','source-parity.mjs','components.mjs','regressions.mjs','reference.py','observe.py','evidence.py']:
    added.add('tools/verification/compositing-'+name)
assert not added&set(prior)
sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(Path(p).read_text().splitlines())<=2000,p
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:
    assert entry(p)==prior[p]
commands=json.loads((root/'checks.json').read_text());assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for c in commands:
    value=Path(c['log']).read_text();assert not any(t in value for t in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),c['name']
    if c['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in value,c['name']
for name in ['build-native','build-wasm','build-asan','contracts','reference','parity','source-parity','components','image-regressions','regressions','observations']:
    path=root/(name+'.log');value=path.read_text();assert value.strip() and not any(t in value for t in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(path)
logs['application-convert']=entry(root/'libreoffice-convert.log')
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=[n for n in tests if n not in old['rustTestNames']]
removed=set(old['rustTestNames'])-set(tests)
assert len(tests)==577 and len(new_tests)==5
assert removed=={'redirected_image_fill_requires_its_own_paint_space_before_decode'}
assert 'background_image_window_reuses_one_decoded_background' in new_tests
assert 'translucent_background_window_is_not_an_ordinary_source_over_fill' in tests
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==80
contracts=json.loads((root/'contracts.log').read_text());assert (contracts['schemas'],contracts['negativeMutations'])==(80,9)
reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['parity','source-parity','components','image-regressions','regressions','reference','observations']}
p,s,c,i,g,r,o=[reports[n] for n in reports]
assert (p['pairedCalls'],len(p['cases']),len(p['controls']),len(p['negatives']),p['rectanglePixels'],p['edgePixels'])==(97,34,24,5,6144,12288)
assert len(p['cliChecks'])==2 and p['cliRejectionOutputAbsent']
assert (s['pairedCalls'],len(s['cases']),sum(v['status']=='rendered' for v in s['cases']),s['previousUnchanged'])==(86,86,70,54)
assert s['previousNewlySupported']==['previous/background-replay','previous/background-alpha']
assert s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
assert (c['triples'],c['legacyFrames'],len(c['negatives']),c['oldCapabilityRejections'])==(190,100,20,2)
assert c['addressSanitizer'] and c['undefinedBehaviorSanitizer'] and not c['leakSanitizer'] and c['noDiagnostics']
assert c['heapControl']==dict(capture=False,outputBytes=67108864)
assert c['heapFailure']==dict(capture=True,invalidated=True,partialPixelsPublished=False)
assert i['pairedCalls']==len(i['cases'])==726
assert g['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
assert (r['officialXsdParts'],len(r['sourceFiles']),len(r['cases']),r['verifiedPixels'])==(210,30,8,960000)
assert len(o['cases'])==10
assert {v['name']:v['constantNeighbourhoodDifferences'] for v in o['cases'] if v['constantNeighbourhoodDifferences']}=={
    'alpha-clear':36556,'alpha-opaque':36556,'clipped-image':17860,'image-clear':6958,
    'multiple-windows':14072,'rotated-ellipse':2771,'tiled-image':10197}
records=0
def audit(v):
    global records
    if isinstance(v,dict):
        if {'path','byteLength','sha256'}<=v.keys():
            actual=entry(v['path']);assert all(actual[k]==v[k] for k in actual),v['path'];records+=1
        for x in v.values():audit(x)
    elif isinstance(v,list):
        for x in v:audit(x)
audit(reports)
previous_source=json.loads(Path(s['previous']['path']).read_text())
for v in previous_source['cases']:
    now=next(x for x in s['cases'] if x['name']=='prior/'+v['name'])
    for key in ['request','source','fonts']:assert v[key]['sha256']==now[key]['sha256']
    if v['name'] in s['previousNewlySupported']:assert v['status']=='error' and now['status']=='rendered'
    else:
        assert v['response']['sha256']==now['response']['sha256']
        if 'pixels' in v:assert v['pixels']['sha256']==now['pixels']['sha256']
builds={name:entry(root/'component'/(name+'-build.json')) for name in ['native','wasm','native-asan']}
lock=json.loads(Path('components/skia/lock.json').read_text())
for name,record in builds.items():
    build=json.loads(Path(record['path']).read_text())
    assert build['lock']==lock and build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
    assert any(v['path']=='components/skia/mo_composite.h' for v in build['componentSources'])
for record in old['componentBuilds'].values():
    audit(record);audit(json.loads(Path(record['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,value in old['artifacts'].items():
    if key not in ['nativeWorker','nativeCli']:audit(value)
schema_checks=0
schemas={p.stem.removesuffix('.schema'):Draft202012Validator(json.loads(p.read_text())) for p in Path('contracts/generated').glob('*.schema.json')}
def validate(schema,record):
    global schema_checks
    schemas[schema].validate(json.loads(Path(record['path']).read_text()));schema_checks+=1
for v in p['cases']:
    stem='image-raster' if v['images'] else 'path-raster';scene='image-scene' if v['images'] else 'scene-raster'
    validate(stem+'-request',v['request']);validate(stem+'-response',v['response'])
    validate(scene+'-request',v['sceneRequest']);validate(scene+'-response',v['sceneResponse'])
for v in p['controls']:
    q=json.loads(Path(v['request']['path']).read_text());stem='image-raster' if 'images' in q else 'path-raster'
    validate(stem+'-request',v['request']);validate(stem+'-response',v['response'])
for v in p['negatives']:
    validate('path-raster-response',v['response'])
    if v['name'] not in ['unknown-blend','snapshot-unknown-field']:validate('path-raster-request',v['request'])
for v in s['cases']:
    validate('pptx-resource-page-raster-response',v['response'])
    if not v['name'].endswith(('/duplicate-json','/unknown-profile')):validate('pptx-resource-page-request',v['request'])
assert schema_checks==362,schema_checks
markdown=links=0
for name in sources:
    path=Path(name)
    if path.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            assert (path.parent/target).exists() or (path.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts={n:entry(v) for n,v in {
    'nativeWorker':'target/debug/mo-raster-worker','nativeCli':'target/debug/mo-cli','nativeTextWorker':'target/debug/mo-text-worker',
    'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
    'cppWasm':str(root/'component/mo-skia.wasm'),'typescriptRaster':str(root/'ts-raster/index.js'),
    'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
resolved={n:entry(root/n) for n in ['build-native-initial.log','build-wasm-initial.log','background-tests-initial.log','initial-wire/parity.log','parity-coordinate-check-initial.log','regressions-missing-worker-initial.log']}
report=dict(format='musteroffice.background-compositing-verification/1',previousEvidence=entry(previous),
    scope='Generic immutable draw-prefix capture and Source/SourceOver composition. Source-page restoration is a draft policy; alpha/tile application differences remain unresolved. Not full PPT or Musterwork replacement acceptance.',
    sourceFiles=[entry(v) for v in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new_tests,replacedTestNames=sorted(removed),
    checks=dict(rustTests=577,netNewRustTests=4,strictClippy=True,rustfmt=True,schemas=80,generatedTypeScript=True,runtimeSchemaChecks=schema_checks,
                sharedPairedCalls=97,sourcePairedCalls=86,previousImageClipPairs=726,previousPageTextPairs=307,
                directComponentTriples=190,oldV7Frames=100,addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,
                realWasmHeapFailure=True,partialOutputPublished=False,originalPptxFiles=30,officialXsdParts=210,
                independentSourcePixels=960000,rectangleRestorePixels=6144,fullAntialiasControlPixels=12288,
                sourceRegressionUnchanged=54,sourceRegressionNewlySupported=2,applicationFiles=10,applicationInteriorDifferenceCases=7,
                thirdPartyLocksUnchanged=True,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,resolvedDevelopmentFailures=resolved,
    reports={n:entry(root/(n+'.json')) for n in reports},artifacts=artifacts,componentBuilds=builds,
    previousComponentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    artifactSizeChange=dict(rustWasmBytes=6145116,previousRustWasmBytes=6131147,cppWasmBytes=2323627,previousCppWasmBytes=2322200,note='Uncompressed individual WASM artifacts only. No whole-product, RSS, performance or installer-size measurement.'),
    limitations=['Draft source restoration interprets the background surface as its completed pixels. Microsoft documentation establishes background-region matching but does not settle every alpha/tile detail.',
                 'LibreOffice source alpha, transparent-image and tile behavior differs. Retain all pixel and interior differences; target Office/WPS evidence is still required.',
                 'Earlier group/root/layout/flip differences remain open. No permission bypass or target-app acceptance claim.',
                 'Snapshot bytes are a separate bounded copy budget, not RSS. Generic Skia allocation can trap; host must terminate invalidated workers.',
                 'Complete native paints/text/effects, advanced content and production Agent/Musterwork integration remain unfinished.'],
    verifiedArtifactRecords=records)
assert artifacts['rustWasm']['byteLength']==6145116 and artifacts['cppWasm']['byteLength']==2323627
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence'
    output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),schemaChecks=schema_checks,verifiedArtifacts=records),ensure_ascii=False))
