"""Verify recorded Ledger root projection work and real product deliveries."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import platform

from evidence_support import (
    cargo_check, cargo_tests, file_entry, local_links, read_json, recorded_command, write_report,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--product-repo', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    product = args.product_repo.resolve()
    runtime = product / 'apps/agent-runtime'
    stage = Path('.codex-work/product-ledger-content-roots')
    output = Path('docs/reviews/evidence/2026-09-27-product-ledger-content-roots-verification.json')
    accepted = read_json(stage / 'accepted-runs.json')
    assert set(accepted) == {'libraries', 'repairs', 'retention', 'authoring', 'commit', 'clippy', 'workspace'}
    commands = {role: recorded_command(stage, name) for role, name in accepted.items()}
    sources = commands['libraries']['sourceAfter']
    for role, command in commands.items():
        assert command['sourceAfter'] == sources, role
    for path, digest in sources.items():
        assert file_entry(runtime / path, runtime)['sha256'] == digest, path
    results = {role: cargo_tests(stage / (accepted[role] + '.log'))
               for role in ['libraries', 'repairs', 'retention', 'authoring', 'commit']}
    assert results['commit']['passed'] == 3 and results['commit']['ignored'] == 0
    for role, names in {
        'libraries': [
            'tool_result_and_authorized_attachments_are_exact_sorted_and_unique',
            'artifact_dependencies_checkpoint_failed_diagnostic_and_output_frame_are_roots',
            'payload_strings_do_not_mint_references_and_late_unknown_fields_discard_all_results',
            'conflicting_identities_malformed_refs_and_duplicate_oneofs_fail_closed',
            'actual_v186_history_keeps_bytes_and_classifies_reference_coverage',
            'roots_follow_original_ledger_retention_and_cannot_be_forged_or_rewritten',
            'source_corruption_and_late_migration_failure_roll_back_projection_and_version',
            'raw_append_defaults_to_pending_until_captured_in_the_same_transaction',
        ],
        'authoring': [
            'generated_result_and_attachment_remain_rooted_across_duplicate_and_reopen',
            'root_projection_failure_rolls_back_tool_ledger_and_success_receipt',
        ],
        'repairs': [
            'upgrade_from_v178_preserves_existing_resource_consent_bytes',
            'v180_upgrade_keeps_original_pause_unknown_without_inventing_dependencies',
            'startup_progress_distinguishes_migration_clean_and_dirty_restart',
            'actual_v186_history_keeps_bytes_and_classifies_reference_coverage',
            'roots_follow_original_ledger_retention_and_cannot_be_forged_or_rewritten',
            'source_corruption_and_late_migration_failure_roll_back_projection_and_version',
            'raw_append_defaults_to_pending_until_captured_in_the_same_transaction',
        ],
        'retention': [
            'projection_retention_reclaims_output_payload_but_preserves_ledger_spine',
            'ledger_reference_index_survives_actual_output_retirement_and_reopen',
            'payload_collection_advances_past_protected_prefix_and_resumes_after_reopen',
        ],
    }.items():
        for name in names:
            assert any(test.endswith(name) for test in results[role]['names']), name
    cargo_check(stage / (accepted['clippy'] + '.log'), strict=True)
    cargo_check(stage / (accepted['workspace'] + '.log'))
    reader_check = recorded_command(stage, 'evidence-reader-01')
    for path, digest in reader_check['sourceAfter'].items():
        assert file_entry(Path(path))['sha256'] == digest, path
    reader_log = (stage / 'evidence-reader-01.log').read_text()
    assert 'Ran 5 tests' in reader_log and '\nOK\n' in reader_log
    assert 'test_error_module_name_is_not_a_diagnostic_but_real_errors_still_fail' in reader_log

    prior = Path('docs/reviews/evidence/2026-09-27-product-execution-spool-verification.json')
    assert file_entry(prior)['sha256'] == '6a8fb237c1363534e6808d6372a3479964cbc48452534708af8e1f489fb0669b'
    previous = read_json(prior)
    # Some Rust tests consume product-owned fixtures outside the Runtime source
    # inventory. Compare those exact bytes with their original sealed report.
    fixture_origin = Path('docs/reviews/evidence/2026-09-26-product-office-manifest-verification.json')
    assert file_entry(fixture_origin)['sha256'] == 'abb68225847e9b777a76e1f0e6479b94e94b66ff615bbd3d24ab849cd4a17c8c'
    fixtures = [item for item in read_json(fixture_origin)['productPrivateFiles']
                if '/fixtures/office-native/' in item['path']]
    assert len(fixtures) == 19
    for item in fixtures:
        assert file_entry(product / item['path'], product) == item
    # Do not rerun the old phase's checker against newer product source. Reuse
    # only its sealed, unchanged material and input identities.
    for item in previous['inputs']:
        assert file_entry(Path(item['path'])) == item
    for key in ['sdk', 'worker', 'productLock']:
        item = previous[key]
        assert file_entry(product / item['path'], product) == item
    sdk = product / previous['sdk']['path']
    for item in read_json(sdk)['files']:
        assert file_entry(sdk.parent / item['path'], sdk.parent) == item
    observer = Path('tools/verification/product-owned-preparation-evidence.py')
    spec = importlib.util.spec_from_file_location('product_delivery_observer', observer)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    baseline = Path('.codex-work/execution-spool-recovery/actual-sdk')
    cohort = Path('docs/reviews/evidence/2026-09-27-execution-spool-verification.json')
    assert file_entry(cohort)['sha256'] == '591330048aae2626baf7da351b660e831c737a29702f4b8bf58c9ce2e85a6a0c'
    for item in read_json(cohort)['sdkConsumer']['assets']:
        assert file_entry(Path(item['path'])) == item
    deliveries = [
        module.delivery(stage / 'actual-commit/version-1', baseline),
        module.delivery(stage / 'actual-commit/version-2', baseline, 'Revised native presentation'),
    ]
    changed = read_json(stage / 'changed-paths.json')
    for path in changed:
        assert len((runtime / path).read_text().splitlines()) <= 2000, path
    documents = [Path(p) for p in [
        'README.md', 'docs/README.md', 'docs/implementation/progress.md',
        'docs/design/implementation/musterwork-adapter-spec.md',
        'docs/implementation/product-ledger-content-roots.md',
    ]]
    tools = [Path('tools/verification') / name for name in [
        'product-ledger-content-roots-evidence.py', 'product_command.py', 'evidence_support.py',
        'test_evidence_support.py',
    ]]
    report = {
        'format': 'musteroffice.product-ledger-content-roots-verification/1',
        'parent': file_entry(prior),
        'sourceFilesBound': len(sources),
        'sourceSetSha256': hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest(),
        'privateProductChangedFiles': [file_entry(runtime / path, product) for path in changed],
        'sourceFiles': [file_entry(path) for path in documents + tools],
        'evidenceReaderChecks': {
            'record': file_entry(stage / 'evidence-reader-01.json'),
            'log': file_entry(stage / 'evidence-reader-01.log'),
            'passed': 5,
        },
        'commands': {role: {
            'record': file_entry(stage / (name + '.json')),
            'log': file_entry(stage / (name + '.log')),
            'tests': results.get(role),
        } for role, name in accepted.items()},
        'supersededChecks': [{
            'record': file_entry(stage / (name + '.json')),
            'log': file_entry(stage / (name + '.log')),
            'exitCode': read_json(stage / (name + '.json'))['exitCode'],
        } for name in read_json(stage / 'superseded-runs.json')],
        'environment': {'system': platform.system(), 'machine': platform.machine(),
                        'performanceMeasurement': False},
        'sdk': previous['sdk'], 'worker': previous['worker'], 'inputs': previous['inputs'],
        'ownedFixtures': {'origin': file_entry(fixture_origin), 'files': fixtures,
                          'scope': 'Final bytes match sealed fixture origin; outside the per-command Runtime inventory.'},
        'deliveryObserver': file_entry(observer), 'deliveries': deliveries,
        'localLinksChecked': local_links(documents, pending=[output]),
        'limitations': [
            'Typed Ledger roots only. Opaque bytes/JSON and other durable owners still require their own explicit dependency roots.',
            'No terminal Invocation garbage collection, account discharge or default PPT route switch is claimed.',
            'Migration fixtures use the complete unchanged v186 migration program, not a large user database or prior released executable.',
            'The production protocol/SQLite libraries and authoring group were run; this is not the complete product test suite.',
            'Workspace compilation covers all targets; the explicit command inventory is not an attestation of every application, build tool or external input.',
            'Ignored tests and subprocess entrypoints are recorded in command logs; overlapping and historical tests are not added together.',
            'Private Device only. PostgreSQL/shared Home, Agent routing, Viewer/Player and history migration remain incomplete.',
            'No full advanced-content, Office/WPS, cross-platform runtime, performance or complete installer acceptance.',
        ],
    }
    write_report(output, report, check=args.check)
    print(json.dumps({'evidence': file_entry(output), 'tests': {
        role: [result['passed'], result['ignored']] for role, result in results.items()},
    }))


if __name__ == '__main__':
    main()
