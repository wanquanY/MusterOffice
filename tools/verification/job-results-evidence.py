"""Bind private output ownership to sources, real storage and scoped evidence."""
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/job-results')
parent_path = Path('docs/reviews/evidence/2026-09-26-sealed-export-verification.json')
output = Path('docs/reviews/evidence/2026-09-26-job-results-verification.json')

def entry(path):
    p = Path(path)
    data = p.read_bytes()
    return dict(path=str(p), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

assert entry(parent_path)['sha256'] == '1cdbed3b46bbfaa59d5ca4670a91ca932bf0bd8f1e125ab0ade38e521caa205a'
parent = json.loads(parent_path.read_text())
prior = {r['path']: r for r in parent['sourceFiles']}
allowed = set('''README.md
crates/mo-standard-host/src/assets.rs
crates/mo-standard-host/src/assets/storage.rs
crates/mo-standard-host/src/assets/upload.rs
crates/mo-standard-host/src/db.rs
crates/mo-standard-host/src/jobs.rs
crates/mo-standard-host/src/lib.rs
crates/mo-standard-host/tests/resource_recovery.rs
docs/README.md
docs/design/agent-interfaces.md
docs/implementation/development.md
docs/implementation/progress.md'''.splitlines())
changed = {p for p, r in prior.items() if entry(p) != r}
assert changed == allowed, (changed-allowed, allowed-changed)
added = set('''crates/mo-standard-host/examples/job_output.rs
crates/mo-standard-host/src/results.rs
crates/mo-standard-host/src/results/reader.rs
crates/mo-standard-host/src/results/sink.rs
crates/mo-standard-host/src/results/storage.rs
crates/mo-standard-host/tests/results.rs
docs/implementation/job-results.md
tools/verification/job-results-workspace.py
tools/verification/job-results-reference.py
tools/verification/job-results-parity.mjs
tools/verification/job-results-evidence.py'''.splitlines())
assert not added & set(prior)
sources = set(prior) | added
for p in sources:
    if Path(p).suffix in ['.rs', '.py', '.mjs', '.ts', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
for p, r in prior.items():
    if p.startswith(('contracts/generated/', 'packages/contracts/src/generated/')):
        assert entry(p) == r

checks = json.loads((root/'workspace.json').read_text())
assert len(checks) == 11 and all(c['exitCode'] == 0 for c in checks)
for c in checks:
    assert not any(bad in Path(c['log']).read_text() for bad in ['error:', 'FAILED', 'Traceback', 'AssertionError']), c['name']
tests = re.findall(r'^test (.+) \.\.\. ok$', (root/'tests.log').read_text(), re.M)
assert len(tests) == 810 and set(parent['rustTestNames']) <= set(tests)
assert len(set(tests)-set(parent['rustTestNames'])) == 14
for name in ['killed_writer_and_sealer_leave_private_recoverable_outputs',
             'real_pptx_streams_from_persistent_inputs_into_verified_private_output',
             'storage_failure_rolls_back_job_and_output_cleanup_together',
             'seal_releases_write_lock_and_cancellation_wins_during_hashing',
             'scope_reservation_is_shared_with_uploads_and_racing_outputs']:
    assert name in tests
assert len(re.findall(r'^check contracts/generated/', (root/'schema-check.log').read_text(), re.M)) == 104
assert len(re.findall(r'^check .+\.ts$', (root/'types-check.log').read_text(), re.M)) == 104
assert 'mo-standard-host' not in (root/'wasm-tree.log').read_text()
assert 'rusqlite' not in (root/'wasm-tree.log').read_text()
assert 'rusqlite' in (root/'host-tree.log').read_text()
unchanged = {k: parent['currentArtifacts'][k] for k in ['releaseCli', 'rustWasm', 'rustWasmGlue']}
for r in unchanged.values():
    assert entry(r['path']) == r

reference = json.loads((root/'reference.json').read_text())
for key in ['privateCandidateMatchesPriorFile', 'persistedChunkBytesMatch', 'noPublicAssetOrJobSuccess',
            'expiryReleasesCandidate', 'twoV2MigrationsPreserveRows', 'zipCrc']:
    assert reference[key]
assert len(reference['calls']) == 3 and reference['slides'] == 2
artifacts = {c['log'] for c in checks} | {str(root/'workspace.json'), str(root/'reference.json'), str(root/'reference-run.log')}
for r in reference['inputs'] + reference['outputs'] + reference['programs']:
    assert entry(r['path']) == r
    artifacts.add(r['path'])
for c in reference['calls']:
    assert c['exitCode'] == 0
    for key in ['stdout', 'stderr']:
        r = c[key]
        assert entry(r['path']) == r
        artifacts.add(r['path'])
attempts = [entry(p) for p in sorted(root.glob('*-attempt-*.log'))]
assert len(attempts) == 4
artifacts |= {r['path'] for r in attempts}
reference_attempt = [entry(p) for p in sorted((root/'attempts/reference-v1').rglob('*')) if p.is_file()]
assert reference_attempt and 'KeyError' in (root/'attempts/reference-v1/reference-run.log').read_text()
artifacts |= {r['path'] for r in reference_attempt}
evidence_attempt = [entry(p) for p in sorted((root/'attempts/evidence-v1').rglob('*')) if p.is_file()]
artifacts |= {r['path'] for r in evidence_attempt}
parity = json.loads((root/'parity.json').read_text())
assert parity['authoredCases'] == 17 and parity['sourceCases'] == 33 and parity['editorCases'] == 22
for r in [parity['native'], parity['wasm'], *parity['reports'].values()]:
    assert entry(r['path']) == r
    artifacts.add(r['path'])
for r in parity['reports'].values():
    suite = json.loads(Path(r['path']).read_text())
    if suite.get('artifactDirectory'):
        artifacts |= {str(p) for p in Path(suite['artifactDirectory']).rglob('*') if p.is_file()}
artifacts.add(str(root/'parity.json'))
artifacts |= {str(p) for p in root.glob('*-parity.stderr')}

links = 0
for name in sources:
    p = Path(name)
    if p.suffix != '.md':
        continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            dest = (p.parent/target).resolve()
            assert dest.exists() or dest == output.resolve(), (name, target)
            links += 1

current = {k: entry(p) for k, p in dict(nativeCli='target/debug/mo-cli', nativeHost='target/debug/mo-host', releaseHost='target/release/mo-host',
                                      privateOutputExample='target/release/examples/job_output').items()}
report = dict(format='musteroffice.job-results-verification/1', previousEvidence=entry(parent_path),
    scope='Private job/fence/request/executor-bound output chunks, shared input/output quota, actual-byte sealing and transactional lifecycle. Not public export job, complete delivery or Musterwork replacement.',
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added), removedSources=[],
    workspaceChecks=checks, rustTestNames=tests,
    checks=dict(rustTests=len(tests), newRustTests=14, killedProcessPhases=['Writing', 'Sealing'],
                schemas=104, wireContractsUnchanged=True, typescript=True, strictClippy=True,
                authoredCalls=17, sourceCalls=33, editorCalls=22,
                releaseProcessInvocations=3, realV2Migrations=2, privatePptxByteIdentical=True,
                publicOutputNotPublished=True, localLinks=links, officeWpsNewAcceptance=False),
    reports=dict(reference=entry(root/'reference.json'), parity=entry(root/'parity.json')), artifacts=[entry(p) for p in sorted(artifacts)],
    currentArtifacts=current, unchangedKernelArtifacts=unchanged, attempts=attempts,
    referenceAttempt=dict(artifacts=reference_attempt, reason='Independent verifier initially expected a failed job in the successful response envelope. Corrected to the existing Failed envelope; production unchanged; entire run repeated from fresh copies.'),
    artifactAudit=dict(artifacts=evidence_attempt, note='Debug CLI hash differed from the frozen parent after workspace verification. Recorded as current artifact and replayed 72 actual public calls against unchanged WASM; did not assert byte-identical debug executable.'),
    limitations=['Full v0.4, E0-E3 and Musterwork replacement remain incomplete and active.',
                 'Only private worker result storage is implemented; public export action, complete bundle, quality claims and atomic public success commit remain required.',
                 'Example uses a running mutation lease to validate storage; it never reports that mutation or an export as succeeded.',
                 'Scope bytes exclude SQLite physical pages/WAL/indexes, RSS and whole-machine storage limits; cleanup can delete many chunks in one transaction and has no worst-case latency claim.',
                 'Expired outputs are reclaimed on admission or task state access, with bounded record scanning; no autonomous background sweeper is implemented.',
                 'Drop cleanup is best effort; execution leases cover retained candidates and killed workers. No transparent resumed writer, process supervisor or power-loss guarantee.',
                 'No new renderer, Native/WASM pixel replay, Office/WPS or product acceptance. Unchanged kernel sources/artifacts preserve their prior scoped evidence.',
                 'No new performance or installation-size estimate. Complete effects/animation/transitions/media/SmartArt/equations and Agent/Musterwork adapters remain required.'])
encoded = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == encoded, 'sealed evidence changed'
else:
    with output.open('x') as stream:
        stream.write(encoded)
print(json.dumps(dict(evidence=entry(output), sourceFiles=len(sources), checks=report['checks']), ensure_ascii=False))
