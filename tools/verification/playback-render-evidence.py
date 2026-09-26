"""Seal real exact-angle playback rendering and its explicitly limited acceptance scope."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/playback-render');previous=Path('docs/reviews/evidence/2026-09-26-timeline-verification.json');output=Path('docs/reviews/evidence/2026-09-26-playback-render-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='8f11488dd47c798822ac5aa98959b1bc2794fe61fc8ae029b6115c1ad6325487'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''Cargo.lock
README.md
contracts/README.md
crates/mo-kernel-api/src/lib.rs
crates/mo-kernel-api/src/page.rs
crates/mo-kernel-api/src/placement.rs
crates/mo-kernel-api/src/pptx_page.rs
crates/mo-kernel-api/src/timeline.rs
crates/mo-presentation-compile/Cargo.toml
crates/mo-presentation-compile/src/lib.rs
crates/mo-presentation-compile/src/page.rs
crates/mo-presentation-compile/src/placement.rs
crates/mo-presentation-compile/src/placement_core.rs
crates/mo-presentation-compile/src/source_placement.rs
crates/mo-presentation-compile/src/trig.rs
crates/mo-wasm/src/lib.rs
crates/mo-wasm/src/raster.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-cli/src/main.rs
tools/mo-cli/src/raster.rs
tools/mo-contract-codegen/src/main.rs
tools/mo-raster-worker/src/main.rs'''.splitlines())
added=set('''crates/mo-presentation-compile/src/angle.rs
crates/mo-presentation-compile/src/playback.rs
crates/mo-presentation-compile/tests/playback.rs
crates/mo-kernel-api/src/playback.rs
crates/mo-kernel-api/tests/playback.rs
fixtures/presentations/playback/page.json
docs/implementation/playback-rendering.md
tools/verification/playback-render-workspace.py
tools/verification/playback-render-parity.mjs
tools/verification/playback-render-reference.py
tools/verification/playback-render-replay.mjs
tools/verification/playback-render-benchmark.mjs
tools/verification/playback-render-preview.py
tools/verification/playback-render-evidence.py'''.splitlines())
for stem in ['playback-page-request','playback-compile-response','playback-raster-response']:
 added|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text());assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen'];assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==665;assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==90
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==90
paths={k:root/(k+'.json')for k in ['product','reference','replay','benchmark','preview']};reports={k:json.loads(p.read_text())for k,p in paths.items()}
product=reports['product'];ref=reports['reference'];replay=reports['replay'];bench=reports['benchmark']
assert product['pairedCalls']==76 and len(product['cases'])==38 and product['rendered']==34
assert product['cli']['overwriteExit']!=0 and product['cli']['failureOutputAbsent']
assert ref['frames']==34 and ref['objects']==104 and ref['scalarChecks']==1040 and ref['precisionDigits']==100
assert ref['rigidPixels']==307200 and ref['rigidCases']==4 and ref['seekAndRecoveryEqual']
assert replay['pairedCalls']==len(replay['cases'])==355 and replay['timelineCalls']==len(replay['timelineCases'])==246
assert len(bench['cases'])==4 and bench['conditions']['warmup']==5 and bench['conditions']['samples']==25
for c in bench['cases']:
 case=next(r for r in product['cases']if r['name']==c['name']);assert case['pixels']['sha256']==c['animated']['pixelSha256']
 for mode in ['animated','staticControl']:
  if mode not in c:continue
  assert c[mode]['pixelSha256']==case['pixels']['sha256']
  for part in ['total','raster','remaining']:assert len(c[mode][part]['samplesMs'])==25
for name in ['parity','reference','replay','benchmark']:
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
for p in ['pnpm-lock.yaml','components/skia/lock.json']:assert entry(p)==prior[p]
assert current['rustWasm']['byteLength']==7170320
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
report=dict(format='musteroffice.playback-render-verification/1',previousEvidence=entry(previous),
 scope='Exact rational animation rotation integrated into shared DrawingML placement, certified author solid-path page rendering and real Native/WASM pixels. Not complete playback or product acceptance.',
 sourceFiles=[entry(p)for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),schemas=90,typescript=True,strictClippy=True,rustfmt=True,publicNativeWasmCalls=76,renderedFrames=34,independentPlacementObjects=104,independentGeometryScalars=1040,independentPrecisionDigits=100,exactRigidPixels=307200,exactRigidCases=4,seekRecoveryEqual=True,staticReplayCalls=355,timelineReplayCalls=246,staticPixelsUnchanged=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p)for k,p in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,note='Raw modules only; not full host/dependency/font/media closure or installer size.'),
 performance=[dict(name=c['name'],sampledMedianMs=c['animated']['total']['medianMs'],staticSamePixelsMedianMs=c.get('staticControl',{}).get('total',{}).get('medianMs'))for c in bench['cases']],
 limitations=['Author solid-path/group profile only; imported resource/text/image/gradient page playback remains to be connected.',
 'Public JSON requests create a plan each call; Rust plans retain immutable documents/timeline/base placement, but dynamic placement, paths and scene lowering still rebuild per frame.',
 'No complete retained compositor, WASM/host playback session, display scheduling, media synchronization or resource lease lifecycle.',
 'Timing containers, remaining effects/subtargets, transitions/Morph and advanced editable objects remain required.',
 'Group and effect behavior still require Office/WPS animation editing and playback observations.',
 'Warm one-machine 320x240 synthetic pages do not establish production playback throughput, cold start, RSS or full installer footprint.',
 'Production Agent distribution, Musterwork host/Artifact integration, migration and full replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
