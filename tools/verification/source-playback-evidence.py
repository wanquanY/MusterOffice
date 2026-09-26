"""Bind native source animation, immutable resource rendering and exact comparison evidence."""
import hashlib,json,re,subprocess,sys,math
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/source-playback')
previous=Path('docs/reviews/evidence/2026-09-26-playback-session-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-source-playback-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='f1bdc4d60e74a7972739f567a4a8151800c6dfbf329573d9aefd82359bb06df7'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
contracts/README.md
crates/mo-kernel-api/src/lib.rs
crates/mo-kernel-api/src/pptx_resource_page.rs
crates/mo-kernel-api/src/pptx_resource_page/diagnostic.rs
crates/mo-pptx/src/timing/source.rs
crates/mo-presentation-compile/src/lib.rs
crates/mo-presentation-compile/src/source_page.rs
crates/mo-presentation-compile/src/source_page/objects.rs
crates/mo-presentation-compile/src/source_page/prepared.rs
crates/mo-presentation-compile/src/source_placement.rs
crates/mo-presentation-compile/src/source_resource_page.rs
crates/mo-wasm/src/raster.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-cli/src/main.rs
tools/mo-cli/src/raster.rs
tools/mo-contract-codegen/src/main.rs
tools/mo-raster-worker/src/main.rs'''.splitlines())
added=set('''crates/mo-kernel-api/src/pptx_playback.rs
crates/mo-kernel-api/src/pptx_resource_page/prepare.rs
crates/mo-presentation-compile/src/source_playback.rs
crates/mo-kernel-api/tests/pptx_playback.rs
crates/mo-presentation-compile/tests/source_playback.rs
tools/test-support/source_playback.rs
tools/verification/source-playback-fixtures.py
tools/verification/source-playback-workspace.py
tools/verification/source-playback-parity.mjs
tools/verification/source-playback-replay.mjs
tools/verification/source-playback-session-replay.mjs
tools/verification/source-playback-benchmark.mjs
tools/verification/source-playback-preview.py
tools/verification/source-playback-evidence.py
docs/implementation/source-playback.md'''.splitlines())
for stem in ['pptx-playback-page-request','pptx-playback-raster-response']:
 added|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
assert len(sources)==1410
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text())
assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen']
assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==678
assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==94
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==94
paths={k:root/(k+'.json')for k in ['fixtures','product','replay','benchmark','preview']};paths['sessionReplay']=root/'session/product.json'
reports={k:json.loads(p.read_text())for k,p in paths.items()}
fixtures=reports['fixtures'];product=reports['product'];replay=reports['replay'];bench=reports['benchmark'];session=reports['sessionReplay']
assert len(fixtures['cases'])==44 and fixtures['positiveCases']==37 and fixtures['officialXsdParts']==238
assert product['pairedCalls']==70 and len(product['cases'])==44 and product['renderedFrames']==37
assert product['independentNativeTransformControls']==26 and product['seekRecovery']
assert product['cli']['overwriteExit']!=0 and product['cli']['failureOutputAbsent']
assert replay['pairedCalls']==len(replay['cases'])==355 and replay['timelineCalls']==len(replay['timelineCases'])==246
assert session['owners']==9 and session['pairedCalls']==len(session['cases'])==111 and session['renderedReplayFrames']==34
assert session['wasmComponentFailureRecovery'] and all(c['exitCode']not in [None,0] for c in session['nativeFramingFailures'])
angles=bindings=control_pixels=0
for c in product['cases']:
 response=json.loads(Path(c['response']['path']).read_text());pixels=Path(c['pixels']['path']).read_bytes();f=next(f for f in fixtures['cases']if f['name']==c['name'])
 if response['status']=='rendered':
  info=response['info'];assert c['rasters']==1 and hashlib.sha256(pixels).hexdigest()==info['page']['page']['scene']['raster']['sha256']
  assert info['playback']['evaluated']['state']['rotations']==f['expectedRotations'];angles+=len(f['expectedRotations'])
  assert info['playback']['objectBindings']==f['expectedBindings'];bindings+=len(f['expectedBindings'])
  assert info['playback']['sourceSha256']==c['source']['sha256']
 else:
  assert c['rasters']==0 and not pixels
  if f['noComponentCalls']:assert c['decodes']==c['shapes']==0
 if 'staticControl'in c:
  control=c['staticControl'];assert c['pixels']['sha256']==control['pixelSha256'] and c['frame']['sha256']==control['frameSha256'];control_pixels+=len(pixels)//4
assert (angles,bindings,control_pixels)==(56,60,3120000)
assert len(bench['cases'])==4 and bench['conditions']['warmup']==5 and bench['conditions']['samples']==20
for c in bench['cases']:
 case=next(r for r in product['cases']if r['name']==c['name']);assert c['pixelSha256']==case['pixels']['sha256']
 for mode in ['animated','static']:
  r=c[mode];v=r['samplesMs'];assert len(v)==20 and all(math.isfinite(x)and x>=0 for x in v)
  assert r['medianMs']==sorted(v)[len(v)//2] and r['p95Ms']==sorted(v)[math.ceil(len(v)*.95)-1]
for name in ['fixtures','parity','replay','session-replay','benchmark','preview']:
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
assert current['rustWasm']['byteLength']==7346932
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
report=dict(format='musteroffice.source-playback-verification/1',previousEvidence=entry(previous),
 scope='Native slide rotation timing drives digest-bound source placement and shared text/image/gradient resource-page rendering. Source bytes and declarations remain immutable. Not complete slideshow or product acceptance.',
 sourceFiles=[entry(p)for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),newRustTests=5,schemas=94,typescript=True,strictClippy=True,rustfmt=True,publicNativeWasmCalls=70,sourceFrames=37,independentExactAngles=angles,sourceObjectBindings=bindings,independentNativeTransformControls=26,exactControlPixels=control_pixels,officialXsdParts=238,seekRecovery=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,staticReplayCalls=355,timelineReplayCalls=246,authorSessionReplayCalls=111,authorSessionReplayFrames=34,staticPixelsUnchanged=True,officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p)for k,p in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw modules only; not full host/dependency/font/media closure or installer size.'),
 performance=[dict(name=c['name'],viewport=c['viewport'],sourceAnimatedMedianMs=c['animated']['medianMs'],samePixelsStaticMedianMs=c['static']['medianMs'])for c in bench['cases']],
 limitations=['Current native once-activation rotation timing projection; unsupported source timing is rejected rather than ignored.',
 'Rust time plan can be reused but public JSON still reopens/indexes source and prepares fonts/decoded images/shaped glyphs each call; source resource sessions remain required.',
 'Dynamic placement, paths and scene lowering still rebuild per sample. No complete retained compositor, display scheduling or media synchronization.',
 'Same existing static resource-page scope and diagnostics apply, including unimplemented stationary image/gradient orientation and unsupported visible content.',
 'Synthetic owned fonts/images and limited source cases do not prove real-world visual quality or Office/WPS animation and editability parity.',
 'Source-byte revision identity is not product authorization/history provenance or a complete engine/resource version tuple.',
 'Timing containers, remaining effects/subtargets, transitions/Morph and advanced editable objects remain required.',
 'Warm one-machine per-call cost does not establish cold start, RSS, production FPS or full installer footprint. Circle case remains around 18 ms; historical gradient performance regression remains open.',
 'Production host/Agent distribution, actual Musterwork integration/migration and full replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
