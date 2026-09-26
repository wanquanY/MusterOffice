"""Bind retained-session source delta, real command streams and bounded cost claims."""
import hashlib,json,re,subprocess,sys,math
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/playback-session')
previous=Path('docs/reviews/evidence/2026-09-26-playback-render-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-playback-session-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='6b1e6fc47b67c15ef30ba92e0971633f0f988464f26d019bdb67fd7c06b10035'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
contracts/README.md
crates/mo-kernel-api/src/lib.rs
crates/mo-kernel-api/src/playback.rs
crates/mo-presentation-compile/src/playback.rs
crates/mo-wasm/src/lib.rs
crates/mo-wasm/src/raster.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-contract-codegen/src/main.rs
tools/mo-raster-worker/src/main.rs'''.splitlines())
added=set('''crates/mo-kernel-api/src/playback_session.rs
crates/mo-kernel-api/tests/playback_session.rs
crates/mo-wasm/src/playback.rs
tools/mo-raster-worker/src/playback.rs
tools/verification/playback-session-workspace.py
tools/verification/playback-session-parity.mjs
tools/verification/playback-session-replay.mjs
tools/verification/playback-session-benchmark.mjs
tools/verification/playback-session-evidence.py
docs/implementation/playback-sessions.md'''.splitlines())
for stem in ['playback-session-request','playback-session-response']:
 added|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
assert len(sources)==1391
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text())
assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen']
assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==673
assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==92
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==92
paths={k:root/(k+'.json')for k in ['product','replay','benchmark']};reports={k:json.loads(p.read_text())for k,p in paths.items()}
product=reports['product'];replay=reports['replay'];bench=reports['benchmark']
assert product['pairedCalls']==len(product['cases'])==111 and product['owners']==9
assert product['replayedCalls']==74 and product['renderedReplayFrames']==34
assert product['wasmComponentFailureRecovery']
assert len(product['nativeFramingFailures'])==2 and all(c['exitCode']not in [None,0] for c in product['nativeFramingFailures'])
assert replay['pairedCalls']==len(replay['cases'])==355 and replay['timelineCalls']==len(replay['timelineCases'])==246
frozen=json.loads(Path(old['reports']['product']['path']).read_text())
for c in product['cases']:
 response=json.loads(Path(c['response']['path']).read_text());pixels=Path(c['pixels']['path']).read_bytes()
 if response['status']=='rendered':
  assert c['calls']==1 and hashlib.sha256(pixels).hexdigest()==response['info']['page']['scene']['raster']['sha256']
 else:assert c['calls']==0 and not pixels
assert len(bench['cases'])==4 and bench['conditions']['warmup']==10 and bench['conditions']['samples']==30
for c in bench['cases']:
 old_case=next(r for r in frozen['cases']if r['name']==c['name']);assert c['pixelSha256']==old_case['pixels']['sha256']
 assert c['requestBytes']==old_case['request']['byteLength'] and c['sampleBytes']<c['requestBytes']
 for mode in ['perCall','retained']:
  for part in ['total','raster','remaining']:
   s=c[mode][part];v=s['samplesMs'];assert len(v)==30 and all(math.isfinite(x)and x>=0 for x in v)
   assert s['medianMs']==sorted(v)[len(v)//2] and s['p95Ms']==sorted(v)[math.ceil(len(v)*.95)-1]
for name in ['parity','replay','benchmark']:
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
for r in old['reports'].values():audit(json.loads(Path(r['path']).read_text()),True)
for record in old['componentBuilds'].values():
 b=json.loads(Path(record['path']).read_text());assert b['lock']==json.loads(Path('components/skia/lock.json').read_text())
 for k in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(b[k])
current={k:entry(p)for k,p in {
 'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
 'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
 'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js','cppWasm':'.codex-work/gradient-coordinates/component/mo-skia.wasm'}.items()}
for k in ['typescriptRaster','cppWasm']:assert current[k]==old['currentArtifacts'][k]
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(p)==prior[p]
assert current['rustWasm']['byteLength']==7284988
markdown=links=0
for name in sources:
 p=Path(name)
 if p.suffix!='.md':continue
 markdown+=1
 for t in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
  if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',t)or t.startswith('#'):continue
  t=unquote(t.split('#')[0].split('?')[0])
  if t:assert (p.parent/t).exists()or(p.parent/t).resolve()==output.resolve(),(name,t);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[]
for p in sorted(root.rglob('*')):
 if not p.is_file()or p.relative_to(root).parts[0]=='attempts' or p.name in ['next.md','seal.log','seal-check.log']:continue
 artifacts.append(entry(p))
report=dict(format='musteroffice.playback-session-verification/1',previousEvidence=entry(previous),
 scope='One immutable author page plan per owned Rust/WASM session or native isolated worker, explicit generation fencing and lifecycle. Not complete slideshow host or retained compositor.',
 sourceFiles=[entry(p)for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),newRustTests=8,schemas=92,typescript=True,strictClippy=True,rustfmt=True,publicNativeWasmCalls=111,persistentOwnerStreams=9,replayedPlaybackCalls=74,unchangedPlaybackPixelFrames=34,wasmComponentFailureRecovery=True,nativeFramingFailures=2,staticReplayCalls=355,timelineReplayCalls=246,staticPixelsUnchanged=True,officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p)for k,p in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw modules only; not full host/dependency/font/media closure or installer size.'),
 performance=[dict(name=c['name'],perCallMedianMs=c['perCall']['total']['medianMs'],retainedMedianMs=c['retained']['total']['medianMs'],requestBytes=c['requestBytes'],sampleBytes=c['sampleBytes'])for c in bench['cases']],
 limitations=['Current author solid-path rotation profile only; source resource/text/image/gradient playback remains unconnected.',
 'Prepared plan reuses document, timeline and base placements; dynamic placement, paths and scene lowering still rebuild per frame.',
 'No complete slideshow state machine, retained compositor, media synchronization, display scheduling or resource lease lifecycle.',
 'Host must bound owner counts and frame queues, raise generation on displayed seeks, reject late results and own process/worker cancellation.',
 'Plan ID is profile-bound content identity, not an authorization token or persisted compiled-plan/engine compatibility tuple.',
 'Preparation does not prove all sample times are drawable; unsupported content, precision budgets or backend errors can reject a sample.',
 'Timing containers, remaining effects/subtargets, transitions/Morph and advanced editable objects remain required.',
 'No new Office/WPS animation/editability observations. Pending system permission has not been bypassed.',
 'Warm one-machine 320x240 cost comparison does not establish cold start, RSS, production FPS or full installer footprint. Historical gradient performance regression remains open.',
 'Agent distribution, actual Musterwork integration/migration and full replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
