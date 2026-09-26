"""Seal durable-operation implementation, real process tests and core regression."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote

root = Path('.codex-work/operation-host')
previous = Path('docs/reviews/evidence/2026-09-26-retained-timing-verification.json')
output = Path('docs/reviews/evidence/2026-09-26-operation-host-verification.json')
def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())
assert entry(previous)['sha256'] == 'e9408007e7d4d7a28bf44f454acbb969e0d9371f8ba7cc896f9c4a91e5ee1ff6'
old = json.loads(previous.read_text())
prior = {r['path']: r for r in old['sourceFiles']}
allowed = set('''Cargo.toml
Cargo.lock
README.md
contracts/README.md
crates/mo-presentation-edit/src/transaction.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-contract-codegen/Cargo.toml
tools/mo-contract-codegen/src/main.rs'''.splitlines())
changed = {p for p, record in prior.items() if entry(p) != record}
assert changed == allowed, (changed-allowed, allowed-changed)
added = {'docs/implementation/operation-host.md', 'docs/design/agent-interfaces.md'}
for directory in ['crates/mo-operation-service', 'crates/mo-standard-host', 'tools/mo-host', 'components/sqlite-host']:
    added |= {str(p) for p in Path(directory).rglob('*') if p.is_file()}
added |= {str(p) for p in Path('tools/verification').glob('operation-host-*') if p.is_file()}
for stem in ['host-request', 'host-response', 'operation-request', 'operation-job']:
    added |= {f'contracts/generated/{stem}.schema.json', f'packages/contracts/src/generated/{stem}.ts'}
    added |= {str(p) for p in Path(f'packages/contracts/src/generated/{stem}').glob('*.ts')}
assert not added & set(prior)
sources = set(prior) | added
for name in sources:
    path = Path(name)
    if path.suffix in ['.rs', '.ts', '.py', '.mjs', '.cpp', '.h']:
        assert len(path.read_text().splitlines()) <= 2000, name
checks = json.loads((root/'workspace.json').read_text())
assert [c['name'] for c in checks] == ['fmt', 'tests', 'clippy', 'native-build', 'host-release', 'schema-check', 'types-check', 'pure-service-wasm', 'rust-wasm', 'bindgen', 'sqlite-build']
assert all(c['exitCode'] == 0 for c in checks)
for c in checks:
    assert not any(s in Path(c['log']).read_text() for s in ['error:', 'FAILED', 'Traceback', 'AssertionError']), c['name']
test_log = (root/'tests.log').read_text()
tests = re.findall(r'^test (.+) \.\.\. ok$', test_log, re.M)
assert len(tests) == 763
assert set(old['rustTestNames']) <= set(tests)
assert 'process_worker ... ignored, invoked as a disposable child' in test_log
for required in ['killed_worker_leaves_a_recoverable_durable_lease_and_no_revision', 'real_cli_persists_queued_work_edits_queries_and_retry_across_processes', 'write_failure_rolls_back_snapshot_head_and_terminal_receipt_together', 'relabelled_base_content_cannot_be_committed_under_a_known_revision', 'simultaneous_edits_commit_exactly_one_revision', 'cancellation_and_commit_race_has_one_durable_winner']:
    assert required in tests
assert len(re.findall(r'^check contracts/generated/', (root/'schema-check.log').read_text(), re.M)) == 100
assert len(re.findall(r'^check .+\.ts$', (root/'types-check.log').read_text(), re.M)) == 100
lines = [s[len('SQLITE_BUILD_RECORD='):] for s in (root/'sqlite-build.log').read_text().splitlines() if s.startswith('SQLITE_BUILD_RECORD=')]
assert len(lines) == 1
sqlite_build = json.loads(lines[0])
assert sqlite_build['version'] == '3.53.2' and sqlite_build['journalMode'] == 'wal'
assert sqlite_build['synchronous'] == 'FULL' and sqlite_build['fullFsync']
component = json.loads(Path('components/sqlite-host/component.json').read_text())
assert len(component['additionalLockedPackages']) == 10
for package in component['additionalLockedPackages']:
    assert entry(package['notice']['path']) == package['notice']
    assert package['selectedLicense'] == 'MIT'
wasm_tree = (root/'wasm-dependency-tree.txt').read_text()
assert 'rusqlite' not in wasm_tree and 'libsqlite3-sys' not in wasm_tree
host_tree = (root/'host-dependency-tree.txt').read_text()
assert 'rusqlite v0.40.2' in host_tree and 'libsqlite3-sys v0.38.2' in host_tree
regression = json.loads((root/'edit-regression.json').read_text())
parity = json.loads((root/'core-parity.json').read_text())
release = json.loads((root/'release-cli.json').read_text())
assert release['processInvocations'] == len(release['cases']) == 9
assert release['counts'] == dict(jobs=2, revisions=2, heads=1)
for key in ['binary', 'database']:
    assert entry(release[key]['path']) == release[key]
assert regression['previousResponsesByteIdentical']
assert regression['requests'] == parity['passed'] == len(parity['cases'])
for key in ['program', 'coreParity', 'parentWasm', 'currentWasm']:
    assert entry(regression[key]['path']) == regression[key]
for key in ['rustWasm', 'rustWasmGlue', 'cppWasm', 'typescriptRaster']:
    assert entry(old['currentArtifacts'][key]['path']) == old['currentArtifacts'][key]
current = {key: entry(path) for key, path in {'nativeCli':'target/debug/mo-cli', 'nativeHost':'target/debug/mo-host', 'releaseHost':'target/release/mo-host', 'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'), 'rustWasmGlue':str(root/'wasm-node/mo_wasm.js')}.items()}
links = 0
for name in sources:
    path = Path(name)
    if path.suffix != '.md':
        continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            assert (path.parent/target).exists() or (path.parent/target).resolve() == output.resolve(), (name, target)
            links += 1
subprocess.run(['git', 'diff', '--check'], check=True)
artifacts = [entry(p) for p in sorted(root.rglob('*')) if p.is_file() and p.name not in ['plan.md', 'next.md', 'seal.log', 'seal-check.log']]
report = dict(
    format='musteroffice.operation-host-verification/1', previousEvidence=entry(previous),
    scope='Persistent create/read/apply for the existing author model through shared pure computation, SQLite StandardHost and real CLI processes. Not full P07/E0-6, MCP or Musterwork replacement acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added), removedSources=[],
    newlyTrackedExistingSources=['docs/design/agent-interfaces.md'],
    workspaceChecks=checks, rustTestNames=tests,
    checks=dict(rustTests=len(tests), newRustTests=25, testOnlySubprocessHelper=1, schemas=100, addedSchemas=4, existingSchemasUnchanged=True, typescript=True, strictClippy=True, pureServiceWasmCompile=True, legacyEditNativeWasmCalls=parity['passed'], legacyEditResponsesByteIdentical=True, realCliRestarts=True, releaseCliInvocations=9, independentDatabaseIntegrity=True, killedComputingProcess=True, failedSqlTransactionRollback=True, physicalPowerLossTested=False, officeWpsNewAcceptance=False, localLinks=links),
    sqliteBuild=sqlite_build, component=entry('components/sqlite-host/component.json'),
    dependencyTrees={name:entry(root/(name+'-dependency-tree.txt')) for name in ['host','wasm']},
    reports={name:entry(root/(name+'.json')) for name in ['edit-regression','core-parity','release-cli']},
    initialAttempt=dict(reason='Initial locked check detected manifest/lock mismatch after adding a workspace dependency; lock regenerated offline before final locked validation.', log=entry(root/'check.log')),
    evidenceAttempt=dict(reason='First seal check incorrectly treated the existing Agent design document as tracked by the previous source manifest. It is now explicitly recorded as a newly tracked existing file; no previous evidence or source hashes were relaxed.', log=entry(root/'attempts/seal-source-inventory.log')),
    artifacts=artifacts, currentArtifacts=current,
    size=dict(nativeHostReleaseBytes=current['releaseHost']['byteLength'], rustWasmBytes=current['rustWasm']['byteLength'], previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'], cppWasmBytes=old['currentArtifacts']['cppWasm']['byteLength'], note='Standalone author-operation host binary and raw modules only; no complete font/media/render/runtime closure or desktop installer.'),
    limitations=['Full phase-one goal remains active; advanced objects, full effects/transitions/media and Office/WPS acceptance remain incomplete.', 'No resource bytes, import/export/render bundle or quality-report submission in this operation service yet.', 'No automatic scheduler, hard response deadline, worker termination or physical power-loss validation. Expiry fences results but does not prove old process memory is released.', 'No MCP stdio/HTTP endpoint, SDK runtime, Skill/Plugin package, Musterwork Artifact/Viewer/history adapter or E0-6 acceptance.', 'Finite request/snapshot and count quotas are not complete storage-byte or retained-resource budgets; no automatic receipt cleanup.', 'SQLite is only in StandardHost; EmbeddedHost must reuse its product owner. Upstream bundled feature set has not been size-trimmed.', 'Current native environment only for host process/durability tests; pure service compiles for WASM, while paired runtime evidence covers the existing editor API.', 'No new performance benchmark or product installer estimate. Previous renderer evidence is referenced without claiming a new full rendering replay.'])
encoded = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == encoded, 'sealed evidence changed'
else:
    assert not output.exists(), 'refuse to overwrite sealed evidence'
    output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output), sourceFiles=len(sources), checks=report['checks'], size=report['size'])))
