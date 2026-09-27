"""Bind Invocation-owned product preparation evidence to actual tested files."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import xml.etree.ElementTree as ET
import zipfile

from evidence_support import (
    cargo_check, cargo_tests, file_entry, local_links, read_json, recorded_command, write_report,
)


def delivery(directory, baseline, expected_title=None):
    manifest = read_json(directory / 'manifest.json')
    files = read_json(directory / 'files.json')
    receipt = read_json(directory / 'receipt.json')
    assert manifest['version'] == 'presentation-artifact/4'
    assert len(manifest['assets']) == len(files) == 12
    assert len(manifest['pages']) == 2
    assert manifest['claims'] == receipt['bundle']['claims']
    assert [(c['kind'], c['status']) for c in manifest['claims']] == [
        ('structure', 'passed'), ('layout', 'not_proven'), ('native-editability', 'not_proven'),
        ('playback', 'not_proven'), ('target-application', 'not_proven')]
    prior = {r['asset']['id']: r['file'] for r in read_json(baseline / 'files.json')}
    assets = []
    title = None
    for asset, row in zip(manifest['assets'], files):
        assert asset['id'] == row['asset']['id']
        path = directory / row['file']
        info = file_entry(path)
        assert info['sha256'] == asset['content']['sha256']
        assert info['byteLength'] == int(asset['content']['byte_length'])
        if expected_title is None:
            assert path.read_bytes() == (baseline / prior[asset['id']]).read_bytes()
        if asset['role'] == 'pptx':
            with zipfile.ZipFile(path) as package:
                assert package.testzip() is None
                assert len([p for p in package.namelist()
                            if p.startswith('ppt/slides/slide') and p.endswith('.xml')]) == 2
                for part in package.namelist():
                    if part.endswith(('.xml', '.rels')):
                        ET.fromstring(package.read(part))
                title = ET.fromstring(package.read('docProps/core.xml')).find(
                    '{http://purl.org/dc/elements/1.1/}title').text
        assets.append(info)
    assert title == manifest['title']
    if expected_title is not None:
        assert title == expected_title
    return dict(manifest=file_entry(directory / 'manifest.json'), assets=assets,
                pages=2, title=title, matchesBaselineBytes=expected_title is None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--product-repo', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = Path.cwd()
    product = args.product_repo.resolve()
    runtime = product / 'apps/agent-runtime'
    stage = Path('.codex-work/product-owned-preparation')
    accepted = read_json(stage / 'accepted-runs.json')
    required = {'authoring', 'preparation', 'commit', 'libraries', 'clippy', 'workspace'}
    assert set(accepted) == required
    records = {role: recorded_command(stage, name) for role, name in accepted.items()}
    current = records['authoring']['sourceAfter']
    for role, record in records.items():
        assert record['sourceAfter'] == current, role
    for path, digest in current.items():
        assert hashlib.sha256((runtime / path).read_bytes()).hexdigest() == digest, path
    results = {role: cargo_tests(stage / (accepted[role] + '.log'))
               for role in ['authoring', 'preparation', 'commit', 'libraries']}
    assert results['authoring']['passed'] == 60 and results['authoring']['ignored'] == 12
    assert results['preparation']['passed'] == 6 and results['preparation']['ignored'] == 0
    assert results['commit']['passed'] == 3 and results['commit']['ignored'] == 0
    for name in ['native_metadata_is_owned_idempotent_and_retained_with_the_original_commit',
                 'native_metadata_reservation_is_atomic_and_rejects_invalid_scope_before_writes',
                 'computed_mutation_reserves_model_and_receipt_before_any_output_write']:
        assert any(test.endswith(name) for test in results['authoring']['names']), name
    cargo_check(stage / (accepted['clippy'] + '.log'), strict=True)
    cargo_check(stage / (accepted['workspace'] + '.log'))
    paths = read_json(stage / 'changed-paths.json')
    for path in paths:
        if path.endswith('.rs'):
            assert len((runtime / path).read_text().splitlines()) <= 2000, path
    for old in ['prepared_export.rs', 'prepared_export/fixture.rs', 'prepared_export/cancellation.rs']:
        assert not (runtime / 'crates/infrastructure/musteroffice/tests' / old).exists()
    verification = recorded_command(stage, 'verification-tests-01')
    verification_log = (stage / 'verification-tests-01.log').read_text()
    assert 'Ran 7 tests' in verification_log and verification_log.rstrip().endswith('OK')
    for path, digest in verification['sourceAfter'].items():
        assert file_entry(Path('tools/verification') / path)['sha256'] == digest
    sdk = 'f3d44dca6f1f2632832eb3327b71d38915c354ab89143d578ec5618b0470471d'
    vendor = runtime / 'vendor/musteroffice' / sdk
    assert file_entry(vendor / 'sdk-manifest.json', vendor)['sha256'] == sdk
    for entry in read_json(vendor / 'sdk-manifest.json')['files']:
        assert file_entry(vendor / entry['path'], vendor) == entry
    worker = Path('.codex-work/embedded-export/frozen/mo-export-worker')
    assert file_entry(worker)['sha256'] == '1151581c9b689d8329ceec6100aed4103d1f78e38ac178c894ebaf63bfede50f'
    baseline = Path('.codex-work/embedded-sdk/actual-product')
    deliveries = [delivery(stage / 'actual-preparation', baseline),
                  delivery(stage / 'actual-commit/version-1', baseline),
                  delivery(stage / 'actual-commit/version-2', baseline, 'Revised native presentation')]
    output = Path('docs/reviews/evidence/2026-09-27-product-owned-preparation-verification.json')
    documents = [Path(p) for p in ['README.md', 'docs/README.md', 'docs/implementation/progress.md',
                 'docs/design/implementation/musterwork-adapter-spec.md',
                 'docs/implementation/product-owned-preparation.md']]
    tools = [Path('tools/verification') / p for p in ['product_command.py', 'test_product_command.py',
             'evidence_support.py', 'test_evidence_support.py', 'product-owned-preparation-evidence.py']]
    commands = []
    for role, record in records.items():
        normalized = [s.replace(str(product), 'MW:').replace(str(root), 'ROOT:')
                      for s in record['args']]
        commands.append(dict(role=role, args=normalized, exitCode=record['exitCode'],
                             elapsedSeconds=record['elapsedSeconds'],
                             log=file_entry(stage / (accepted[role] + '.log')),
                             sourceUnchanged=True))
    prior = Path('docs/reviews/evidence/2026-09-27-product-invocation-content-verification.json')
    assert file_entry(prior)['sha256'] == 'ae58a86a475d20ef9f0f41361d1880883cf4c9e9bf451a89237e77550ff005df'
    report = dict(format='musteroffice.product-owned-preparation-verification/1',
                  parent=file_entry(prior),
                  sourceFiles=[file_entry(p) for p in documents + tools],
                  privateProductChangedFiles=[file_entry(runtime / p, runtime) for p in paths],
                  productSourceSetSha256=hashlib.sha256(json.dumps(current, sort_keys=True).encode()).hexdigest(),
                  sourceFilesBound=len(current), commands=commands, tests=results,
                  environment=dict(system=platform.system(), machine=platform.machine(),
                      rustc=subprocess.check_output(['rustc', '--version'], text=True).strip()),
                  sdkManifestSha256=sdk, worker=file_entry(worker), deliveries=deliveries,
                  evidenceReaderTests=dict(passed=7, log=file_entry(stage / 'verification-tests-01.log')),
                  localLinksChecked=local_links(documents, pending=[output]),
                  limitations=[
                      'The production adapters and metadata preparer are integrated under the real Runtime in conformance tests; the public Agent tool route is still pending.',
                      'Canonical draft retention includes the Tool result. No terminal garbage collection, account discharge or worker crash-spool recovery is claimed.',
                      'Legacy ordinary prewrites remain readable and committable; this change is not a new versioned commit provenance gate.',
                      'Private Device only. Shared Home/PostgreSQL, Viewer/Player and history conversion are incomplete.',
                      'The frozen SDK is verified; concurrent core changes are not certified by these product tests.',
                      'No complete advanced-content, Office/WPS, cross-platform runtime, performance or installer acceptance.',
                      'Authoring includes the new unit of integration and storage tests. Historical and repeated counts are not added together; three existing external-environment tests remain ignored.',
                  ])
    write_report(output, report, check=args.check)
    print(json.dumps(dict(report=str(output), sha256=file_entry(output)['sha256'], tests={
        k: v['passed'] for k, v in results.items()})))


if __name__ == '__main__':
    main()
