"""Bind verified product ranges without importing private product source."""
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
stage = Path('.codex-work/product-content-range')
output = Path('docs/reviews/evidence/2026-09-27-product-content-range-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-product-content-retirement-verification.json')


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


assert entry(parent_path)['sha256'] == '3a9d07aa0ad22036419624eda7439fdddff764f6a8d43beb711a564f31fbea1d'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {p for p, record in prior.items() if entry(p) != record}
assert changed == {'docs/README.md', 'docs/implementation/progress.md'}, changed
added = {'docs/implementation/product-content-range.md', 'tools/verification/product-content-range-evidence.py'}
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
assert baseline['files'] == parent['productPrivateFiles']
current = read(stage / 'product-source-3.json')
assert len(current['files']) == 39
for record in current['files']:
    assert entry(product / record['path'], product) == record, record['path']
    if record['path'].endswith('.rs'):
        assert len((product / record['path']).read_text().splitlines()) <= 2000
base_files = {r['path']: r for r in baseline['files']}
product_changes = sorted(r['path'] for r in current['files']
                         if r['path'] in base_files and r != base_files[r['path']])
prefix = 'apps/agent-runtime/crates/'
assert product_changes == sorted(prefix + p for p in [
    'runtime/content/src/lib.rs', 'runtime/content/src/read_set.rs',
    'infrastructure/sqlite/src/content.rs', 'infrastructure/sqlite/src/content/material/fs.rs',
    'infrastructure/sqlite/src/content/stream.rs', 'infrastructure/sqlite/src/content/stream_file.rs'])
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


def tests(logfile, count, ignored):
    log = (stage / logfile).read_text()
    names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
    assert len(names) == count and 'FAILED' not in log and 'warning:' not in log
    assert f'{count} passed; 0 failed; {ignored} ignored' in log
    binaries = re.findall(r'Running unittests src/lib.rs \((target/debug/deps/[^)]+)\)', log)
    assert len(binaries) == 1
    return names, entry(product / 'apps/agent-runtime' / binaries[0], product)


content_names, content_executable = tests('content-tests-4.log', 26, 0)
sqlite_names, sqlite_executable = tests('sqlite-tests-2.log', 55, 3)
for required in [
    'arbitrary_ranges_match_verified_content_across_block_boundaries',
    'post_verification_mutation_cannot_return_changed_or_unchecked_bytes',
    'oversize_and_cancellation_reject_before_or_between_bounded_source_reads',
    'multi_megabyte_virtual_source_uses_bounded_buffers_and_compact_fingerprints',
    'unsupported_store_never_materializes_a_whole_file_as_range_fallback',
]:
    assert any(n.endswith('::' + required) for n in content_names), required
for required in [
    'exact_ranges_record_verified_identity_and_reject_invalid_authority',
    'retained_handle_ignores_path_replacement_and_rejects_in_place_corruption',
    'cancelled_verification_keeps_concurrency_permit_until_queued_work_exits',
    'metadata_revoked_after_initial_authorization_never_returns_or_records_a_reader',
    'actual_delivery_files_roundtrip_through_reverse_ranges',
    'range_open_rejects_symlink_and_corrupt_material_without_recording',
]:
    assert any(n.endswith('::' + required) for n in sqlite_names), required
clippy = (stage / 'clippy-2.log').read_text()
assert 'Finished `dev` profile' in clippy and 'error:' not in clippy and 'warning:' not in clippy
assert not (stage / 'fmt.log').read_bytes()
windows = read(stage / 'windows-check.json')
assert windows['target'] == 'x86_64-pc-windows-msvc'
assert windows['crate'] == 'musterwork-agent-runtime-content' and windows['exitCode'] == 0
assert not windows['executedOnWindows'] and not windows['sqliteBackendChecked']
assert entry(product / windows['metadata']['path'], product) == windows['metadata']
windows_log = (stage / 'windows-content-check-stable.log').read_text()
assert 'Finished `dev` profile' in windows_log and 'error:' not in windows_log

