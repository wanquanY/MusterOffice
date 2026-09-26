"""Seal typed PPTX transform edits and retain the independent rendering failure."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/transform-edit')
previous=Path('docs/reviews/evidence/2026-09-26-attribute-edit-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-transform-edit-verification.json')
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='16e02f082ff1177718c355dedd19fccf741d31c7fe493f0b4d4e1091bbc2657a'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
crates/mo-kernel-api/src/lib.rs
crates/mo-kernel-api/src/pptx_source.rs
crates/mo-pptx/src/source.rs
crates/mo-pptx/src/source/edit.rs
crates/mo-pptx/src/source/presentation.rs
crates/mo-pptx/src/source/surface.rs
crates/mo-pptx/src/source/transform.rs
crates/mo-wasm/src/lib.rs
tools/mo-cli/src/main.rs
tools/mo-cli/src/pptx.rs
tools/mo-contract-codegen/src/main.rs'''.splitlines())
added=set('''crates/mo-pptx/src/source/transform_edit.rs
crates/mo-pptx/src/source/transform_edit/binding.rs
crates/mo-pptx/src/source/transform_edit/types.rs
crates/mo-pptx/tests/source_transform_edit.rs
contracts/generated/pptx-transform-edits.schema.json
packages/contracts/src/generated/pptx-transform-edits.ts
docs/implementation/source-transform-edit.md
tools/verification/transform-edit-checks.py
tools/verification/transform-edit-fixtures.py
tools/verification/transform-edit-parity.mjs
tools/verification/transform-edit-reference.py
tools/verification/transform-edit-render.mjs
tools/verification/transform-edit-numeric.py
tools/verification/transform-edit-evidence.py'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'checks.json').read_text())
assert [c['name'] for c in checks]==['fmt','tests','clippy','schema-write','types-write','schema-check','types-check','native-build','rust-wasm','bindgen']
assert all(c['exitCode']==0 for c in checks)
for c in checks:
    assert not any(s in Path(c['log']).read_text() for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==636
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==83
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==83
paths={k:root/(k+'.json') for k in ['fixtures','product','reference','render','numeric']}
reports={k:json.loads(p.read_text()) for k,p in paths.items()}
fixtures=reports['fixtures'];product=reports['product'];ref=reports['reference'];render=reports['render'];numeric=reports['numeric']
assert len(fixtures['cases'])==17 and fixtures['officialXsdParts']==91
assert len(product['cases'])==90 and product['edited']==65 and product['rejected']==25 and product['regressionCalls']==121
assert product['overwriteRefused'] and product['noTemporaryFiles']
assert ref['lexicalAndSemanticCases']==65 and ref['changedXmlParts']==50 and ref['officialXsdParts']==420
assert ref['preservedParts']==ref['preservedCompressedEntries']==1315
assert len(render['cases'])==8 and render['comparedPixels']==960000
assert render['beforeNativeWasmPages']==2 and render['afterNativeWasmPages']==8
assert render['exactCases']==4 and render['discrepancyCases']==4 and render['rigidTransformInvarianceAchieved'] is False
assert {c['name']:(c['differentPixels'],c['maxChannelDifference']) for c in render['cases'] if not c['exactRigidTransform']}=={
    'off-center-translate':(4,1),'off-center-flip-h':(4,1),'off-center-flip-v':(1,1),'off-center-quarter':(3,1)}
assert numeric['status']=='unresolved-rendering-precision' and numeric['changedWordIndices']==[95,98,126,127]
assert any(s['expandedInverseDiffers'] for s in numeric['samples'])
for name in ['fixtures','parity','reference','render','numeric','checks-run']:
    assert not any(s in (root/(name+'.log')).read_text() for s in ['error:','FAILED','Traceback','AssertionError']),name

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
builds={k:Path(r['path']) for k,r in old['componentBuilds'].items()}
for path in builds.values():
    b=json.loads(path.read_text());assert b['lock']==json.loads(Path('components/skia/lock.json').read_text())
    for k in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(b[k])
    for tool in b['lock']['emsdkTools']:
        actual=entry(Path('.codex-work/emsdk')/tool['path'])
        assert actual['sha256']==tool['sha256'] and actual['byteLength']==tool['byteLength']
current={k:entry(p) for k,p in {
 'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
 'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
 'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js','cppWasm':'.codex-work/elliptic-fast/component/mo-skia.wasm'}.items()}
for k in ['typescriptRaster','cppWasm']:assert current[k]==old['currentArtifacts'][k]
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(p)==prior[p]
assert current['cppWasm']['byteLength']==2352940
markdown=links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md':continue
    markdown+=1
    for t in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',t) or t.startswith('#'):continue
        t=unquote(t.split('#')[0].split('?')[0])
        if t:assert (p.parent/t).exists() or (p.parent/t).resolve()==output.resolve(),(name,t);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[]
for p in sorted(root.rglob('*')):
    if not p.is_file() or 'attempts' in p.relative_to(root).parts:continue
    if p.name in ['next.md','seal.log','seal-check.log','initial-tests.log','unit.log','unit2.log','unit3.log']:continue
    artifacts.append(entry(p))
report=dict(format='musteroffice.transform-edit-verification/1',previousEvidence=entry(previous),
 scope='Typed edits to existing native object transform declarations, actual public runtimes and source-preserving candidate checks. An independent rendering precision criterion remains FAILED.',
 sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=636,schemas=83,strictClippy=True,rustfmt=True,typescript=True,
   newNativeWasmCalls=90,transformEditSuccesses=65,expectedRejections=25,oldProductCallsUnchanged=121,
   independentLexicalSemanticOutputs=65,independentCompressedEntriesPreserved=1315,outputOfficialXsdParts=420,
   cliCreate=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,cliNoTemporaryFiles=True,
   beforeAfterNativeWasmRenderedPages=10,independentPixelSamples=960000,exactRigidTransformCases=4,
   failedRigidTransformCases=4,rigidTransformInvarianceAchieved=False,officeWpsNewAcceptance=False,
   unchangedDependencies=True,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p) for k,p in paths.items()},componentBuilds={k:entry(p) for k,p in builds.items()},
 artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,
 historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=6436330,
   rustWasmIncrease=current['rustWasm']['byteLength']-6436330,cppWasmBytes=2352940,
   note='Raw current module bytes; no complete feature closure, fonts/media distribution or installer size estimate.'),
 unresolvedRenderingFinding=dict(status='failed-independent-exactness',affectedCases=4,totalDifferentPixels=12,
   maxChannelDifference=1,next='Trace and correct shared shader coordinate/inverse arithmetic, without compensating in the PPTX editor or weakening quality criteria.'),
 limitations=['Only existing xfrm and coordinate leaves; structural insertion/removal and coordinated alternate representations remain incomplete.',
   'Typed declaration edits are not global-position commands, advanced object semantic editing, playback or final Agent tool distribution.',
   'All Native/WASM render results agree, but four independent rigid-transform reference comparisons fail exact pixel equality.',
   'The scalar numeric diagnostic identifies position-sensitive float32 inverse arithmetic; it does not prove all Skia execution details.',
   'No new Office/WPS, cold start, RSS, performance or desktop installer acceptance.',
   'Full advanced objects, Agent/Musterwork integration and replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),rustTests=636,newCalls=90,
    auditedArtifactRecords=records,rigidTransformInvarianceAchieved=False,size=report['size'])))
