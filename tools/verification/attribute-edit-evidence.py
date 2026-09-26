"""Seal source XML attribute edits, independent checks and product regression."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote

root=Path('.codex-work/attribute-edit')
previous=Path('docs/reviews/evidence/2026-09-26-elliptic-fast-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-attribute-edit-verification.json')
def entry(p):
    p=Path(p);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='5ff78b1e7d6c376a295333da1984316b221cfd98796d01b9395082c9ee489483'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
crates/mo-xml/src/lib.rs
crates/mo-xml/src/text_edit.rs'''.splitlines())
added=set('''crates/mo-xml/src/attribute_edit.rs
crates/mo-xml/src/edit_bytes.rs
crates/mo-xml/tests/attribute_edit.rs
docs/implementation/source-attribute-overlay.md
tools/verification/attribute-edit-checks.py
tools/verification/attribute-edit-reference.py
tools/verification/attribute-edit-parity.mjs
tools/verification/attribute-edit-product-parity.mjs
tools/verification/xml-attribute-probe.rs
tools/verification/stationary-gradient-fixtures.py
tools/verification/attribute-edit-evidence.py'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r}
assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:
        assert len(Path(p).read_text().splitlines())<=2000,p

checks=json.loads((root/'checks.json').read_text())
assert [c['name'] for c in checks]==['fmt','tests','clippy','native-build','schema-check','types-check',
    'rust-wasm','bindgen','native-xml','wasm-xml','native-probe','wasm-probe']
assert all(c['exitCode']==0 for c in checks)
for c in checks:
    log=Path(c['log']).read_text()
    assert not any(s in log for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
assert len(tests)==628
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==82
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==82
paths={k:root/(k+'.json') for k in ['reference','product']}
paths['stationaryInputs']=Path('.codex-work/stationary-gradient/inputs.json')
reports={k:json.loads(p.read_text()) for k,p in paths.items()}
ref=reports['reference'];product=reports['product'];stationary=reports['stationaryInputs']
assert len(ref['cases'])==ref['pairedCalls']==416 and ref['successes']==386
assert sum(c['status']==0 for c in ref['cases'])==386
assert ref['independentParser']=='lxml/libxml2'
assert Path(ref['nativeOutput']['path']).read_bytes()==Path(ref['wasmOutput']['path']).read_bytes()
assert product['comparedCalls']==121 and product['inspectionCalls']==72 and product['editCalls']==49
assert product['edited']==42 and product['rejected']==7 and product['overwriteRefused'] and product['noTemporaryFiles']
assert len(stationary['cases'])==30 and stationary['officialXsdParts']==210
assert stationary['status']=='inputs-only; target observation awaits system permission'
for path in [root/'reference.log',root/'product.log',root/'probe-build.log',root/'probe-wasm.log',
             Path('.codex-work/stationary-gradient/fixtures.log')]:
    assert not any(s in path.read_text() for s in ['error:','FAILED','Traceback','AssertionError']),path

records=0;historical_changes=set()
mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(v,historical=False):
    global records
    if isinstance(v,dict):
        if {'path','byteLength','sha256'}<=v.keys():
            if historical and v['path'] in allowed|mutable:historical_changes.add(v['path'])
            else:
                actual=entry(v['path'])
                assert all(v[k]==value for k,value in actual.items()),v['path']
                records+=1
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
 'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js',
 'cppWasm':'.codex-work/elliptic-fast/component/mo-skia.wasm',
 'nativeXmlProbe':str(root/'native-probe'),'wasmXmlProbe':str(root/'probe.wasm')}.items()}
for k in ['typescriptRaster','cppWasm']:assert current[k]==old['currentArtifacts'][k]
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(p)==prior[p]
assert current['rustWasm']['byteLength']==6436330 and current['cppWasm']['byteLength']==2352940
assert current['wasmXmlProbe']['byteLength']==341898
assert current['rustWasm']['byteLength']-old['currentArtifacts']['rustWasm']['byteLength']==66
markdown=links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md':continue
    markdown+=1
    for t in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',t) or t.startswith('#'):continue
        t=unquote(t.split('#')[0].split('?')[0])
        if t:
            assert (p.parent/t).exists() or (p.parent/t).resolve()==output.resolve(),(name,t)
            links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[]
excluded={'next.md','seal.log','seal-check.log','checks-run.log','initial-tests.log','clippy1.log',
          'tests1.log','tests2.log','tests3.log'}
for p in sorted(root.rglob('*')):
    if not p.is_file() or p.name in excluded or p.suffix=='.o':continue
    if any(s.startswith('attempt') for s in p.relative_to(root).parts):continue
    artifacts.append(entry(p))
for p in sorted(Path('.codex-work/stationary-gradient/sources').glob('*.pptx')):artifacts.append(entry(p))
artifacts.append(entry('.codex-work/stationary-gradient/fixtures.log'))
report=dict(format='musteroffice.attribute-edit-verification/1',previousEvidence=entry(previous),
 scope='Atomic source XML attribute edits with encoding preservation; format-semantic binding and public Agent property operations remain incomplete.',
 sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=628,schemas=82,strictClippy=True,rustfmt=True,typescript=True,
   xmlProbeNativeWasmCases=416,independentXmlSuccesses=386,productPairedCallsUnchanged=121,
   sourceInspections=72,sourceTextEdits=42,sourceTextExpectedRejections=7,
   cliCreate=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,cliNoTemporaryFiles=True,
   stationaryInputPptx=30,stationaryOfficialXsdParts=210,stationaryTargetObservation=False,
   unchangedDependencies=True,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p) for k,p in paths.items()},componentBuilds={k:entry(p) for k,p in builds.items()},
 artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,
 historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=6436330,previousRustWasmBytes=6436264,increase=66,
   cppWasmBytes=2352940,xmlDiagnosticWasmBytes=341898,
   note='Raw modules. Public product API does not yet call the attribute editor; diagnostic probe includes shared dependencies. Neither additive full feature cost nor installer size.'),
 limitations=['AttributeEdit physical ordinals are internal bindings, not stable Agent object identifiers.',
   'PPTX source digest, inheritance and coordinated multi-representation attribute edits remain adapter work.',
   'Existing PPTX inspection/text edit regression does not establish public property editing or Office/WPS interoperability.',
   'Thirty stationary gradient inputs are valid owned fixtures only; native application observation awaits OS permissions.',
   'No new raster performance, memory, startup or installation measurements.',
   'Full advanced objects, playback, Agent distribution and Musterwork replacement acceptance remain incomplete.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence'
    output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),rustTests=628,
    xmlPairs=416,productCalls=121,auditedArtifactRecords=records)))