write_crashes = read(stage / 'write-crashes-2.json')
retirement_crashes = read(stage / 'retirement-crashes-2.json')
assert [r['phase'] for r in write_crashes] == ['unregistered', 'partial', 'persisted', 'published']
assert [r['phase'] for r in retirement_crashes] == ['tombstoned', 'unlinked', 'dequeued', 'completed']
for row in write_crashes + retirement_crashes:
    assert row['confirmedLiveBeforeKill'] and row['terminatedUnsuccessfully']
for row in write_crashes:
    assert row['journalAfter'] == 0 and row['namespaceAfter'] == ['registry.lock']
    assert row['publishedPreserved'] == (row['phase'] == 'published')
for row in retirement_crashes:
    assert row['durableRetirementsAfter'] == 0 and not row['blobAfter'] and row['tombstonePreserved']

files = read(stage / 'file-verification.json')
assert files['actualFilesPerMode'] == 13 and files['assets'] == 12 and files['pages'] == 2
assert files['allBytesUnchanged'] and files['finalStoredInspectionUnchanged']
for key in ['cli', 'request']:
    assert entry(files[key]['path']) == files[key]
for mode in ['range', 'stream']:
    assert len(files[mode]) == 13
    for record in files[mode]:
        assert entry(record['path']) == record
        previous = Path('.codex-work/product-content-retirement/actual-3') / Path(record['path']).name
        assert Path(record['path']).read_bytes() == previous.read_bytes()
assert read(stage / 'inspected.json') == read('.codex-work/product-content-retirement/inspected.json')

report = dict(
    format='musteroffice.product-content-range-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed), addedSources=sorted(added),
    localLinksChecked=links, kernelComputingSourcesUnchanged=True, productBaseline=entry(stage / 'product-baseline.json'),
    productHeadObserved=environment['productHeadObserved'], productPrivateFiles=current['files'], changedProductSources=product_changes,
    environment=entry(stage / 'environment.json'), productAndKernelLockBytesUnchanged=True,
    priorProductSdkAndReaderImplementationsUnchanged=True,
    tests=dict(content=26, sqliteSelected=55, contentNames=content_names, sqliteNames=sqlite_names,
               finalRegressionHardKills=8, ignoredHarnessEntrypoints=3, externalPreviousBinaryUpgradeRerun=False,
               strictContentAndSqliteProductionClippy=True, sqliteWholeSuite=False),
    executables=[content_executable, sqlite_executable], windowsCompiler=windows,
    verifiedRange=dict(fullInitialLengthAndHash=True, verifiedEof=True, perBlockRevalidation=True,
                       chunkBytes=65536, digestBytesPerBlock=32, wholeFileBufferFallback=False,
                       exactReadSetRecording=True, metadataRecheckedAfterScan=True,
                       cancellationRetainsActualScanPermit=True, existingInitializationPermits=8,
                       privateMaterialIsNotFreshAuthority=True, productSdkBridgeConnected=False),
    actualFilesPerMode=13, actualAssets=12, actualPages=2, rangeAndStreamBytesUnchanged=True,
    sharedInspectionUnchanged=True, artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'Actual file execution is macOS arm64; Windows compilation covers the shared content crate, not SQLite file I/O or runtime behavior.',
        'Each range verifies its entire block. No throughput, latency, RSS, installer or whole-product performance claim.',
        '64 KiB bounds internal scan/range scratch, not caller buffers, concurrent consumers, all handles or process memory.',
        'Retained private material can outlive tombstoning like an already returned Vec; the original Runtime must cancel, release and reauthorize the final commit.',
        'Cloud range sources, SDK ReaderAt bridge, Invocation budgets, native producer and fenced Artifact commit remain open.',
        'No new kernel Native/WASM/worker build, complete product regression or repeat of the external V183 binary migration experiment.',
        'Viewer/Player, advanced content, Office/WPS editing, history, platform distribution and E0-E3 remain open.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(current['files']), links=links,
                     contentTests=26, sqliteTests=55, hardKills=8, filesPerMode=13, evidence=entry(output))))
