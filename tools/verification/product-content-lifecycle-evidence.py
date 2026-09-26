"""Bind physical-content recovery evidence without importing product source."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import unquote

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', required=True, type=Path)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
product = args.product_repo.resolve()
stage = Path('.codex-work/product-content-lifecycle')
output = Path('docs/reviews/evidence/2026-09-26-product-content-lifecycle-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-product-content-stream-verification.json')


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


assert entry(parent_path)['sha256'] == 'cbd7cb127b165e36ce79ac63191bc0eac1c5cab8112a6a9963f88e633a878fd3'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {p for p, record in prior.items() if entry(p) != record}
assert changed == {'docs/README.md', 'docs/implementation/progress.md', 'docs/implementation/dependencies.md'}, changed
added = {'docs/implementation/product-content-lifecycle.md',
         'tools/verification/product-content-lifecycle-evidence.py'}
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
    else:
        data = subprocess.check_output(['git', 'show', f"{baseline['head']}:{record['path']}"], cwd=product)
        assert len(data) == record['byteLength'] and hashlib.sha256(data).hexdigest() == record['sha256']
current = read(stage / 'product-verified-files.json')
assert len(current['files']) == 28
for record in current['files']:
    assert entry(product / record['path'], product) == record, record['path']
    if record['path'].endswith('.rs'):
        assert len((product / record['path']).read_text().splitlines()) <= 2000, record['path']
for name in ['embedded-sdk', 'product-office-manifest']:
    previous = read(f'docs/reviews/evidence/2026-09-26-{name}-verification.json')
    for record in previous['productPrivateFiles']:
        if record['path'] == 'apps/agent-runtime/crates/runtime/artifact/src/lib.rs':
            # Concurrent product work added its own application facade. Verify
            # that exact addition rather than restoring somebody else's code.
            data = (product / record['path']).read_bytes()
            addition = b'mod projected_application;\npub use projected_application::ProjectedArtifactApplication;\n'
            assert data.count(addition) == 1
            original = data.replace(addition, b'')
            assert len(original) == record['byteLength'] and hashlib.sha256(original).hexdigest() == record['sha256']
        else:
            assert entry(product / record['path'], product) == record, record['path']

environment = read(stage / 'environment-final.json')
previous_environment = read('.codex-work/product-content-stream/environment.json')
for key, root in [('productLocks', product), ('kernelLocks', Path('.'))]:
    previous_locks = {r['path']: r for r in previous_environment[key]}
    for record in environment[key]:
        assert entry(root / record['path'], root) == record
        if key != 'productLocks' or record['path'] != 'apps/agent-runtime/Cargo.lock':
            assert record == previous_locks[record['path']]
old_lock = tomllib.loads(subprocess.check_output(
    ['git', 'show', environment['lockComparisonCommit'] + ':apps/agent-runtime/Cargo.lock'], cwd=product, text=True))
new_lock = tomllib.loads((product / 'apps/agent-runtime/Cargo.lock').read_text())
registry = lambda lock: {(p['name'], p['version'], p.get('source'), p.get('checksum'))
                         for p in lock['package'] if p.get('source')}
assert registry(old_lock) == registry(new_lock)
old_internal = {p['name']: p for p in old_lock['package'] if not p.get('source')}
new_internal = {p['name']: p for p in new_lock['package'] if not p.get('source')}
internal_changes = sorted(k for k in old_internal if old_internal[k] != new_internal[k])
assert internal_changes == ['musterwork-agent-cloud-api', 'musterwork-agent-runtime-artifact', 'musterwork-agent-runtime-content']
for name in ['apps/agent-runtime/crates/runtime/artifact/Cargo.toml',
             'apps/agent-runtime/crates/runtime/artifact/src/projected_application.rs']:
    committed = subprocess.check_output(['git', 'show', environment['concurrentProductCommit'] + ':' + name], cwd=product)
    assert committed == (product / name).read_bytes(), name
manifest = tomllib.loads((product / 'apps/agent-runtime/crates/infrastructure/sqlite/Cargo.toml').read_text())
assert 'tempfile' not in manifest['dependencies']
assert manifest['dev-dependencies']['tempfile'] == '3.27.0'

log = (stage / 'tests-5.log').read_text()
names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
assert len(names) == 41 and 'FAILED' not in log and '41 passed; 0 failed; 1 ignored' in log
executables = re.findall(r'Running unittests src/lib.rs \((target/debug/deps/[^)]+)\)', log)
assert len(executables) == 1
executable = entry(product / 'apps/agent-runtime' / executables[0], product)
for required in ['actual_process_kill_recovers_registration_partial_and_durable_write_boundaries',
                 'cancelled_recovery_keeps_registry_until_queued_file_removal_finishes',
                 'cancelled_temporary_open_cannot_outlive_its_material_lease',
                 'retirement_rechecks_alias_claims_after_tombstone_commit',
                 'reopening_version_182_preserves_existing_metadata_and_blob_bytes']:
    assert any(name.endswith('::' + required) for name in names), required
clippy = (stage / 'clippy-3.log').read_text()
assert 'Finished `dev` profile' in clippy and 'error:' not in clippy and 'warning:' not in clippy
assert not (stage / 'fmt-verified.log').read_bytes()
crashes = read(stage / 'crash-5.json')
assert [r['phase'] for r in crashes] == ['unregistered', 'partial', 'persisted', 'published']
for row in crashes:
    assert row['confirmedLiveBeforeKill'] and row['terminatedUnsuccessfully']
    assert row['journalAfter'] == 0 and row['namespaceAfter'] == ['registry.lock']
    assert row['publishedPreserved'] == (row['phase'] == 'published')

files = read(stage / 'file-verification-current.json')
assert files['actualFiles'] == 13 and files['assets'] == 12 and files['pages'] == 2
assert files['allBytesUnchanged'] and files['finalStoredInspectionUnchanged']
for key in ['cli', 'request']:
    assert entry(files[key]['path']) == files[key]
for record in files['files']:
    assert entry(record['path']) == record
    previous = Path('.codex-work/product-content-stream/actual-final') / Path(record['path']).name
    assert Path(record['path']).read_bytes() == previous.read_bytes()
assert read(stage / 'inspected-current.json') == read('.codex-work/product-content-stream/inspected.json')
windows = read(stage / 'windows-platform-check.json')
assert windows['target'] == 'x86_64-pc-windows-msvc' and windows['exitCode'] == 0
assert windows['executedOnWindows'] is False and windows['entireProductChecked'] is False
assert entry(product / windows['source']['path'], product) == windows['source']
assert entry(windows['metadata']['path']) == windows['metadata']

report = dict(
    format='musteroffice.product-content-lifecycle-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), localLinksChecked=links, kernelComputingSourcesUnchanged=True,
    productBaseline=entry(stage / 'product-baseline.json'),
    productHeadObserved=environment['productHeadObserved'], productPrivateFiles=current['files'],
    priorProductSdkAndReaderImplementationsUnchanged=True,
    concurrentProductFacadeAdditionVerified=True, productTestExecutable=executable,
    environment=entry(stage / 'environment-final.json'), kernelLockBytesUnchanged=True,
    externalRegistryVersionsUnchanged=True, productCargoLockBytesUnchanged=False,
    internalLockChangesSinceObservedCommit=internal_changes,
    dependencies=dict(tempfileDevelopmentOnly='3.27.0', existingFs4='1.1.0', existingUuid='1.24.0', existingRustix='1.1.4'),
    tests=dict(sqliteSelected=41, ignoredStandaloneCrashEntrypoints=1, actuallyKilledChildren=4,
               names=names, strictSqliteProductionClippy=True, sqliteWholeSuite=False),
    migration=dict(fromVersion=182, toVersion=183, preservesExistingContent=True),
    physicalWriteReservation=dict(maxBytes=512 * 1024 * 1024, maxWrites=1024, perJobQuota=False),
    cancellation=dict(queuedOpenLease=True, queuedRecoveryRegistry=True, deletedContentClaimsRechecked=True),
    actualFiles=13, actualAssets=12, actualPages=2, sharedInspectionUnchanged=True,
    windowsDirectoryPrimitive=windows,
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'Actual process termination and filesystem tests ran on macOS arm64; no hardware power-cut, kernel-crash or Windows runtime acceptance.',
        'Windows compiler check covers the actual directory-sync module only, not the entire product or runtime behavior.',
        '512 MiB and 1024 are in-flight physical reservations, not per-job quotas, total disk capacity, RSS or throughput.',
        'Recovery is triggered by startup, writes, deletion and the explicit endpoint; it is not a new background task owner.',
        'Published metadata and admission stages retain bytes. Full Artifact/Invocation orphan GC, old staging names and interrupted independent deletions remain open.',
        'Early cancellation test failures stopped at a SQL await; they do not prove queued deletion. Final tests observe actual I/O submission.',
        'Cloud streaming, immutable range-read integration, native Runtime producer, fenced Artifact commit and Viewer/Player remain unconnected.',
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
                     tests=41, hardKills=4, actualFiles=13, evidence=entry(output))))
