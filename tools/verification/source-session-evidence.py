"""Seal explicit source delta, retained resources, public sessions and regression evidence."""
import hashlib,json,re,subprocess,sys,math
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/source-session')
previous=Path('docs/reviews/evidence/2026-09-26-source-playback-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-source-session-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='47c9c046134e4761fe9990a8879d15cdf7f0d5c82a9179dff67f8186ecc3cce7'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
contracts/README.md
crates/mo-kernel-api/src/lib.rs
crates/mo-kernel-api/src/playback_session.rs
crates/mo-kernel-api/src/pptx_playback.rs
crates/mo-presentation-compile/src/source_page.rs
crates/mo-presentation-compile/src/source_page/emit.rs
crates/mo-presentation-compile/src/source_playback.rs
crates/mo-presentation-compile/src/source_resource_page.rs
crates/mo-presentation-compile/src/source_text_page.rs
crates/mo-raster/src/image/resources.rs
crates/mo-raster/src/image_tests.rs
crates/mo-wasm/src/playback.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-contract-codegen/src/main.rs
tools/mo-raster-worker/src/main.rs'''.splitlines())
added=set('''crates/mo-kernel-api/src/playback_owner.rs
crates/mo-kernel-api/src/pptx_playback_session.rs
crates/mo-kernel-api/tests/pptx_playback_session.rs
crates/mo-presentation-compile/src/source_resource_page/retained.rs
crates/mo-presentation-compile/src/source_text_page/placement.rs
crates/mo-presentation-compile/src/source_text_page/retained.rs
tools/mo-raster-worker/src/source_playback.rs
tools/verification/source-session-fixtures.py
tools/verification/source-session-workspace.py
tools/verification/source-session-parity.mjs
tools/verification/source-session-replay.mjs
tools/verification/source-session-author-replay.mjs
tools/verification/source-session-source-replay.mjs
tools/verification/source-session-benchmark.mjs
tools/verification/source-session-preview.py
tools/verification/source-session-evidence.py
docs/implementation/source-playback-sessions.md'''.splitlines())
for stem in ['pptx-playback-session-request','pptx-playback-session-response']:
 added|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
assert len(sources)==1431
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text())
assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen']
assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==683,len(tests)
assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==96
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==96
paths={k:root/(k+'.json')for k in ['fixtures','product','replay','benchmark','preview']};paths.update(authorReplay=root/'session/product.json',sourceReplay=root/'oneshot/product.json')
reports={k:json.loads(p.read_text())for k,p in paths.items()}
f=reports['fixtures'];p=reports['product'];b=reports['benchmark'];r=reports['replay'];a=reports['authorReplay'];s=reports['sourceReplay']
assert len(f['cases'])==7 and f['officialXsdParts']==24
assert p['owners']==13 and p['pairedCalls']==len(p['cases'])==135 and p['replayFrames']==37 and p['mixedFrames']==7
assert p['inputsImmutable']and p['wasmResourceFailureRecovery']and all(x['exitCode']not in [None,0]for x in p['framing'])
assert r['pairedCalls']==len(r['cases'])==355 and r['timelineCalls']==len(r['timelineCases'])==246
assert a['owners']==9 and a['pairedCalls']==len(a['cases'])==111 and a['renderedReplayFrames']==34
assert s['pairedCalls']==70 and s['renderedFrames']==37 and s['independentNativeTransformControls']==26
assert a['wasmComponentFailureRecovery']and all(x['exitCode']not in [None,0]for x in a['nativeFramingFailures'])
assert s['cli']['overwriteExit']!=0 and s['cli']['failureOutputAbsent']
frames=prepares=zero=0
for c in p['cases']:
 response=json.loads(Path(c['response']['path']).read_text());pixels=Path(c['pixels']['path']).read_bytes();q=json.loads(Path(c['request']['path']).read_text())
 if response['status']=='rendered':
  info=response['info'];assert c['rasters']==1 and hashlib.sha256(pixels).hexdigest()==info['page']['page']['scene']['raster']['sha256'];frames+=1
  assert c['decodes']==c['shapes']==info['page']['gatherCopyBytes']==0
  for key in ['componentCalls','fontUploadBytes','requestWords']:assert info['page']['textWork'][key]==0
  zero+=1
  if c['name'].startswith('mixed-'):
   fi=next(fi for fi in f['cases']if fi['name']==c['name']);assert info['playback']['evaluated']['state']['rotations']==fi['expectedRotations']
  if 'frozen'in c:
   expected=json.loads(Path(c['frozen']['path']).read_text())
   for k in ['componentCalls','fontUploadBytes','requestWords']:expected['info']['page']['textWork'][k]=0
   expected['info']['page']['gatherCopyBytes']=0;assert expected==response
 else:assert c['rasters']==0 and not pixels
 if response['status']=='prepared':
  preparation=response['info']['preparation'];assert preparation['decodedImages']==c['decodes']and preparation['textWork']['componentCalls']==c['shapes'];prepares+=1
 if q['operation']!='prepare':assert c['decodes']==c['shapes']==0
assert (frames,zero,prepares)==(52,52,7),(frames,zero,prepares)
control_pixels=sum(next(c['pixels']['byteLength']for c in p['cases']if c['name']==v['name'])//4 for v in p['controls']if 'staticControl'in v)
assert control_pixels==360000,control_pixels
for v in p['controls']:
 c=next(c for c in p['cases']if c['name']==v['name']);assert c['pixels']['sha256']==v['pixelSha256']
 if 'staticControl'in v:assert c['pixels']['sha256']==v['staticPixelSha256']
assert len(b['cases'])==4 and b['conditions']['samples']==20 and b['conditions']['warmup']==5
for c in b['cases']:
 oldcase=next(v for v in s['cases']if v['name']==c['name']);assert c['pixelSha256']==oldcase['pixels']['sha256']
 for mode,count in [('oneShot',20),('retained',20),('prepare',5)]:
  v=c[mode];n=v['samplesMs'];assert len(n)==count and all(math.isfinite(x)and x>=0 for x in n);assert v['medianMs']==sorted(n)[len(n)//2]and v['p95Ms']==sorted(n)[math.ceil(len(n)*.95)-1]
for name in ['fixtures','parity','replay','author-replay','source-replay','benchmark','preview']:
 assert not any(s in (root/(name+'.log')).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),name
records=0;historical_changes=set();mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(v,historical=False):
 global records
 if isinstance(v,dict):
  if {'path','byteLength','sha256'}<=v.keys():
   if historical and v['path']in allowed|mutable:historical_changes.add(v['path'])
   else:
    actual=entry(v['path']);assert all(v[k]==value for k,value in actual.items()),v['path'];records+=1
  for x in v.values():audit(x,historical)
 elif isinstance(v,list):
  for x in v:audit(x,historical)
audit(reports);audit(old,True)
for v in old['reports'].values():audit(json.loads(Path(v['path']).read_text()),True)
for v in old['componentBuilds'].values():
 build=json.loads(Path(v['path']).read_text());assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
 for k in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(build[k])
current={k:entry(path)for k,path in {
 'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
 'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
 'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js','cppWasm':'.codex-work/gradient-coordinates/component/mo-skia.wasm'}.items()}
for k in ['typescriptRaster','cppWasm']:assert current[k]==old['currentArtifacts'][k]
for path in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(path)==prior[path]
markdown=links=0
for name in sources:
 path=Path(name)
 if path.suffix!='.md':continue
 markdown+=1
 for t in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
  if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',t)or t.startswith('#'):continue
  t=unquote(t.split('#')[0].split('?')[0])
  if t:assert (path.parent/t).exists()or(path.parent/t).resolve()==output.resolve(),(name,t);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[entry(path)for path in sorted(root.rglob('*'))if path.is_file()and path.relative_to(root).parts[0]!='attempts'and path.name not in ['next.md','seal.log','seal-check.log']]
report=dict(format='musteroffice.source-session-verification/1',previousEvidence=entry(previous),
 scope='Explicit retained source index/time graph, owned decoded image bundle and compact local text paint; shared certified per-frame placement and owner lifecycle across Native/WASM. Not full slideshow/product acceptance.',
 sourceFiles=[entry(path)for path in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),newRustTests=5,schemas=96,typescript=True,strictClippy=True,rustfmt=True,sourceSessionOwners=p['owners'],sourceSessionCalls=p['pairedCalls'],sourceSessionFrames=frames,zeroSampleDecodeAndShapingFrames=zero,priorExactFrames=37,mixedGroupFrames=7,independentNativeControlPixels=control_pixels,officialXsdParts=f['officialXsdParts'],staticReplayCalls=355,timelineReplayCalls=246,authorReplayCalls=111,sourceReplayCalls=70,oldResultsUnchangedExceptExplicitSessionWork=True,officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),
 reports={k:entry(path)for k,path in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw module sizes, not full host/font/media dependency closure or installer size.'),
 performance=[dict(name=c['name'],viewport=c['viewport'],prepareMedianMs=c['prepare']['medianMs'],oneShotMedianMs=c['oneShot']['medianMs'],retainedMedianMs=c['retained']['medianMs'],reductionPercent=100*(1-c['retained']['medianMs']/c['oneShot']['medianMs']))for c in b['cases']],
 limitations=['Current native once-activation rotation profile and prior resource-page scope only; all full phase-one obligations remain.',
 'Preparation certifies declared base pose and rejects failure atomically; it is not arbitrary-time lazy preparation.',
 'Source geometry, paint placement, precision and scene compilation still rebuild. Pixel resources are transferred to raster component each frame; no GPU retained texture ABI or zero-copy claim.',
 'Logical retained data byte limits are not whole-process RSS, peak allocation or total bundle size promises.',
 'One-machine warm synthetic small-page measurements exclude production host, scheduling, cold start and installer size; historical gradient regression remains open.',
 'Source digest/generation is not product authorization or history provenance; planId is not a serialized cache compatibility or full dependency fingerprint.',
 'Synthetic fonts/images and independent native transform controls do not establish real-world visual quality, complete animations or Office/WPS interoperability.',
 'Timing containers, effects/subtargets, transitions/Morph/media and advanced editable objects remain required.',
 'Production Agent packaging, actual Musterwork Artifact/history migration and complete replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
