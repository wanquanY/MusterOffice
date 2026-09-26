"""Bind retained interval semantics, owner work, frozen outputs and measured costs."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/retained-timing');previous=Path('docs/reviews/evidence/2026-09-26-container-lifecycle-verification.json');output=Path('docs/reviews/evidence/2026-09-26-retained-timing-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='6df136642857288790bd452214f15316305e33d4dddb42d3a850d06722f8ceb1'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
contracts/README.md
crates/mo-kernel-api/src/playback.rs
crates/mo-kernel-api/src/playback_session.rs
crates/mo-kernel-api/src/pptx_playback.rs
crates/mo-kernel-api/src/pptx_playback_session.rs
crates/mo-kernel-api/tests/playback_session.rs
crates/mo-kernel-api/tests/pptx_playback_session.rs
crates/mo-presentation-compile/src/playback.rs
crates/mo-presentation-compile/src/source_playback.rs
crates/mo-presentation-compile/tests/playback.rs
crates/mo-presentation-compile/tests/source_playback.rs
crates/mo-timeline/src/evaluate.rs
crates/mo-timeline/src/lib.rs
crates/mo-timeline/src/tree.rs
crates/mo-timeline/src/tree/sample.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md'''.splitlines())
for stem in ['playback-session-request','playback-session-response','pptx-playback-session-request','pptx-playback-session-response']:
 allowed|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
added={'crates/mo-timeline/src/events.rs','crates/mo-timeline/src/sampler.rs','crates/mo-timeline/tests/retained.rs','docs/implementation/retained-timing.md'}
added|={'tools/verification/retained-timing-'+s for s in ['workspace.py','parity.mjs','replay.py','benchmark.mjs','evidence.py']}
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed);assert not added&set(prior)
sources=set(prior)|added
for name in sources:
 p=Path(name)
 if p.suffix in ['.rs','.ts','.py','.mjs','.cpp','.h']:assert len(p.read_text().splitlines())<=2000,name
checks=json.loads((root/'workspace.json').read_text());assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen'];assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==738;assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==96
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==96
reports={name:json.loads((root/(name+'.json')).read_text())for name in ['product','regression','benchmark']}
p=reports['product'];r=reports['regression'];b=reports['benchmark']
assert p['owners']==10 and p['authorFrames']==76 and p['sourceFrames']==108
assert len(p['cases'])==p['pairedCalls']==654;assert sum(c['operation']=='owner-inspectTiming'and json.loads(Path(c['response']['path']).read_text())['status']=='timingInspected'for c in p['cases'])==p['inspected']
rendered=retained=inspected=0
for c in p['cases']:
 response=json.loads(Path(c['response']['path']).read_text());pixels=Path(c['pixels']['path']).read_bytes()
 if response['status']=='rendered':
  rendered+=1;assert c['rasters']==1
  page=response['info']
  while 'scene'not in page:page=page['page']
  assert hashlib.sha256(pixels).hexdigest()==page['scene']['raster']['sha256']
  if c['operation']=='owner-render':
   retained+=1;assert c['decodes']==c['shapes']==0
   if 'playback'in response['info']:
    assert response['info']['page']['gatherCopyBytes']==0
    assert all(response['info']['page']['textWork'][key]==0 for key in ['componentCalls','fontUploadBytes','requestWords'])
 elif response['status']=='timingInspected':
  inspected+=1;assert c['rasters']==c['decodes']==c['shapes']==0
  info=response['info'];assert info['binding']==json.loads(Path(c['request']['path']).read_text())['binding']
  for k in ['schedulesBuilt','schedulesReused','retainedIntervals','retainedEvents']:
   value=info['sampler'][k];assert isinstance(value,str)and str(int(value))==value and 0<=int(value)<=2**64-1
  assert not pixels
 else:assert not pixels and c['rasters']==0
assert retained==p['authorFrames']+p['sourceFrames'] and inspected==p['inspected']
assert [c['pairedCalls']for c in r['cases']]==[355,111,70,135,562,877,968,874,904]
for run in r['cases']:
 actual=json.loads(Path(run['current']['path']).read_text());expected=json.loads(Path(run['previous']['path']).read_text());assert len(actual['cases'])==len(expected['cases'])
 for a,e in zip(actual['cases'],expected['cases']):
  assert a['name']==e['name']
  for key in ['request','response','pixels','frame']:
   if key in e:assert a[key]['sha256']==e[key]['sha256'],(run['name'],a['name'],key)
 if run['name']=='replay':assert actual['timelineCalls']==246 and actual['timelineCases']==expected['timelineCases']
 for a,e in zip(actual.get('exports',[]),expected.get('exports',[])):assert a['output']['sha256']==e['output']['sha256']
assert len(b['cases'])==6
for c in b['cases']:
 assert c['rounds']==21 and c['warmups']==3
 for key,measure in [('oldMs','before'),('newMs','after')]:
  values=sorted(c[key]);assert len(values)==21 and values[10]==c[measure]['median']and values[19]==c[measure]['p95']
 assert c['ratio']==c['after']['median']/c['before']['median']
 if c['cache']=='same-prefix':assert c['timing']['schedulesBuilt']=='1'
 else:assert c['timing']['schedulesReused']=='1'
records=0;historical_changes=set();mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(v,historical=False):
 global records
 if isinstance(v,dict):
  if {'path','byteLength','sha256'}<=v.keys():
   if historical and v['path']in allowed|mutable:historical_changes.add(v['path'])
   else:assert all(v[k]==value for k,value in entry(v['path']).items()),v['path'];records+=1
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
links=0
for name in sources:
 p=Path(name)
 if p.suffix!='.md':continue
 for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
  if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target)or target.startswith('#'):continue
  target=unquote(target.split('#')[0].split('?')[0])
  if target:assert(p.parent/target).exists()or(p.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[entry(p)for p in sorted(root.rglob('*'))if p.is_file()and p.relative_to(root).parts[0]!='attempts'and p.name not in ['plan.md','next.md','seal.log','seal-check.log']]
report=dict(format='musteroffice.retained-timing-verification/1',previousEvidence=entry(previous),scope='One owned exact interval schedule per validated consumed event prefix; full input validation, generation invalidation, atomic timeline publication and work diagnostics. Actual Native/WASM owners and historical output replay. Not full slideshow or Musterwork acceptance.',sourceFiles=[entry(p)for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),removedSources=[],workspaceChecks=checks,rustTestNames=tests,checks=dict(rustTests=len(tests),newRustTests=7,schemas=96,typescript=True,strictClippy=True,rustfmt=True,newPairedCalls=reports['product']['pairedCalls'],owners=reports['product']['owners'],diagnosticReads=inspected,renderedCalls=rendered,retainedFrames=retained,authorFrames=reports['product']['authorFrames'],sourceFrames=reports['product']['sourceFrames'],replayCalls=sum(x['pairedCalls']for x in r['cases']),timelineReplayCalls=246,oldRecordedBytesUnchanged=True,warmOwnerCostCases=6,officeWpsNewAcceptance=False,localLinks=links),reports={name:entry(root/(name+'.json'))for name in reports},componentBuilds=old['componentBuilds'],attempts=[dict(reason='Initial all-targets check found an overlapping mutable borrow in a source sample test; test operation ordering corrected before targeted and complete workspace verification.',log=entry(root/'attempts/initial-check.log'))],artifacts=artifacts,currentArtifacts=current,performance=b,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw modules only; not full font/media/runtime closure or desktop installer.'),limitations=['All full phase-one obligations remain; current timeline effect profile is still rotation.','Container time manipulation, repetition/restart, full effects/subtargets, transitions/Morph/media, advanced editable objects, production Agent packages and real Musterwork Artifact/history migration remain required.','The cache validates the full history every sample and retains one exact consumed prefix; changing prefixes still recompute, and peak rebuild memory includes the old entry plus new temporary work.','Timeline work counts do not mean delivered frames; downstream raster/page failure may preserve valid computation.','WASM host still owns termination and result fencing; no new mid-call JS cancellation or live display compositor.','Cost measurements are warm prepared author owners with controlled fixtures, excluding prepare, real fonts/media, imported-source cost, Native IPC, RSS, cold start and full installer.','Synthetic source fonts and resources do not establish production visual quality or actual Office/WPS playback compatibility; previous gradient performance/application differences remain open.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
