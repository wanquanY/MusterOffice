"""Bind execution recovery, real worker and standalone SDK observations."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import sys

from evidence_support import (
    cargo_check, cargo_tests, file_entry, local_links, read_json, recorded_command, write_report,
)
from product_command import snapshot


def files(directory):
    rows = read_json(directory / 'files.json')
    assert len(rows) == 12
    result = {}
    for row in rows:
        assert Path(row['file']).name == row['file']
        entry = file_entry(directory / row['file'])
        assert entry['byteLength'] == int(row['asset']['byteLength'])
        assert entry['sha256'] == row['asset']['sha256']
        assert row['asset']['id'] not in result
        result[row['asset']['id']] = entry
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    project = Path.cwd()
    stage = Path('.codex-work/execution-spool-recovery')
    frozen = stage / 'build-source'
    inputs = read_json(stage / 'build-source-complete.json')
    assert snapshot(frozen, inputs['sourceInventory']) == inputs['sourceFiles']
    assert hashlib.sha256(json.dumps(inputs['sourceFiles'], sort_keys=True).encode()).hexdigest() == inputs['sourceSetSha256']
    changed = read_json(stage / 'changed-paths.json')
    for path in changed:
        assert Path(path).read_bytes() == (frozen / path).read_bytes(), path
    current_differences = [p for p, digest in inputs['sourceFiles'].items()
                           if not Path(p).is_file() or file_entry(p)['sha256'] != digest]

    names = ['cohort-libraries', 'cohort-clippy-02', 'cohort-release-build',
             'cohort-linux', 'cohort-windows', 'cohort-sdk-tests',
             'cohort-sdk-one', 'cohort-sdk-two', 'cohort-release-integration', 'cohort-reference']
    commands = []
    for name in names:
        command = recorded_command(stage, name)
        assert all(inputs['sourceFiles'].get(p) == digest for p, digest in command['sourceAfter'].items())
        argv = [s.replace(str(frozen.resolve()), 'BUILD_ROOT:').replace(str(project), 'ROOT:')
                for s in command['args']]
        commands.append(dict(name=name, args=argv, exitCode=command['exitCode'],
                             elapsedSeconds=command['elapsedSeconds'], sourceUnchanged=True,
                             sourceFilesBound=len(command['sourceAfter']),
                             log=file_entry(stage / (name + '.log'))))
    for name in ['cohort-clippy-02', 'cohort-linux', 'cohort-windows']:
        cargo_check(stage / (name + '.log'), strict=True)
    assert 'Finished `release` profile' in (stage / 'cohort-release-build.log').read_text()
    library_tests = cargo_tests(stage / 'cohort-libraries.log')
    process_tests = cargo_tests(stage / 'cohort-release-integration.log')
    assert (library_tests['passed'], library_tests['ignored']) == (23, 1)
    assert (process_tests['passed'], process_tests['ignored']) == (11, 1)
    sdk_log = (stage / 'cohort-sdk-tests.log').read_text()
    assert 'Ran 9 tests' in sdk_log and sdk_log.rstrip().endswith('OK')

    worker = read_json(stage / 'worker-cohort.json')
    assert file_entry(worker['path']) == worker
    invocation = recorded_command(stage, 'cohort-release-integration')['args']
    assert 'MO_EXPORT_WORKER_TEST_BIN=' + str(Path(worker['path']).resolve()) in invocation
    assert 'MO_EXPORT_WORKER_TEST_SHA256=' + worker['sha256'] in invocation

    # Pinned source bundle, two independently prepared byte-identical archives.
    sys.path.insert(0, str(Path('tools/sdk').resolve()))
    from verify import verify
    sdk = stage / 'sdk-cohort-one'
    sdk_pin = file_entry(sdk / 'sdk-manifest.json')['sha256']
    manifest = verify(sdk, sdk_pin)
    assert verify(stage / 'sdk-cohort-two', sdk_pin) == manifest
    for entry in manifest['sourceFiles']:
        assert file_entry(frozen / entry['path'], frozen) == entry
    archive_one = file_entry(stage / 'sdk-cohort-one.tar.gz')
    archive_two = file_entry(stage / 'sdk-cohort-two.tar.gz')
    assert archive_one['sha256'] == archive_two['sha256']
    assert archive_one['byteLength'] == archive_two['byteLength']
    assert 'rustix' in {p['name'] for p in manifest['registryPackages']}

    consumer = recorded_command(stage, 'sdk-consumer')
    for p, digest in consumer['sourceAfter'].items():
        assert file_entry(p)['sha256'] == digest, p
    assert '"assets":12' in (stage / 'sdk-consumer.log').read_text()
    actual = stage / 'actual-cohort'
    actual_files = files(actual)
    consumer_files = files(stage / 'actual-sdk')
    assert actual_files.keys() == consumer_files.keys()
    for identity, entry in actual_files.items():
        assert (entry['byteLength'], entry['sha256']) == (
            consumer_files[identity]['byteLength'], consumer_files[identity]['sha256'])
    assert read_json(actual / 'receipt.json') == read_json(stage / 'actual-sdk/receipt.json')
    assert read_json(actual / 'inspection.json') == read_json(stage / 'actual-sdk/inspection.json')
    reference = read_json(actual / 'reference.json')
    assert reference['nativeObjects'] == 15 and reference['nativeTextRuns'] == 8
    assert len(reference['schemasChecked']) == 10 and len(reference['pages']) == 2
    bundle = read_json(actual / 'bundle.json')
    assert [(c['kind'], c['status']) for c in bundle['claims']] == [
        ('structure', 'passed'), ('layout', 'not_proven'), ('native-editability', 'not_proven'),
        ('playback', 'not_proven'), ('target-application', 'not_proven')]

    baseline_evidence = Path('docs/reviews/evidence/2026-09-26-embedded-export-verification.json')
    baseline_entry = read_json(baseline_evidence)['native']
    assert file_entry(baseline_entry['path']) == baseline_entry
    baseline = read_json(baseline_entry['path'])
    assert file_entry(baseline['independentReference']['path']) == baseline['independentReference']
    assert file_entry(baseline['actualPptx']['path']) == baseline['actualPptx']
    pptx = actual_files[bundle['pptxAssetId']]
    assert (pptx['byteLength'], pptx['sha256']) == (
        baseline['actualPptx']['byteLength'], baseline['actualPptx']['sha256'])
    prior_reference = read_json(baseline['independentReference']['path'])
    assert [p['premultipliedSha256'] for p in reference['pages']] == [
        p['premultipliedSha256'] for p in prior_reference['pages']]
    compatibility = read_json(stage / 'legacy-worker.json')
    assert compatibility['exitCode'] != 0 and compatibility['stdoutBytes'] == 0
    assert file_entry(compatibility['worker']['path']) == compatibility['worker']

    documents = [Path(p) for p in ['README.md', 'docs/README.md', 'docs/implementation/progress.md',
                 'docs/implementation/dependencies.md', 'docs/implementation/execution-spool-recovery.md',
                 'docs/design/implementation/musterwork-adapter-spec.md']]
    output = Path('docs/reviews/evidence/2026-09-27-execution-spool-verification.json')
    report = dict(format='musteroffice.execution-spool-verification/1',
                  buildSource=file_entry(stage / 'build-source-complete.json'),
                  buildSourceSetSha256=inputs['sourceSetSha256'],
                  sourceFilesBound=len(inputs['sourceFiles']),
                  currentImplementationFiles=[file_entry(p) for p in changed],
                  currentWorkspaceDifferences=current_differences,
                  commands=commands, libraryTests=library_tests, processTests=process_tests,
                  sdkTests=9, worker=worker, sdkManifest=file_entry(sdk / 'sdk-manifest.json'),
                  sdkArchives=[archive_one, archive_two], sdkLibraries=manifest['libraries'],
                  sdkRegistryPackages=manifest['registryPackages'],
                  sdkConsumer=dict(command=file_entry(stage / 'sdk-consumer.json'),
                      log=file_entry(stage / 'sdk-consumer.log'), assets=list(consumer_files.values()),
                      sameActualBytesAndInspection=True),
                  actualAssets=list(actual_files.values()),
                  reference=file_entry(actual / 'reference.json'),
                  baselineEvidence=file_entry(baseline_evidence), pptxAndPixelsUnchanged=True,
                  legacyWorkerRejection=file_entry(stage / 'legacy-worker.json'),
                  environment=dict(system=platform.system(), machine=platform.machine(),
                      rustc=subprocess.check_output(['rustc', '--version'], text=True).strip()),
                  documentation=[file_entry(p) for p in documents],
                  verifier=file_entry(Path(__file__).resolve().relative_to(project)),
                  localLinksChecked=local_links(documents, pending=[output]),
                  limitations=[
                      'macOS process/runtime evidence; Linux and Windows host-library cross compilation is not platform runtime or renderer acceptance.',
                      'Two ignored tests are subprocess entry points explicitly exercised by parent crash/recovery tests; they are not missing external tests.',
                      'Build inputs are frozen and own changed files are checked against the workspace. This is scoped validation, not a complete workspace or capability regression.',
                      'Musterwork still uses the previous frozen SDK/worker. Product SDK refresh and startup/maintenance wiring are pending.',
                      'Legacy unleased execution directories are preserved. No application terminal content GC, quota-account retirement, aggregate disk bound or power-loss claim.',
                      'Complete advanced content, Viewer/Player, history, Office/WPS, performance, installer and E0-E3 acceptance remain open.',
                  ])
    write_report(output, report, check=args.check)
    print(json.dumps(dict(report=str(output), sha256=file_entry(output)['sha256'],
                         libraries=library_tests['passed'], process=process_tests['passed'],
                         sdk=9, sourceFilesBound=len(inputs['sourceFiles']))))


if __name__ == '__main__':
    main()
