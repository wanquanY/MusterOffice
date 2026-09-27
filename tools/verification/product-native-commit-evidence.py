"""Verify this product commit stage using shared evidence readers.

Run against the private product checkout; the report records hashes, not source.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET
import zipfile

from evidence_support import cargo_check, cargo_tests, file_entry, local_links, read_json, write_report

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
product = args.product_repo.resolve()
stage = Path('.codex-work/product-native-commit')
checks = stage / 'checks-02'
parent_path = Path('docs/reviews/evidence/2026-09-27-product-native-export-verification.json')
output = Path('docs/reviews/evidence/2026-09-27-product-native-commit-verification.json')
assert file_entry(parent_path)['sha256'] == 'ddf708ad428760ccf1c2a64ad2778901cfd95ae61926d24d5bc0b147983d938f'
parent = read_json(parent_path)
explicit = set(read_json(stage / 'changed-product-files.json'))
prior = {r['path']: r for r in parent['productPrivateFiles']}
names = sorted(set(prior) | explicit)
changed = {n for n, old in prior.items() if file_entry(product / n, product) != old}
assert changed <= explicit, changed - explicit
observed = read_json(checks / 'checked-product-sources.json')
current = {n: file_entry(product / n, product)['sha256'] for n in names}
real_run = read_json(stage / 'real-run.json')
assert real_run['exitCode'] == 0 and real_run['sourceBefore'] == real_run['sourceAfter'] == current
# Only the ignored real-worker test fixture changed after the general suite.
# All production code and ordinary authoring tests keep their tested hashes;
# the changed fixture was subsequently rebuilt and explicitly executed above.
fixture_delta = {n for n in names if observed[n] != current[n]}
assert fixture_delta == {'apps/agent-runtime/tests/conformance/tests/sqlite_agent_loop/presentation_office_exports.rs'}
for name in explicit:
    if name.endswith('.rs'):
        assert len((product / name).read_text().splitlines()) <= 2000, name

commands = read_json(checks / 'checks.json')
assert [c['name'] for c in commands] == ['migration-final', 'migration-guard', 'authoring-regression', 'library-tests', 'clippy-scoped', 'workspace-check']
assert all(c['exitCode'] == 0 and c['sourceBefore'] == c['sourceAfter'] for c in commands)
assert all(c['sourceBefore'] == hashlib.sha256(json.dumps(observed,sort_keys=True).encode()).hexdigest() for c in commands)
tests = {n: cargo_tests(checks / (n + '.log')) for n in ['migration-final', 'migration-guard', 'authoring-regression', 'library-tests']}
tests['realCommit'] = cargo_tests(stage / 'commit-resumed.log')
integration = read_json(stage / 'integration-checks.json')
assert [c['name'] for c in integration] == ['bridge-content', 'artifact-application']
assert all(c['exitCode'] == 0 and c['sourcesUnchanged'] for c in integration)
for name in ['bridge-content', 'artifact-application']:
    tests[name] = cargo_tests(stage / (name + '.log'))
assert tests['library-tests']['passed'] == 71
assert tests['bridge-content']['passed'] == 5 and tests['artifact-application']['passed'] == 6
assert tests['realCommit']['passed'] == 3 and tests['realCommit']['ignored'] == 0
assert tests['migration-final']['passed'] == 5 and tests['migration-guard']['passed'] == 3
assert sum('office_drafts::computed::' in n for n in tests['authoring-regression']['names']) == 6
cargo_check(checks / 'clippy-scoped.log', strict=True)
cargo_check(checks / 'workspace-check.log')
assert not (stage / 'format-final.log').read_bytes()
assert 'Ran 3 tests' in (stage / 'evidence-helper-tests.log').read_text()
assert (stage / 'evidence-helper-tests.log').read_text().rstrip().endswith('OK')

environment = read_json(stage / 'environment.json')
assert file_entry(product / 'apps/agent-runtime/Cargo.lock', product) == environment['productLock']
assert environment['productLock'] == read_json(parent['environment']['path'])['productLock']
assert file_entry(environment['worker']['path']) == environment['worker']
sdk = 'f3d44dca6f1f2632832eb3327b71d38915c354ab89143d578ec5618b0470471d'
vendor = product / 'apps/agent-runtime/vendor/musteroffice' / sdk
assert file_entry(vendor / 'sdk-manifest.json', product)['sha256'] == sdk
for row in read_json(vendor / 'sdk-manifest.json')['files']:
    assert file_entry(vendor / row['path'], vendor) == row

baseline = Path('.codex-work/embedded-sdk/actual-product')
original_files = read_json(baseline / 'files.json')
deliveries = []
for version in [1, 2]:
    directory = stage / 'actual-resumed' / f'version-{version}'
    manifest = read_json(directory / 'manifest.json')
    candidate = read_json(directory / 'candidate.json')
    checkpoint = read_json(directory / 'checkpoint.json')
    files = read_json(directory / 'files.json')
    assert manifest['version'] == 'presentation-artifact/4'
    assert manifest['artifact_version'] == str(version)
    assert candidate['schema'] == 'presentation-candidate-state/3'
    assert checkpoint['version'] == 'presentation-authoring-checkpoint/4'
    assert checkpoint['states'] == [dict(kind='office-candidate', json_utf8=(directory / 'candidate.json').read_text())]
    assert candidate['manifest_json_utf8'] == (directory / 'manifest.json').read_text()
    assert len(files) == len(manifest['assets']) == 12 and len(manifest['pages']) == 2
    assert manifest['claims'] == read_json(directory / 'receipt.json')['bundle']['claims']
    assert [(c['kind'], c['status']) for c in manifest['claims']] == [('structure','passed'), ('layout','not_proven'), ('native-editability','not_proven'), ('playback','not_proven'), ('target-application','not_proven')]
    pptx = None
    for row, asset, original in zip(files, manifest['assets'], original_files):
        assert row['asset'] == asset
        path = directory / row['file']
        info = file_entry(path)
        assert info['sha256'] == asset['content']['sha256']
        assert str(info['byteLength']) == asset['content']['byte_length']
        if version == 1:
            assert path.read_bytes() == (baseline / original['file']).read_bytes()
        if asset['content']['media_type'] == 'application/vnd.openxmlformats-officedocument.presentationml.presentation':
            pptx = path
    assert pptx is not None
    with zipfile.ZipFile(io.BytesIO(pptx.read_bytes())) as package:
        assert package.testzip() is None
        slides = sorted(n for n in package.namelist() if re.fullmatch(r'ppt/slides/slide\d+\.xml', n))
        assert len(slides) == 2
        for name in slides:
            ET.fromstring(package.read(name))
        title = ET.fromstring(package.read('docProps/core.xml')).find('{http://purl.org/dc/elements/1.1/}title')
        assert title is not None and title.text == manifest['title']
    if version == 2:
        assert manifest['title'] == 'Revised native presentation'
    deliveries.append(dict(artifactVersion=version, manifest=file_entry(directory / 'manifest.json'),
                           pptx=file_entry(pptx), assets=12, pages=2))

sources = ['README.md', 'docs/README.md', 'docs/implementation/progress.md',
           'docs/implementation/product-native-commit.md', 'docs/design/implementation/musterwork-adapter-spec.md',
           'tools/verification/product-native-commit-evidence.py', 'tools/verification/evidence_support.py',
           'tools/verification/test_evidence_support.py']
executables = set()
for log in [stage / 'commit-resumed.log', stage / 'bridge-content.log', stage / 'artifact-application.log',
            *[checks / (name + '.log') for name in ['migration-final', 'migration-guard', 'authoring-regression', 'library-tests']]]:
    executables.update(re.findall(r'Running [^\n]+ \((target/debug/deps/[^)]+)\)', log.read_text()))
report = dict(format='musteroffice.product-native-commit-verification/1', parent=file_entry(parent_path),
              sourceFiles=[file_entry(p) for p in sources], localLinksChecked=local_links(sources, pending=[output]),
              productPrivateFiles=[file_entry(product / n, product) for n in names],
              explicitProductChanges=sorted(explicit), changedPriorProductFiles=sorted(changed),
              fixtureRebuiltAndExplicitlyRerunAfterGeneralChecks=sorted(fixture_delta),
              environment=file_entry(stage / 'environment.json'), tests=tests, deliveries=deliveries,
              executables=[file_entry(product / 'apps/agent-runtime' / p, product) for p in sorted(executables)],
              productSchema=185, nativeCandidateSchema=3, nativeCheckpointSchema=4,
              sameAttemptArtifactToolLedgerTransaction=True, retainedDraftEditable=True,
              storedCheckpointReopenedAndCommitted=True, oldRequestReplayAfterSecondVersion=True,
              staleRuntimeWriterRejected=True,
              scopedProductionStrictClippy=True, productWorkspaceAllTargetsCompile=True,
              wholeProductTestSuite=False, productionToolRouteChanged=False,
              artifacts=[file_entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
              limitations=[
                  'Candidate state is not an external grant; the original owner and actual producer remain required.',
                  'The real-worker conformance uses a historical fixture Tool identity, not the new production Agent route.',
                  'Migration tests use unchanged historical DDL and synthetic relational fixtures, not user databases.',
                  'Independent storage reopen is tested; worker crash recovery and full Invocation prewrite lifetime remain pending.',
                  'Shared Home/PostgreSQL, Viewer/Player, historical presentation/template conversion and full advanced content remain open.',
                  'Frozen SDK and worker tests do not revalidate concurrently changed core sources or resolve author/source semantic convergence.',
                  'No Office/WPS editing, cross-platform runtime, installer, performance or E0-E3 acceptance is established.',
              ])
write_report(output, report, args.check)
print(dict(productFiles=len(names), tests={n:v['passed'] for n,v in tests.items()}, deliveries=len(deliveries)))
