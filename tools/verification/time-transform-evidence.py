"""Seal finite behavior time transforms, independent references and unchanged historical results."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/time-transform');previous=Path('docs/reviews/evidence/2026-09-26-timing-tree-verification.json');output=Path('docs/reviews/evidence/2026-09-26-time-transform-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='afa9c20766b865ac75fa83d707960b109b7073e5ea402a9c4ca838c9989d4d1a'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
contracts/README.md
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
crates/mo-kernel-api/tests/timeline.rs
crates/mo-pptx/src/source/drawingml/color.rs
crates/mo-pptx/src/timing.rs
crates/mo-pptx/src/timing/read.rs
crates/mo-pptx/tests/timing.rs
crates/mo-presentation-edit/tests/timing.rs
crates/mo-timeline/src/evaluate.rs
crates/mo-timeline/src/lib.rs
crates/mo-timeline/src/model.rs
crates/mo-timeline/src/plan.rs
crates/mo-timeline/src/tree.rs
crates/mo-timeline/src/tree/sample.rs
crates/mo-timeline/tests/timeline.rs
crates/mo-timeline/tests/tree.rs'''.splitlines())
for stem in ['document','kernel-request','kernel-response','page-placement-request','page-render-request','playback-page-request','playback-session-request','pptx-export-request','pptx-timing-response','timeline-evaluate-request','transaction']:
 allowed|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
added=set('''crates/mo-timeline/src/clock.rs
crates/mo-timeline/tests/time_transform.rs
crates/mo-pptx/src/timing/read/time_transform.rs
docs/implementation/time-transforms.md'''.splitlines())
for name in ['workspace.py','fixtures.py','parity.mjs','replay.py','native-reference.py','preview.py','evidence.py']:added.add('tools/verification/time-transform-'+name)
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed);assert not added&set(prior)
sources=set(prior)|added;assert len(sources)==1455
for p in sources:
 if Path(p).suffix in ['.rs','.ts','.py','.mjs','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text());assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen'];assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==703;assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==96
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==96
paths={key:root/(name+'.json')for key,name in [('authorFixtures','author-fixtures'),('sourceFixtures','source-fixtures'),('product','product'),('regression','regression'),('nativeReference','native-reference'),('preview','preview')]};reports={key:json.loads(path.read_text())for key,path in paths.items()}
a=reports['authorFixtures'];s=reports['sourceFixtures'];p=reports['product'];r=reports['regression'];n=reports['nativeReference']
assert len(a['cases'])==33 and sum(len(c['samples'])for c in a['cases'])==471
assert len(s['cases'])==60 and s['officialXsdParts']==390
assert p['owners']==9 and p['pairedCalls']==len(p['cases'])==877 and p['evaluated']==494 and p['authorFrames']==51 and p['sourceFrames']==60 and p['staticControls']==95
assert len(p['exports'])==2 and n['officialXsdParts']==6 and n['checkedNativeEntries']==7
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
assert rendered==317 and zero_source==60,(rendered,zero_source)
for c in s['cases']:
 one=cases[c['name']+'/one-shot'];kept=cases[c['name']+'/retained'];assert one['pixels']['sha256']==kept['pixels']['sha256'];assert one['frame']['sha256']==kept['frame']['sha256']
 state=json.loads(Path(one['response']['path']).read_text())['info']['playback']['evaluated']['state'];assert state['rotations']==c['expectedRotations']
 if 'staticControl'in c:assert cases[c['name']+'/static']['pixels']['sha256']==one['pixels']['sha256']
for c in a['cases']:
 for i,sample in enumerate(c['samples']):
  state=json.loads(Path(cases[f'{c["name"]}/{i}']['response']['path']).read_text())['frame']['state'];assert state['rotations']==sample['rotations']
assert [v['name']for v in r['cases']]==['replay','author-replay','source-replay','parity','hierarchy'];assert [v['pairedCalls']for v in r['cases']]==[355,111,70,135,562]
for run in r['cases']:
 current=json.loads(Path(run['current']['path']).read_text());historical=json.loads(Path(run['previous']['path']).read_text());assert len(current['cases'])==len(historical['cases'])
 for actual,expected in zip(current['cases'],historical['cases']):
  assert actual['name']==expected['name']
  for key in ['request','response','pixels','frame']:
   if key in expected:assert actual[key]['sha256']==expected[key]['sha256'],(run['name'],actual['name'],key)
 if run['name']=='replay':assert current['timelineCalls']==246 and current['timelineCases']==historical['timelineCases']
 if run['name']=='hierarchy':
  assert len(current['exports'])==len(historical['exports'])==2
  for actual,expected in zip(current['exports'],historical['exports']):assert actual['output']['sha256']==expected['output']['sha256']
for name in ['fixtures','workspace-run','parity','native-reference','regression-run','replay-regression','author-replay-regression','source-replay-regression','parity-regression','hierarchy-regression','preview']:
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
assert current['rustWasm']['byteLength']==7778080
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
report=dict(format='musteroffice.time-transform-verification/1',previousEvidence=entry(previous),scope='Exact finite leaf speed/reversal, autoReverse and integrated acceleration/deceleration shared by flat/tree scheduling and sampling; native PPTX read/write and actual retained/one-shot Native/WASM pages. Not full slideshow or product acceptance.',sourceFiles=[entry(path)for path in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),newRustTests=8,schemas=96,typescript=True,strictClippy=True,rustfmt=True,authorGraphs=33,independentAuthorSamples=471,evaluatedPublicFrames=494,newPairedCalls=877,owners=9,authorFrames=51,sourceFrames=60,renderedCalls=rendered,staticControls=95,staticControlPixels=static_pixels,zeroSampleSourcePreparationFrames=60,nativeExports=2,independentNativeEntries=7,officialXsdParts=396,staticReplayCalls=355,timelineReplayCalls=246,authorReplayCalls=111,sourceReplayCalls=70,retainedSourceReplayCalls=135,hierarchyReplayCalls=562,oldRecordedBytesUnchanged=True,officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),reports={key:entry(path)for key,path in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw modules, not complete font/media/runtime closure or desktop installer.'),
 limitations=['All full phase-one obligations remain: this stage adds finite behavior time manipulation to the once-activation rotation profile.',
 'Container time manipulation, container repeats/restarts, repeatDur/indefinite repeats, additional native conditions/defaults and seq navigation/concurrency remain unimplemented and explicitly diagnosed.',
 'Import projects supported native semantics; it does not implement arbitrary imported time-tree editing or prove target application semantics.',
 'Native percentage projection rejects unrepresentable values; source lexical forms remain preserved, and strict invalid easing diagnosis differs from general SMIL fallback.',
 'Retained resources remain scoped to the existing rotation/resource-page contract; text/geometry/paint-changing effects need new invalidation evidence.',
 'No new FPS, throughput, RSS, cold start or full installer measurements; prior gradient performance regression remains open.',
 'Synthetic fonts and controlled resources are correctness fixtures, not production visual-quality or Office/WPS acceptance.',
 'Full effects/subtargets, transitions/Morph/media and advanced editable objects, production Agent packages and actual Musterwork Artifact/history migration remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
