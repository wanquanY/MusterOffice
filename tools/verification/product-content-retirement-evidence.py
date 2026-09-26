"""Bind durable retirement to real product tests, crashes and old databases."""
import argparse
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import unquote

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
product = args.product_repo.resolve()
stage = Path('.codex-work/product-content-retirement')
output = Path('docs/reviews/evidence/2026-09-26-product-content-retirement-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-product-content-lifecycle-verification.json')


def read(path):
    return json.loads(Path(path).read_text())


def entry(path, base=Path('.')):
    path = Path(path)
    digest = hashlib.sha256()
    length = 0
    with path.open('rb') as stream:
        while data := stream.read(1024 * 1024):
            length += len(data)
            digest.update(data)
    return dict(path=path.relative_to(base).as_posix(), byteLength=length, sha256=digest.hexdigest())


assert entry(parent_path)['sha256'] == '6246396208d698f4dec391fab11ae7a02bf2dbb1d0567528e271a1ac4a943a7e'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {p for p, record in prior.items() if entry(p) != record}
assert changed == {'docs/README.md', 'docs/implementation/progress.md'}, changed
added = {'docs/implementation/product-content-retirement.md',
         'tools/verification/product-content-retirement-evidence.py',
         'tools/verification/product-content-retirement-migration.py'}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs', '.py', '.mjs', '.ts', '.cpp', '.h']:
        assert len(path.read_text().splitlines()) <= 2000, name
    if path.suffix == '.md':
        for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text()):
            if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
                continue
            target = unquote(target.split('#')[0].split('?')[0])
            if target:
                resolved = (path.parent / target).resolve()
                assert resolved.exists() or resolved == output.resolve(), (name, target)
                links += 1

baseline = read(stage / 'product-baseline.json')
previous_product = {r['path']: r for r in parent['productPrivateFiles']}
for record in baseline['files']:
    if record['path'] in previous_product:
        assert record == previous_product[record['path']]
current = read(stage / 'product-source-3.json')
assert len(current['files']) == 34
for record in current['files']:
    assert entry(product / record['path'], product) == record, record['path']
    if record['path'].endswith('.rs'):
        assert len((product / record['path']).read_text().splitlines()) <= 2000
baseline_files = {r['path']: r for r in baseline['files']}
product_changes = sorted(r['path'] for r in current['files']
                         if r['path'] in baseline_files and r != baseline_files[r['path']])
prefix = 'apps/agent-runtime/crates/infrastructure/sqlite/src/'
assert product_changes == sorted(prefix + p for p in [
    'content/material.rs', 'content/material/fs.rs', 'content/material/tests.rs',
    'content/material/tests/cancellation.rs', 'lib.rs', 'schema.rs', 'schema_collaboration.rs'])
for name in ['embedded-sdk', 'product-office-manifest']:
    previous = read(f'docs/reviews/evidence/2026-09-26-{name}-verification.json')
    for record in previous['productPrivateFiles']:
        if record['path'] == 'apps/agent-runtime/crates/runtime/artifact/src/lib.rs':
            data = (product / record['path']).read_bytes()
            addition = b'mod projected_application;\npub use projected_application::ProjectedArtifactApplication;\n'
            assert data.count(addition) == 1
            original = data.replace(addition, b'')
            assert len(original) == record['byteLength'] and hashlib.sha256(original).hexdigest() == record['sha256']
        else:
            assert entry(product / record['path'], product) == record, record['path']

environment = read(stage / 'environment.json')
previous_environment = read(parent['environment']['path'])
for key, root in [('productLocks', product), ('kernelLocks', Path('.'))]:
    assert environment[key] == previous_environment[key]
    for record in environment[key]:
        assert entry(root / record['path'], root) == record

log = (stage / 'tests-3.log').read_text()
names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
assert len(names) == 48 and 'FAILED' not in log and '48 passed; 0 failed; 3 ignored' in log
executables = re.findall(r'Running unittests src/lib.rs \((target/debug/deps/[^)]+)\)', log)
assert len(executables) == 1
executable = entry(product / 'apps/agent-runtime' / executables[0], product)
for required in [
    'actual_process_kill_recovers_registration_partial_and_durable_write_boundaries',
    'actual_process_kill_recovers_tombstone_unlink_and_uncommitted_dequeue',
    'failed_delete_is_durable_and_idle_maintenance_ignores_vacuum_threshold',
    'admission_release_reenqueues_retained_bytes_and_rollback_keeps_claim',
    'retained_oldest_entry_does_not_starve_bounded_batches_or_targeted_delete',
    'unlink_failure_keeps_durable_intent_for_retry',
    'cancelled_retirement_holds_registry_and_replays_missing_file_before_discharge',
    'retirement_migration_backfills_distinct_tombstones_and_rolls_back_late_failure',
    'reopening_version_182_preserves_existing_metadata_and_blob_bytes',
]:
    assert any(name.endswith('::' + required) for name in names), required
