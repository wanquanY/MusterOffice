"""Seal resource lifecycle, actual binary transport, migration and Writer use."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from urllib.parse import unquote

root=Path('.codex-work/resource-host')
previous=Path('docs/reviews/evidence/2026-09-26-operation-host-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-resource-host-verification.json')
def entry(path):
    path=Path(path);data=path.read_bytes()
    return dict(path=str(path),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
assert entry(previous)['sha256']=='8a1a625966c2d22f5210e63e758c7cf29f87e8270f582b29e64434dcecdc8a94'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''Cargo.lock
README.md
contracts/README.md
crates/mo-operation-service/src/contract.rs
crates/mo-operation-service/src/identity.rs
crates/mo-operation-service/src/lib.rs
crates/mo-standard-host/Cargo.toml
crates/mo-standard-host/src/db.rs
crates/mo-standard-host/src/lib.rs
docs/README.md
docs/design/agent-interfaces.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-contract-codegen/src/main.rs
tools/mo-host/Cargo.toml
tools/mo-host/src/main.rs'''.splitlines())
for stem in ['host-request','host-response','operation-job']:
    allowed|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
changed={p for p,r in prior.items() if entry(p)!=r}
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-operation-service/src/assets.rs
crates/mo-standard-host/src/assets.rs
crates/mo-standard-host/src/assets/bindings.rs
crates/mo-standard-host/src/assets/reader.rs
crates/mo-standard-host/src/assets/storage.rs
crates/mo-standard-host/src/assets/upload.rs
crates/mo-standard-host/tests/resources.rs
crates/mo-standard-host/tests/resource_recovery.rs
crates/mo-standard-host/examples/resource_pptx.rs
tools/mo-host/tests/resources.rs
docs/implementation/resource-host.md'''.splitlines())
added|={str(p) for p in Path('tools/verification').glob('resource-host-*') if p.is_file()}
for stem in ['asset-info','asset-binding','upload-request','upload-info']:
    added|={f'contracts/generated/{stem}.schema.json',f'packages/contracts/src/generated/{stem}.ts'}
    added|={str(p) for p in Path(f'packages/contracts/src/generated/{stem}').glob('*.ts')}
assert not added&set(prior)
sources=set(prior)|added
for name in sources:
    path=Path(name)
    if path.suffix in ['.rs','.ts','.py','.mjs','.cpp','.h']:assert len(path.read_text().splitlines())<=2000,name
checks=json.loads((root/'workspace.json').read_text())
assert [c['name'] for c in checks]==['fmt','tests','clippy','native-build','host-release','schema-check','types-check','pure-service-wasm','host-tree','wasm-tree']
assert all(c['exitCode']==0 for c in checks)
for c in checks:assert not any(s in Path(c['log']).read_text() for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
assert len(tests)==779 and set(old['rustTestNames'])<=set(tests)
for name in ['killed_verifier_is_never_published_and_expiry_releases_staging','native_pptx_writer_consumes_scope_authorized_range_readers','chunked_cli_upload_restart_seal_and_binary_range_read','failure_on_final_receipt_update_cannot_publish_an_asset','hashing_releases_write_lock_and_cancel_wins_before_publication','metadata_query_preserves_failed_upload_state_without_claiming_usable_bytes']:
    assert name in tests
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==104
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==104
assert 'rusqlite' not in (root/'wasm-tree.log').read_text()
assert 'mo-standard-host' not in (root/'wasm-tree.log').read_text()
assert 'libsqlite3-sys v0.38.2' in (root/'host-tree.log').read_text()
# Check the recorded SQLite component archive pins. Additional workspace edges
# use the existing root dependency declarations, which remain hash-unchanged.
current_packages={(p['name'],p['version']):p['checksum'] for p in tomllib.loads(Path('Cargo.lock').read_text())['package'] if 'checksum' in p}
parent_lock=next(a for a in old['artifacts'] if a['path'].endswith('host-dependency-tree.txt'))
assert entry(parent_lock['path'])==parent_lock
component=json.loads(Path('components/sqlite-host/component.json').read_text())
for p in component['additionalLockedPackages']:assert current_packages[(p['name'],p['version'])]==p['sha256']
release=json.loads((root/'release-cli.json').read_text())
pptx=json.loads((root/'pptx-reference.json').read_text())
assert release['migrationPreservedRows'] and release['processInvocations']==len(release['calls'])
assert release['processInvocations']==21
assert release['scopeBytes']==2_097_883 and release['chunks']==9 and release['maxChunkBytes']==262144
for key in ['binary','database','source','download','upgradedDatabase','priorDatabase']:assert entry(release[key]['path'])==release[key]
assert release['source']['sha256']==release['download']['sha256']
assert pptx['mediaBytesMatch'] and pptx['baselineByteIdentical'] and pptx['zipCrc'] and pptx['xmlWellFormed']
for key in ['pptx','inlineBaseline','producer','sourceRequest','sourceResources']:assert entry(pptx[key]['path'])==pptx[key]
assert pptx['pptx']['sha256']==pptx['inlineBaseline']['sha256']
assert pptx['slides']==2 and pptx['nativeObjects']==15
current={key:entry(path) for key,path in {'nativeHost':'target/debug/mo-host','releaseHost':'target/release/mo-host'}.items()}
unchanged={key:entry(old['currentArtifacts'][key]['path']) for key in ['nativeCli','rustWasm','rustWasmGlue']}
for key,record in unchanged.items():assert record==old['currentArtifacts'][key]
links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md':continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:assert (p.parent/target).exists() or (p.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[entry(p) for p in sorted(root.rglob('*')) if p.is_file() and p.name not in ['plan.md','next.md','seal.log','seal-check.log']]
report=dict(format='musteroffice.resource-host-verification/1',previousEvidence=entry(previous),scope='Scope-authorized persistent binary input resources, short chunk transactions, declared byte reservations, hash sealing, immutable range readers, real CLI and existing native Writer integration. Not export job/public bundle or Musterwork replacement acceptance.',sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),removedSources=[],workspaceChecks=checks,rustTestNames=tests,
    checks=dict(rustTests=len(tests),newRustTests=16,schemas=104,addedSchemas=4,modifiedSchemas=3,existingKernelSchemasUnchanged=True,typescript=True,strictClippy=True,pureServiceWasmCompile=True,sqliteMigrationVersion=2,previousDatabaseRowsPreserved=True,previousDatabaseBytesUnchanged=True,releaseCliInvocations=release['processInvocations'],resourceBytes=release['scopeBytes'],resourceChunks=9,maxStoredChunkBytes=262144,realVerifierProcessKilled=True,finalAssetReceiptRollback=True,pptxSlides=pptx['slides'],pptxNativeObjects=pptx['nativeObjects'],pptxBytesMatchInlineProvider=True,officeWpsNewAcceptance=False,localLinks=links),
    reports={name:entry(root/(name+'.json')) for name in ['release-cli','pptx-reference']},artifacts=artifacts,currentArtifacts=current,unchangedKernelArtifacts=unchanged,
    attempts=[dict(reason='Initial production check used usize as a SQLite parameter; converted the already bounded chunk length to signed SQLite integer.',log=entry(root/'initial-check.log')),dict(reason='Initial test helpers requested u64 from SQLite; used the database signed integer representation without altering production limits.',log=entry(root/'initial-tests.log')),dict(reason='Initial strict Clippy flagged a collapsible conditional; corrected before final full tests/checks. Failed-upload metadata query and its test/example also completed before final run.',log=entry(root/'attempts/pre-final/clippy.log')),dict(reason='First seal detected that the Python verification script hashed an upgraded database before closing its connection and WAL checkpoint. Explicitly close all inspector connections, preserve first reports and repeat actual release commands. Independent object inventory now separately counts 13 slide objects and 2 master/layout objects.',log=entry(root/'attempts/pre-seal/initial-seal.log'))],
    size=dict(nativeHostReleaseBytes=current['releaseHost']['byteLength'],previousNativeHostReleaseBytes=old['size']['nativeHostReleaseBytes'],rustWasmBytes=unchanged['rustWasm']['byteLength'],cppWasmBytes=old['size']['cppWasmBytes'],note='Independent native operation/resource host only. No full render/font/media/runtime closure or installer size estimate.'),
    limitations=['Full v0.4/P02/P07/Musterwork goal remains incomplete; this is resource authority and bytes, not complete delivery.', 'Only SHA-256 integrity verified at registration; MIME is declared. Decoder, license, purpose and media capability checks remain consumer responsibilities.', 'Resource byte reservations exclude SQLite pages/indexes/WAL/metadata and deleted-space reclamation. No complete storage, RSS or throughput budget acceptance.', 'No automatic sealed-resource deletion or deduplication; references/retention must precede release. Expired receipts remain idempotent and count toward quota.', 'Hash verification freeze has a finite lease. Expiry fences publication, but does not establish physical process termination or power-loss validation.', 'Library Writer consumes bounded resource readers; authored output still uses existing Vec. No unified export job, spool/sealed-result validation or public bundle yet.', 'Actual standard host/CLI execution validated on current native environment. Pure operation types compile for WASM, SQLite is excluded; this is not a browser storage adapter.', 'MCP/SDK/Skill/Plugin and actual Musterwork Artifact/Viewer/history integration, all advanced objects and full playback still required.', 'Prior editor/renderer production sources and recorded core modules unchanged; no new full rendering replay or Office/WPS acceptance.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),checks=report['checks'],size=report['size'])))
