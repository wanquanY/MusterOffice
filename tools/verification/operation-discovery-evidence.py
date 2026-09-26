"""Bind the shared host port and live capability/schema discovery to evidence."""
import hashlib
import gzip
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root=Path('.codex-work/operation-discovery')
parent_path=Path('docs/reviews/evidence/2026-09-26-export-host-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-operation-discovery-verification.json')
def entry(path):
    p=Path(path); b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())

assert entry(parent_path)['sha256']=='913785c32342065aa09315263061dd74f781e1489517408790aedf0a5c50b181'
parent=json.loads(parent_path.read_text()); prior={r['path']:r for r in parent['sourceFiles']}
allowed=set('''Cargo.lock
README.md
crates/mo-common/src/lib.rs
crates/mo-operation-service/src/contract.rs
crates/mo-operation-service/src/lib.rs
crates/mo-standard-host/src/exports.rs
crates/mo-standard-host/src/lib.rs
crates/mo-standard-host/tests/durable.rs
crates/mo-standard-host/tests/exports.rs
crates/mo-standard-host/tests/resource_recovery.rs
crates/mo-standard-host/tests/resources.rs
crates/mo-standard-host/tests/results.rs
crates/mo-wasm/Cargo.toml
crates/mo-wasm/src/lib.rs
docs/README.md
docs/design/agent-interfaces.md
docs/implementation/dependencies.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-contract-codegen/Cargo.toml
tools/mo-contract-codegen/src/main.rs'''.splitlines())
for stem in ['host-request','host-response']:
    allowed |= {f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items() if entry(p)!=r}
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-operation-service/src/discovery.rs
crates/mo-operation-service/src/host.rs
crates/mo-operation-service/src/schemas.rs
crates/mo-operation-service/tests/discovery.rs
crates/mo-standard-host/src/operation_host.rs
crates/mo-standard-host/tests/discovery.rs
crates/mo-standard-host/tests/support/mod.rs
docs/implementation/operation-discovery.md
tools/verification/operation-discovery-workspace.py
tools/verification/operation-discovery-reference.py
tools/verification/operation-discovery-parity.py
tools/verification/operation-discovery-evidence.py'''.splitlines())
for stem in ['host-capabilities','schema-document']:
    added |= {f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
assert not added & set(prior)
sources=set(prior)|added
for name in sources:
    p=Path(name)
    if p.suffix in ['.rs','.py','.mjs','.ts','.cpp','.h']:
        assert len(p.read_text().splitlines())<=2000,name
history=json.loads((root/'workspace.json').read_text()); checks={c['name']:c for c in history}
assert set(checks)=={'fmt','tests','clippy','native-build','schema-check','types-check',
    'pure-operation-wasm','rust-wasm','bindgen','native-integration','export-integration','native-tree','wasm-tree','release-build'}
assert all(c['exitCode']==0 for c in checks.values())
for c in checks.values():
    assert not any(bad in Path(c['log']).read_text() for bad in ['error:','FAILED','Traceback','AssertionError'])
tests=re.findall(r'^test (.+) \.\.\. ok$',Path(checks['tests']['log']).read_text(),re.M)
assert len(tests)==826 and set(parent['rustTestNames'])<=set(tests)
assert len(set(tests)-set(parent['rustTestNames']))==5
fixture_path=root/'fixture-validation.json'
fixture_checks=json.loads(fixture_path.read_text())
assert [c['name'] for c in fixture_checks]==['fixture-fmt','fixture-tests','fixture-clippy']
for c in fixture_checks:
    assert c['exitCode']==0
    assert not any(bad in Path(c['log']).read_text() for bad in ['error:','FAILED','Traceback','AssertionError'])
fixture_tests=re.findall(r'^test (.+) \.\.\. ok$',Path(fixture_checks[1]['log']).read_text(),re.M)
assert len(fixture_tests)==50 and set(fixture_tests)<=set(tests)
explicit={}
for kind,count in [('native',2),('export',8)]:
    c=checks[kind+'-integration']; assert entry(c['worker']['path'])['sha256']==c['worker']['sha256']
    explicit[kind]=re.findall(r'^test (.+) \.\.\. ok$',Path(c['log']).read_text(),re.M)
    assert len(explicit[kind])==count
assert len(re.findall(r'^check contracts/generated/',Path(checks['schema-check']['log']).read_text(),re.M))==109
assert len(re.findall(r'^check .+\.ts$',Path(checks['types-check']['log']).read_text(),re.M))==109
tree=Path(checks['wasm-tree']['log']).read_text()
assert all(n not in tree for n in ['rusqlite','mo-standard-host','mo-native-render','mo-native-worker'])

ref_path=root/'reference-1/reference.json'; reference=json.loads(ref_path.read_text())
assert reference['schemaPairs']==10 and reference['oldAssets']==12 and reference['oldJobAndAssetsUnchanged']
for key in ['cli','worker','schemas','schemaParityProgram','schemaParityLog','oldDatabase','copy','export']:
    r=reference[key]; assert entry(r['path'])==r
for c in reference['calls']:
    assert c['exitCode']==0
    for k in ['stdin','stdout','stderr']: assert entry(c[k]['path'])==c[k]
export=json.loads(Path(reference['export']['path']).read_text())
assert len(export['calls'])==31 and export['publicAssets']==12 and export['pages']==2
assert len(export['migrations'])==2
for key in ['cli','worker','reference','database']:
    r=export[key]; assert entry(r['path'])==r
for c in export['calls']:
    assert c['exitCode']==0
    for key in ['stdin','stdout','stderr']:
        if key in c: assert entry(c[key]['path'])==c[key]
for m in export['migrations']:
    assert m['allRowsUnchanged']
    for key in ['before','after']: assert entry(m[key]['path'])==m[key]
independent=json.loads(Path(export['reference']['path']).read_text())
assert (len(independent['artifacts']),len(independent['pages']),len(independent['schemasChecked']),independent['nativeObjects'],independent['nativeTextRuns'])==(12,2,10,15,8)
for r in [*independent['artifacts'],independent['fixture'],independent['bundle'],independent['independentProgram']]:
    assert entry(r['path'])==r

parity_reference=json.loads((root/'parity-reference.json').read_text())
assert parity_reference['calls']=={'authored':17,'source':33,'editor':22}
assert parity_reference['pagePairs']==2 and parity_reference['allPreviousCasesUnchanged']
parity=json.loads(Path(parity_reference['current']['path']).read_text())
for key in ['previous','frozenProgram','replayProgram','current']:
    r=parity_reference[key]; assert entry(r['path'])==r
for r in parity['artifacts']+list(parity['reports'].values()): assert entry(r['path'])==r
for p in parity['pages']:
    for r in p.values(): assert entry(r['path'])==r
old_wasm=next(r for r in parent['artifacts'] if r['path']=='.codex-work/export-host/wasm-node/mo_wasm_bg.wasm')
assert entry(old_wasm['path'])==old_wasm
new_wasm=entry(root/'wasm-node/mo_wasm_bg.wasm')
size=dict(previousRustWasm=old_wasm,currentRustWasm=new_wasm,
    uncompressedByteDelta=new_wasm['byteLength']-old_wasm['byteLength'],
    previousGzip9Bytes=len(gzip.compress(Path(old_wasm['path']).read_bytes(),compresslevel=9,mtime=0)),
    currentGzip9Bytes=len(gzip.compress(Path(new_wasm['path']).read_bytes(),compresslevel=9,mtime=0)),
    scope='Rust WASM module raw and gzip-9 bytes; this excludes JS, graphics/shaping components, fonts, media and product runtime. Not an installer or complete-closure estimate.')
links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'): continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            dest=(p.parent/target).resolve(); assert dest.exists() or dest==output.resolve(),(name,target)
            links+=1
artifacts={str(p) for p in root.rglob('*') if p.is_file() and p.suffix not in ['.md','.pyc']}
for r in parity['reports'].values():
    suite=json.loads(Path(r['path']).read_text())
    if suite.get('artifactDirectory'): artifacts|={str(p) for p in Path(suite['artifactDirectory']).rglob('*') if p.is_file()}
artifacts|={r['path'] for r in parity['artifacts']}
artifacts|={reference['cli']['path'],reference['worker']['path'],str(parent_path)}
report=dict(format='musteroffice.operation-discovery-verification/1',previousEvidence=entry(parent_path),
    scope='Shared synchronous owner port and actual permission/configuration-aware operation discovery with identical Native/WASM schema documents; not complete Agent or Musterwork acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),removedSources=[],
    workspaceChecks=list(checks.values()),workspaceAttemptHistory=history,rustTestNames=tests,explicitIntegrationTests=explicit,
    finalFixtureChecks=fixture_checks,finalFixtureTestNames=fixture_tests,
    checks=dict(rustTests=826,newRustTests=5,explicitNativeIntegrationTests=2,explicitExportIntegrationTests=8,
        finalAffectedHostTests=50,
        schemas=109,newSchemas=2,changedPreviousSchemas=2,unchangedPreviousSchemas=105,
        strictClippy=True,typescript=True,pureOperationWasmCompile=True,liveSchemaNativeWasmPairs=10,
        discoveryAndOldDataCalls=len(reference['calls']),releaseExportCalls=31,oldV4PublicAssetsUnchanged=12,
        oldV3Migrations=2,newPublicAssets=12,pages=2,xsdParts=10,nativeObjects=15,nativeTextRuns=8,
        smallHostAssetBudgetBytes=96*1024,authoredCalls=17,sourceCalls=33,editorCalls=22,pageNativeWasmPairs=2,
        fullFeatureAcceptance=False,completeEmbeddedHost=False,completeWasmOwner=False,mcpAdapter=False,musterworkIntegration=False,localLinks=links),
    reports=dict(reference=entry(ref_path),parity=entry(root/'parity-reference.json'),fixtureValidation=entry(fixture_path)),wasmModuleSize=size,
    artifacts=[entry(p) for p in sorted(artifacts)],
    changesExplained=[
        'Routing moved out of SQLite StandardHost into the pure service. Existing standard storage/authority/job/commit methods implement a synchronous owner port; no second owner was introduced.',
        'Permissions and document profiles in discovery use the same operation catalogue as authorization. Renderer absence is unavailable, not unimplemented or accepted quality.',
        'Strict schema equality exposed missing runtime $id. Codegen and runtime now share schema identity instead of removing the assertion.',
        'Configured export limits now bound model/font output reservations before storage admission. A real 96 KiB asset-budget export verifies discovery and actual computation agree.',
        'WASM adds real schema discovery from the same Rust types. This does not create a browser task owner or imply complete WASM export.',
        'Parallel export integration exposed timestamp-only fixture directory collisions. Six host test suites now share exclusive allocation with a process-local atomic sequence; all affected ordinary tests and eight parallel real-worker exports were rerun. The earlier full 826-test suite predates this fixture-only change.',
    ],
    limitations=[
        'Full v0.4 and E0-E3 remain incomplete; the goal stays active.',
        'The operation catalogue lists 15 implemented draft service operations, not all native format features or complete product acceptance.',
        'OperationHost is synchronous and intended for a dedicated worker. Async browser/storage bridges and actual Musterwork task/Artifact integration still require implementation.',
        'MCP stdio/HTTP, Skill/Plugin, complete SDK/Viewer, independent third-party installation and product migration are not implemented by this stage.',
        'Capabilities are an advisory permission/configuration snapshot. Every operation must reauthorize; configured renderer availability does not prove target-application fidelity.',
        'Binary upload and range reads remain separate ports, not a JSON/base64 file channel. No public HTTP resource transport is claimed.',
        'Explicit queued run, failure/expiry behavior, missing retention/GC/scheduling and full process-budget limitations remain as recorded in the prior export stage.',
        'Public fixture remains two pages with owned synthetic glyphs and eight A A runs; no commercial visual, Office/WPS editing, playback, FPS or final package-size acceptance.',
        'Complete advanced objects, effects, transitions, media, source-model import and all originally required features remain in scope.',
    ])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv: assert output.read_text()==encoded,'sealed evidence changed'
else:
    with output.open('x') as f: f.write(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),checks=report['checks'],wasmModuleSize=size),ensure_ascii=False))