clippy = (stage / 'clippy.log').read_text()
assert 'Finished `dev` profile' in clippy and 'error:' not in clippy and 'warning:' not in clippy
assert not (stage / 'fmt.log').read_bytes()
write_crashes = read(stage / 'write-crashes-3.json')
assert [r['phase'] for r in write_crashes] == ['unregistered', 'partial', 'persisted', 'published']
for row in write_crashes:
    assert row['confirmedLiveBeforeKill'] and row['terminatedUnsuccessfully']
    assert row['journalAfter'] == 0 and row['namespaceAfter'] == ['registry.lock']
    assert row['publishedPreserved'] == (row['phase'] == 'published')
retirement_crashes = read(stage / 'retirement-crashes-3.json')
assert [r['phase'] for r in retirement_crashes] == ['tombstoned', 'unlinked', 'dequeued', 'completed']
for row in retirement_crashes:
    assert row['confirmedLiveBeforeKill'] and row['terminatedUnsuccessfully'] and row['tombstonePreserved']
    assert row['blobBefore'] == (row['phase'] == 'tombstoned') and not row['blobAfter']
    assert row['durableRetirementsBefore'] == int(row['phase'] != 'completed')
    assert row['durableRetirementsAfter'] == 0
migration = read(stage / 'migration/report.json')
assert migration['currentExecutable'] == executable
assert entry(product / migration['priorExecutable']['path'], product) == migration['priorExecutable']
assert migration['priorExecutable'] == baseline['frozenPriorExecutable']
assert [r['mode'] for r in migration['observations']] == ['published', 'tombstoned']
for row in migration['observations']:
    assert row['fromVersion'] == 183 and row['toVersion'] == 184
    assert row['confirmedLiveBeforeKill'] and row['actuallyKilled'] and row['databaseCreatedByPriorBinary']
    assert row['publishedPreserved'] == (row['mode'] == 'published')
    assert row['independentTombstoneBackfill'] == row['tombstonedBlobReclaimed'] == (row['mode'] == 'tombstoned')
    assert row['durableRetirementsAfter'] == 0
    upgrade_log = (stage / f"migration/{row['mode']}-current.log").read_text()
    assert '1 passed; 0 failed' in upgrade_log and 'FAILED' not in upgrade_log

files = read(stage / 'file-verification.json')
assert files['actualFiles'] == 13 and files['assets'] == 12 and files['pages'] == 2
assert files['allBytesUnchanged'] and files['finalStoredInspectionUnchanged']
for key in ['cli', 'request']:
    assert entry(files[key]['path']) == files[key]
for record in files['files']:
    assert entry(record['path']) == record
    previous = Path('.codex-work/product-content-lifecycle/actual-current') / Path(record['path']).name
    assert Path(record['path']).read_bytes() == previous.read_bytes()
assert read(stage / 'inspected.json') == read('.codex-work/product-content-lifecycle/inspected-current.json')

report = dict(
    format='musteroffice.product-content-retirement-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), localLinksChecked=links, kernelComputingSourcesUnchanged=True,
    productBaseline=entry(stage / 'product-baseline.json'), productHeadObserved=environment['productHeadObserved'],
    productPrivateFiles=current['files'], changedProductSources=product_changes,
    productTestExecutable=executable, environment=entry(stage / 'environment.json'),
    productAndKernelLockBytesUnchanged=True, priorProductSdkAndReaderImplementationsUnchanged=True,
    tests=dict(sqliteSelected=48, externalPriorVersionUpgrades=2, ignoredHarnessEntrypoints=3,
               actuallyKilledChildren=10, names=names, strictSqliteProductionClippy=True, sqliteWholeSuite=False),
    migration=dict(fromVersion=183, toVersion=184, previousBinary=migration['priorExecutable'],
                   actualDatabases=2, independentTombstoneBackfill=True, lateFailureRollback=True),
    retirement=dict(maxHashesPerBatch=64, sameTransactionAsClaimRelease=True, retainedClaimsReenqueuedOnRelease=True,
                    boundedDirectDelete=True, existingIdleMaintenance=True, independentOfVacuumThreshold=True,
                    indexedAdmissionLookup=True, queuedUnlinkLease=True, syncEvenWhenAlreadyMissing=True,
                    fullBusinessRetention=False, perJobQuota=False),
    actualFiles=13, actualAssets=12, actualPages=2, sharedInspectionUnchanged=True,
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'Real crash and filesystem execution is macOS arm64 only; no power-cut, kernel-crash or Windows runtime acceptance.',
        'Only released known physical claims are reclaimed. Unknown old CAS, external staging and unreferenced READY business retention remain open.',
        '64 bounds retirement hashes per pass, not physical write recovery, database history size or total disk quota.',
        'No new timer or task authority. Active Runtime runs defer idle maintenance; startup, write, delete and explicit recovery remain available.',
        'Full Invocation quotas, cloud streaming, immutable range-read integration, native Runtime producer, fenced Artifact commit and Viewer/Player remain open.',
        'No new kernel Native/WASM/worker build, full product test, resource benchmark or installer claim.',
        'Full advanced content, Office/WPS editing, history migration, distribution and E0-E3 remain open.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(current['files']), links=links,
                     tests=48, externalUpgrades=2, hardKills=10, actualFiles=13, evidence=entry(output))))
