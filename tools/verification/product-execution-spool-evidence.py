"""Read product host/maintenance observations and exact delivered file evidence."""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import tomllib

from evidence_support import (
    cargo_check, cargo_tests, file_entry, local_links, read_json, recorded_command, write_report,
)


def history_comparison(current):
    """Compare complete old/new assets; enumerate every permitted metadata delta."""
    previous = Path('.codex-work/embedded-sdk/actual-product')
    old = read_json(previous / 'files.json')
    new = read_json(current / 'files.json')
    assert len(old) == len(new) == 12
    ids = {b['asset']['id']: a['asset']['id'] for a, b in zip(old, new)}
    unchanged, metadata = [], []
    for a, b in zip(old, new):
        assert a['asset']['role'] == b['asset']['role']
        left, right = previous / a['file'], current / b['file']
        if left.read_bytes() == right.read_bytes():
            unchanged.append({'role': a['asset']['role'], 'file': file_entry(right)})
            continue
        before, after = read_json(left), read_json(right)
        normalized = copy.deepcopy(after)
        if a['file'] == '005.bin':
            assert after['previewRenderer']['implementationSha256'] == '47a2f4ff55ed7df0272870e645e02604a252dc355faef8dd6d687921015682e7'
            normalized['previewRenderer']['implementationSha256'] = before['previewRenderer']['implementationSha256']
            assert after['settingsDigest'] != before['settingsDigest']
            normalized['settingsDigest'] = before['settingsDigest']
            changes = ['previewRenderer.implementationSha256', 'settingsDigest']
        elif a['file'] in ('007.bin', '009.bin'):
            assert before['format'] == 'musteroffice.preview-evidence/1-draft'
            assert after['format'] == 'musteroffice.preview-evidence/3-draft'
            assert after['planSha256'] == after['render']['page']['page']['sourceSha256']
            normalized.pop('planSha256')
            normalized['format'] = before['format']
            normalized['render']['page']['page']['sourceSha256'] = before['render']['page']['page']['sourceSha256']
            changes = ['format', 'planSha256', 'render.page.page.sourceSha256']
        elif a['file'] == '010.bin':
            normalized['previews'] = [ids[value] for value in after['previews']]
            normalized['contextAssetId'] = ids[after['contextAssetId']]
            changes = ['previews', 'contextAssetId']
        else:
            raise ValueError(f'unexpected asset change: {a["file"]}')
        assert normalized == before, a['file']
        metadata.append({'old': file_entry(left), 'new': file_entry(right), 'changedFields': changes})
    assert len(unchanged) == 8 and len(metadata) == 4
    assert sum(row['role'] == 'pptx' for row in unchanged) == 1
    assert sum(row['role'] == 'preview' for row in unchanged) == 2
    return {'unchangedAssets': unchanged, 'metadataChanges': metadata}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--product-repo', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    product = args.product_repo.resolve()
    runtime = product / 'apps/agent-runtime'
    stage = Path('.codex-work/product-execution-spool')
    output = Path('docs/reviews/evidence/2026-09-27-product-execution-spool-verification.json')
    accepted = read_json(stage / 'accepted-runs.json')
    assert set(accepted) == {'host', 'device', 'preparation', 'commit', 'bridge',
                             'authoring', 'content', 'clippy', 'workspace', 'binary'}
    records = {role: recorded_command(stage, name) for role, name in accepted.items()}
    sources = records['workspace']['sourceAfter']
    for role, record in records.items():
        # Binary error mapping is checked by the final all-targets build and
        # binary tests. Earlier library tests bind the identical shared sources.
        missing = set(sources) - set(record['sourceAfter'])
        assert all(path.startswith('bins/musterwork-agent-device/') for path in missing), role
        assert all(sources.get(path) == digest for path, digest in record['sourceAfter'].items()), role
    for path, digest in sources.items():
        assert file_entry(runtime / path, runtime)['sha256'] == digest, path
    results = {role: cargo_tests(stage / (accepted[role] + '.log'))
               for role in accepted if role not in ('clippy', 'workspace')}
    for role, passed, ignored in [('host', 3, 1), ('device', 3, 0), ('preparation', 6, 0),
                                   ('commit', 3, 0), ('bridge', 1, 0), ('authoring', 60, 12)]:
        assert results[role]['passed'] == passed and results[role]['ignored'] == ignored, role
    assert (results['content']['passed'], results['content']['ignored']) == (41, 3)
    assert (results['binary']['passed'], results['binary']['ignored']) == (9, 0)
    assert 'tests::office_component_failure_keeps_its_public_startup_identity' in results['binary']['names']
    cargo_check(stage / (accepted['clippy'] + '.log'), strict=True)
    cargo_check(stage / (accepted['workspace'] + '.log'))
    for role, name in [('host', 'live_process_survives_recovery_then_restart_reclaims_after_real_kill'),
                       ('host', 'cancelled_queued_maintenance_retains_capacity_and_does_not_delete'),
                       ('device', 'device_startup_and_existing_maintenance_recover_without_product_activation')]:
        assert any(test.endswith(name) for test in results[role]['names']), name

    sdk = '35e9a712d7bde8342783fcdf7a284335fd907d0a0f8acbfd4ad8a6c40650fb07'
    worker_hash = '47a2f4ff55ed7df0272870e645e02604a252dc355faef8dd6d687921015682e7'
    pins = read_json(product / 'components/musteroffice/lock.json')
    assert pins['sdkManifestSha256'] == sdk
    assert pins['workers']['darwin-arm64'] == {'sha256': worker_hash, 'byteLength': 10940400}
    manifest = runtime / 'vendor/musteroffice' / sdk / 'sdk-manifest.json'
    assert file_entry(manifest, product)['sha256'] == sdk
    for item in read_json(manifest)['files']:
        assert file_entry(manifest.parent / item['path'], manifest.parent) == item
    cargo = tomllib.loads((runtime / 'Cargo.toml').read_text())
    assert cargo['workspace']['dependencies']['mo-embedded-sdk']['path'] == f'vendor/musteroffice/{sdk}/crates/mo-embedded-sdk'
    lock = tomllib.loads((runtime / 'Cargo.lock').read_text())
    assert [p['version'] for p in lock['package'] if p['name'] == 'rustix'] == ['1.1.5']
    worker = product / f'components/musteroffice/generated/workers/darwin-arm64/{worker_hash}/mo-export-worker'
    assert file_entry(worker, product)['sha256'] == worker_hash
    material = recorded_command(stage, 'installed-material-01')
    for path, digest in material['sourceAfter'].items():
        assert file_entry(product / path, product)['sha256'] == digest, path
    preparation = recorded_command(stage, 'preparation-script-01')
    assert 'pass 15' in (stage / 'preparation-script-01.log').read_text()
    for path, digest in preparation['sourceAfter'].items():
        assert file_entry(product / path, product)['sha256'] == digest, path

    # Reuse the existing observer without editing or rewriting its source.
    observer = Path('tools/verification/product-owned-preparation-evidence.py')
    spec = importlib.util.spec_from_file_location('product_delivery_observer', observer)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    baseline = Path('.codex-work/execution-spool-recovery/actual-sdk')
    cohort_path = Path('docs/reviews/evidence/2026-09-27-execution-spool-verification.json')
    assert file_entry(cohort_path)['sha256'] == '591330048aae2626baf7da351b660e831c737a29702f4b8bf58c9ce2e85a6a0c'
    cohort = read_json(cohort_path)
    for item in cohort['sdkConsumer']['assets']:
        assert file_entry(Path(item['path'])) == item
    prior_command = cohort['sdkConsumer']['command']
    assert file_entry(Path(prior_command['path'])) == prior_command
    input_pins = read_json(prior_command['path'])['sourceAfter']
    inputs = [file_entry(path) for path in sorted(Path('.codex-work/execution-spool-recovery/sdk-input').iterdir())]
    assert len(inputs) == 5
    for item in inputs:
        assert input_pins[item['path']] == item['sha256']
    deliveries = [module.delivery(stage / 'actual-preparation-02', baseline),
                  module.delivery(stage / 'actual-commit/version-1', baseline),
                  module.delivery(stage / 'actual-commit/version-2', baseline,
                                  'Revised native presentation')]
    bridge = stage / 'actual-bridge'
    before = {r['asset']['id']: r['file'] for r in read_json(baseline / 'files.json')}
    bridge_files = []
    for row in read_json(bridge / 'files.json'):
        path = bridge / row['file']
        info = file_entry(path)
        assert info['sha256'] == row['asset']['sha256']
        assert info['byteLength'] == int(row['asset']['byteLength'])
        assert path.read_bytes() == (baseline / before[row['asset']['id']]).read_bytes()
        bridge_files.append(info)
    assert len(bridge_files) == 12
    for name in ('receipt.json', 'inspection.json'):
        assert read_json(bridge / name) == read_json(baseline / name)
    changed = read_json(stage / 'changed-paths.json')
    for path in changed:
        if path.endswith('.rs'):
            assert len((runtime / path).read_text().splitlines()) < 2000
    report = {
        'format': 'musteroffice.product-execution-spool-verification/1',
        'environment': {'platform': platform.platform(), 'machine': platform.machine(),
                        'performanceMeasurement': False},
        'sourceSetSha256': hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest(),
        'sourceFilesBound': len(sources),
        'productPrivateFiles': [file_entry(runtime / path, product) for path in changed],
        'productLock': file_entry(runtime / 'Cargo.lock', product),
        'sdk': file_entry(manifest, product),
        'worker': file_entry(worker, product),
        'inputs': inputs,
        'commands': {role: {'record': file_entry(stage / (name + '.json')),
                            'log': file_entry(stage / (name + '.log')),
                            'tests': results.get(role)} for role, name in accepted.items()},
        'materialValidation': file_entry(stage / 'installed-material-01.json'),
        'preparerTests': file_entry(stage / 'preparation-script-01.json'),
        'supersededChecks': [
            {'record': file_entry(stage / 'preparation-01.json'),
             'reason': 'Old worker receipt baseline differed in four metadata assets; assertions kept, rerun against independently sealed matching SDK baseline.'},
            {'record': file_entry(stage / 'workspace-01.json'),
             'reason': 'All-targets build found missing OfficeRuntime error mapping in Device binary; fixed the mapping and added a startup error test.'},
        ],
        'deliveries': deliveries,
        'bridgeAssets': bridge_files,
        'historicalComparison': history_comparison(baseline),
        'deliveryObserver': file_entry(observer),
        'verifier': file_entry(Path(__file__).relative_to(Path.cwd())),
        'commandRecorder': file_entry(Path('tools/verification/product_command.py')),
        'kernelCohort': file_entry(cohort_path),
        'documentsChecked': local_links([Path('README.md'), Path('docs/README.md'),
            Path('docs/implementation/progress.md'), Path('docs/implementation/product-execution-spool.md'),
            Path('docs/design/implementation/musterwork-adapter-spec.md')], pending=[output]),
        'limits': [
            'host ignored test is a child entry explicitly executed by its parent process test',
            '9 ignored real-worker authoring tests separately executed; 3 older external tests remain ignored',
            'content suite explicitly invokes two ignored subprocess helpers; one old-version external-store test remains ignored',
            'all-targets build retains 10 unused-import warnings in unchanged conformance tests; not a full-workspace strict lint claim',
            'Device lifecycle test invokes startup and the same maintenance function; does not start full authenticated background host',
            'only macOS runtime; no new Linux/Windows execution proof',
            'fixed SDK cohort does not certify unrelated evolving core sources',
            'no new public Agent tool route, Viewer/Player, shared Home, terminal business GC or account retirement',
            'advanced features, Office/WPS, history, complete performance/install size and P00/E0-E3 remain open',
        ],
    }
    write_report(output, report, args.check)
    print(json.dumps({'evidence': file_entry(output), 'tests': {k: (v['passed'], v['ignored']) for k, v in results.items()}}))


if __name__ == '__main__':
    main()
