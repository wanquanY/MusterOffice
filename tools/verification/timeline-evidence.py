"""Seal exact timeline semantics and native boundaries without claiming playback acceptance."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/timeline');previous=Path('docs/reviews/evidence/2026-09-26-gradient-coordinate-verification.json');output=Path('docs/reviews/evidence/2026-09-26-timeline-verification.json')
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='a5b52bec7ccbd4d61e724bb16b7e839b2933cc2ffeaa38f2334dac3ac8ddc659'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''Cargo.lock
Cargo.toml
README.md
contracts/README.md
crates/mo-common/src/ids.rs
crates/mo-kernel-api/Cargo.toml
crates/mo-kernel-api/src/lib.rs
crates/mo-pptx/Cargo.toml
crates/mo-pptx/src/definitions.rs
crates/mo-pptx/src/lib.rs
crates/mo-presentation-edit/Cargo.toml
crates/mo-presentation-edit/src/apply.rs
crates/mo-presentation-edit/src/lib.rs
crates/mo-presentation-edit/src/operations.rs
crates/mo-presentation-edit/src/transaction.rs
crates/mo-presentation-model/Cargo.toml
crates/mo-presentation-model/src/document.rs
crates/mo-presentation-model/src/lib.rs
crates/mo-presentation-model/src/validation.rs
crates/mo-wasm/src/lib.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-cli/src/main.rs
tools/mo-contract-codegen/src/main.rs'''.splitlines())
added=set('''crates/mo-kernel-api/src/pptx_timing.rs
crates/mo-kernel-api/src/timeline.rs
crates/mo-kernel-api/tests/timeline.rs
crates/mo-pptx/src/timing.rs
crates/mo-pptx/src/timing/read.rs
crates/mo-pptx/src/timing/source.rs
crates/mo-pptx/tests/timing.rs
crates/mo-presentation-edit/src/timing.rs
crates/mo-presentation-edit/tests/timing.rs
crates/mo-presentation-model/src/timing.rs
crates/mo-timeline/Cargo.toml
crates/mo-timeline/src/evaluate.rs
crates/mo-timeline/src/exact.rs
crates/mo-timeline/src/generation.rs
crates/mo-timeline/src/lib.rs
crates/mo-timeline/src/model.rs
crates/mo-timeline/src/plan.rs
crates/mo-timeline/tests/timeline.rs
docs/implementation/timeline-rotation.md
tools/verification/timeline-workspace.py
tools/verification/timeline-fixtures.py
tools/verification/timeline-parity.mjs
tools/verification/timeline-reference.py
tools/verification/timeline-replay.mjs
tools/verification/timeline-evidence.py'''.splitlines())
for stem in ['document','kernel-request','kernel-response','page-placement-request','page-render-request','pptx-export-request','transaction-receipt','transaction']:
 allowed|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
for stem in ['timeline-evaluate-request','timeline-evaluate-response','pptx-timing-query','pptx-timing-response']:
 added|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items()if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text());assert [c['name']for c in checks]==['fmt','tests','clippy','native-build','schema-write','schema-check','types-write','types-check','rust-wasm','bindgen'];assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text()for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==655;assert set(old['rustTestNames'])<=set(tests)
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==87
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==87
paths={k:root/(k+'.json')for k in ['product','reference','replay']};reports={k:json.loads(p.read_text())for k,p in paths.items()}
product=reports['product'];ref=reports['reference'];replay=reports['replay'];assert product['pairedCalls']==len(product['cases'])==246
assert len(product['exports'])==2 and product['precisionFailure']['outputAbsent'] and product['precisionFailure']['exitCode']!=0
assert product['staticCompatibility']['unchangedSnapshot'] and product['staticCompatibility']['unchangedPptx']
assert ref['frames']==193 and ref['nodes']==2309 and ref['nativeBehaviors']==10 and ref['xsdParts']>0
assert replay['pairedCalls']==len(replay['cases'])==355
records=0;historical_changes=set();mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(v,historical=False):
 global records
 if isinstance(v,dict):
  if {'path','byteLength','sha256'}<=v.keys():
   if historical and v['path'] in allowed|mutable:historical_changes.add(v['path'])
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
report=dict(format='musteroffice.timeline-verification/1',previousEvidence=entry(previous),
 scope='Once-activation rotation graphs, exact event-dependent state, authored atomic edits and native PPTX mapping. Not animation rendering, complete playback or product acceptance.',
 sourceFiles=[entry(p)for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=len(tests),schemas=87,typescript=True,strictClippy=True,rustfmt=True,
  publicNativeWasmCalls=product['pairedCalls'],independentExactFrames=ref['frames'],independentNodeStates=ref['nodes'],independentProperties=ref['properties'],
  staticReplayCalls=355,staticPixelsUnchanged=True,staticAuthorSnapshotUnchanged=True,staticPptxUnchanged=True,
  nativeExports=2,nativeBehaviors=10,xsdParts=ref['xsdParts'],cliOverwriteRefused=True,cliPrecisionFailureOutputAbsent=True,
  officeWpsNewAcceptance=False,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p)for k,p in paths.items()},componentBuilds=old['componentBuilds'],artifacts=artifacts,currentArtifacts=current,
 auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],increase=current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength'],cppWasmBytes=current['cppWasm']['byteLength'],cppUnchanged=True,
  note='Raw modules only; not complete dependency, font, media, host or installer size. No playback throughput claim.'),
 limitations=['Only explicit once-activation flat rotation graphs; containers, multiple conditions, remaining effects and transitions are unimplemented.',
  'The property evaluator is not yet connected to real playback rendering, retained composition, checkpoints or media synchronization.',
  'Strict source projection rejects unimplemented native structures and does not provide preservation editing of arbitrary imported timelines.',
  'Native unsigned-millisecond export rejects unrepresentable times instead of rounding them.',
  'Replace/freeze and native timing behavior still require Office/WPS editing and playback observations; XSD and self-roundtrip are insufficient.',
  'Advanced editable objects, production Agent surfaces, full resource/session lifecycle and Musterwork replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check'in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
