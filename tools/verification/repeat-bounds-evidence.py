"""Seal bounded/indefinite repetitions, independent references and unchanged historical results."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/repeat-bounds');previous=Path('docs/reviews/evidence/2026-09-26-time-transform-verification.json');output=Path('docs/reviews/evidence/2026-09-26-repeat-bounds-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='aa1d236dfd85bde0ee074bd8a5bfed6dd9a7d5b14f0a0244db676695508fcba3'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
contracts/README.md
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
crates/mo-kernel-api/tests/timeline.rs
crates/mo-pptx/src/timing.rs
crates/mo-pptx/src/timing/read.rs
crates/mo-pptx/tests/timing.rs
crates/mo-presentation-edit/tests/timing.rs
crates/mo-timeline/src/evaluate.rs
crates/mo-timeline/src/clock.rs
crates/mo-timeline/src/model.rs
crates/mo-timeline/src/tree/sample.rs
crates/mo-timeline/tests/time_transform.rs
crates/mo-timeline/tests/timeline.rs
crates/mo-timeline/tests/tree.rs'''.splitlines())
for stem in ['document','kernel-request','kernel-response','page-placement-request','page-render-request','playback-page-request','playback-session-request','pptx-export-request','pptx-timing-response','timeline-evaluate-request','transaction']:
 allowed|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
added=set(['crates/mo-timeline/tests/repeat_bounds.rs','docs/implementation/repeat-bounds.md'])
for name in ['workspace.py','fixtures.py','parity.mjs','replay.py','native-reference.py','preview.py','evidence.py']:added.add('tools/verification/repeat-bounds-'+name)
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed);assert not added&set(prior)
sources=set(prior)|added;assert len(sources)==1464
for p in sources:
 if Path(p).suffix in ['.rs','.ts','.py','.mjs','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text());assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen'];assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==711;assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==96
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==96
paths={key:root/(name+'.json')for key,name in [('authorFixtures','author-fixtures'),('sourceFixtures','source-fixtures'),('product','product'),('regression','regression'),('nativeReference','native-reference'),('preview','preview')]};reports={key:json.loads(path.read_text())for key,path in paths.items()}
a=reports['authorFixtures'];s=reports['sourceFixtures'];p=reports['product'];r=reports['regression'];n=reports['nativeReference']
assert len(a['cases'])==33 and sum(len(c['samples'])for c in a['cases'])==441
assert len(s['cases'])==96 and s['officialXsdParts']==624
assert p['owners']==13 and p['pairedCalls']==len(p['cases'])==968 and p['evaluated']==474 and p['authorFrames']==46 and p['sourceFrames']==96 and p['staticControls']==126
assert len(p['exports'])==3 and n['officialXsdParts']==9 and n['checkedNativeEntries']==10
cases={c['name']:c for c in p['cases']};rendered=zero_source=0;static_pixels=0
for c in p['cases']:
 response=json.loads(Path(c['response']['path']).read_text());pixels=Path(c['pixels']['path']).read_bytes()
 if response['status']=='rendered':
  rendered+=1;assert c['rasters']==1
  info=response['info'].get('page',response['info']);page=response['info']
  while 'scene'not in page:page=page['page']
  assert hashlib.sha256(pixels).hexdigest()==page['scene']['raster']['sha256']
  if c['name'].endswith('/static'):static_pixels+=len(pixels)//4
  if c['operation']=='owner-render':
   assert c['decodes']==c['shapes']==0
   if 'playback'in response['info']:
    zero_source+=1;assert info['gatherCopyBytes']==0;assert all(info['textWork'][key]==0 for key in ['componentCalls','fontUploadBytes','requestWords'])
 else:assert not pixels and c['rasters']==0
assert rendered==411 and zero_source==96,(rendered,zero_source)
for c in s['cases']:
 one=cases[c['name']+'/one-shot'];kept=cases[c['name']+'/retained'];assert one['pixels']['sha256']==kept['pixels']['sha256'];assert one['frame']['sha256']==kept['frame']['sha256']
 state=json.loads(Path(one['response']['path']).read_text())['info']['playback']['evaluated']['state'];assert state['rotations']==c['expectedRotations']
 if 'staticControl'in c:assert cases[c['name']+'/static']['pixels']['sha256']==one['pixels']['sha256']
for c in a['cases']:
 for i,sample in enumerate(c['samples']):
  state=json.loads(Path(cases[f'{c["name"]}/{i}']['response']['path']).read_text())['frame']['state'];assert state['rotations']==sample['rotations']
  if 'expectedNodes'in sample:
   for node in state['nodes']:
    expected=sample['expectedNodes'][node['node']]
    for key in ['start','end','iteration','progress']:assert node[key]==expected.get(key)
for name in ['evaluate','one-shot','retained']:
 c=cases['unbounded-reverse/'+name];assert c['rasters']==0 and c['pixels']['byteLength']==0
 assert json.loads(Path(c['response']['path']).read_text())['status']=='error'
assert json.loads(Path(cases['unbounded-reverse/before-start']['response']['path']).read_text())['status']=='rendered'
assert [v['name']for v in r['cases']]==['replay','author-replay','source-replay','parity','hierarchy','clock'];assert [v['pairedCalls']for v in r['cases']]==[355,111,70,135,562,877]
for run in r['cases']:
 current=json.loads(Path(run['current']['path']).read_text());historical=json.loads(Path(run['previous']['path']).read_text());assert len(current['cases'])==len(historical['cases'])
 for actual,expected in zip(current['cases'],historical['cases']):
  assert actual['name']==expected['name']
  for key in ['request','response','pixels','frame']:
   if key in expected:assert actual[key]['sha256']==expected[key]['sha256'],(run['name'],actual['name'],key)
 if run['name']=='replay':assert current['timelineCalls']==246 and current['timelineCases']==historical['timelineCases']
 if run['name']in ['hierarchy','clock']:
  assert len(current['exports'])==len(historical['exports'])==2
  for actual,expected in zip(current['exports'],historical['exports']):assert actual['output']['sha256']==expected['output']['sha256']
for name in ['fixtures','workspace-run','parity','native-reference','regression-run','replay-regression','author-replay-regression','source-replay-regression','parity-regression','hierarchy-regression','clock-regression','preview']:
 assert not any(s in (root/(name+'.log')).read_text()for s in ['error:','FAILED','Traceback','AssertionError','TypeError']),name
records=0;historical_changes=set();mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(v,historical=False):
 global records
 if isinstance(v,dict):
  if {'path','byteLength','sha256'}<=v.keys():
   if historical and v['path']in allowed|mutable:historical_changes.add(v['path'])
   else:
    actual=entry(v['path']);assert all(v[k]==value for k,value in actual.items()),v['path'];records+=1
  for value in v.values():audit(value,historical)
 elif isinstance(v,list):
  for value in v:audit(value,historical)
audit(reports);audit(old,True)
for v in old['reports'].values():audit(json.loads(Path(v['path']).read_text()),True)
for run in r['cases']:audit(json.loads(Path(run['current']['path']).read_text()))
for v in old['componentBuilds'].values():
 build=json.loads(Path(v['path']).read_text());assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
 for key in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(build[key])
current={key:entry(path)for key,path in {'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker','rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js','cppWasm':'.codex-work/gradient-coordinates/component/mo-skia.wasm'}.items()}
for key in ['typescriptRaster','cppWasm']:assert current[key]==old['currentArtifacts'][key]
for path in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(path)==prior[path]
assert current['rustWasm']['byteLength']==7793362
markdown=links=0
for name in sources:
 path=Path(name)
 if path.suffix!='.md':continue
 markdown+=1
 for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
  if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target)or target.startswith('#'):continue
  target=unquote(target.split('#')[0].split('?')[0])
  if target:assert(path.parent/target).exists()or(path.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[entry(path)for path in sorted(root.rglob('*'))if path.is_file()and path.relative_to(root).parts[0]!='attempts'and path.name not in ['plan.md','next.md','seal.log','seal-check.log']]
report=dict(format='musteroffice.repeat-bounds-verification/1',previousEvidence=entry(previous),scope='Explicit finite/indefinite repeat count and repeat duration bounds in shared flat/tree clocks, resolved ancestor endpoints for indefinite reversal, native PPTX read/write and actual retained/one-shot Native/WASM pages. Not full slideshow or product acceptance.',sourceFiles=[entry(path)for path in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),newRustTests=8,schemas=96,typescript=True,strictClippy=True,rustfmt=True,authorGraphs=33,independentAuthorSamples=441,evaluatedPublicFrames=474,newPairedCalls=968,owners=13,authorFrames=46,sourceFrames=96,renderedCalls=rendered,staticControls=126,staticControlPixels=static_pixels,zeroSampleSourcePreparationFrames=96,nativeExports=3,independentNativeEntries=10,officialXsdParts=633,staticReplayCalls=355,timelineReplayCalls=246,authorReplayCalls=111,sourceReplayCalls=70,retainedSourceReplayCalls=135,hierarchyReplayCalls=562,clockReplayCalls=877,oldRecordedBytesUnchanged=True,undefinedReverseErrors=3,ownerRecoveryFrames=1,officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),reports={key:entry(path)for key,path in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw modules, not complete font/media/runtime closure or desktop installer.'),
 limitations=['All full phase-one obligations remain: this stage adds repeat bounds to the once-activation rotation profile.',
 'Container time manipulation and repetition/restarts, full condition/end/default semantics and seq navigation/concurrency remain unimplemented and explicitly diagnosed.',
 'Import projects supported native semantics; it does not implement arbitrary imported time-tree editing or prove target application semantics.',
 'Native missing repeatCount uses the XSD default 1000; repeatDur/minimum-bound computation is an explicit draft profile pending actual PowerPoint/WPS playback verification.',
 'Unbounded negative repetition without a resolved ancestor or declared endpoint yields a semantic sample error with no image; full event-driven endpoints remain required.',
 'Retained resources remain scoped to the existing rotation/resource-page contract; text/geometry/paint-changing effects need new invalidation evidence.',
 'No new FPS, throughput, RSS, cold start or full installer measurements; prior gradient performance regression remains open.',
 'Synthetic fonts and controlled resources are correctness fixtures, not production visual-quality or Office/WPS acceptance.',
 'Full effects/subtargets, transitions/Morph/media and advanced editable objects, production Agent packages and actual Musterwork Artifact/history migration remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
