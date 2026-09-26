"""Verify native product readers without importing private application source.

Run from MusterOffice with --product-repo PATH. Product code stays in that
repository; this report records relative paths and hashes only.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
from urllib.parse import unquote

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
product = args.product_repo.resolve()
stage = Path('.codex-work/product-native-manifest')
output = Path('docs/reviews/evidence/2026-09-26-product-office-manifest-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-embedded-export-verification.json')


def entry(path, base=Path('.')):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path.relative_to(base)), byteLength=len(data),
                sha256=hashlib.sha256(data).hexdigest())


def read(path):
    return json.loads(Path(path).read_text())


def product_git(*arguments):
    return subprocess.check_output(['git', *arguments], cwd=product)


assert entry(parent_path)['sha256'] == '35f680df59857e0dcda66db1dab36675715681a234cb0f6f43b2c8084bb664ae'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {name for name, record in prior.items() if entry(name) != record}
assert changed == {'docs/README.md', 'docs/implementation/progress.md',
                   'docs/implementation/dependencies.md'}, changed
added = {'docs/implementation/product-office-manifest.md',
         'docs/design/implementation/musterwork-adapter-spec.md',
         'tools/verification/product-office-fixture.py',
         'tools/verification/product-office-manifest-evidence.py'}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
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
head = product_git('rev-parse', 'HEAD').decode().strip()
assert baseline['head'] == head == '836a5cf3eb68de78aebad887cc6002b08a892125'
for record in baseline['files']:
    old = product_git('show', f"{head}:{record['path']}")
    assert len(old) == record['byteLength']
    assert hashlib.sha256(old).hexdigest() == record['sha256']
tracked = {r['path'] for r in baseline['files']}
assert set(product_git('diff', '--name-only').decode().splitlines()) == tracked
private_sources = tracked | {
    'apps/agent-runtime/crates/runtime/artifact/src/presentation_office_manifest.rs',
    'packages/agent-runtime-product-client/src/office-presentation-artifact.ts',
    'packages/agent-runtime-product-client/src/office-presentation-values.ts',
    'packages/agent-runtime-product-client/src/office-presentation-json.ts',
    'packages/agent-runtime-product-client/tests/office-presentation-artifact.spec.ts',
}
for directory in ['apps/agent-runtime/crates/runtime/artifact/src/presentation_office_manifest',
                  'packages/agent-runtime-product-client/tests/fixtures/office-native']:
    private_sources |= {str(p.relative_to(product)) for p in (product / directory).rglob('*') if p.is_file()}
for name in private_sources:
    if Path(name).suffix in ['.rs', '.ts']:
        assert len((product / name).read_text().splitlines()) <= 2000, name

fixture = product / 'packages/agent-runtime-product-client/tests/fixtures/office-native'
for path in (stage / 'fixture-reproduced').iterdir():
    assert path.read_bytes() == (fixture / path.name).read_bytes(), path.name
assert len(read(fixture / 'cases.json')) == 62
assert len(read(fixture / 'raw-cases.json')) == 14
manifest = read(fixture / 'manifest.json')
assert manifest['version'] == 'presentation-artifact/4'
assert len(manifest['assets']) == 12 and len(manifest['pages']) == 2
assert [(c['kind'], c['status']) for c in manifest['claims']] == [
    ('structure', 'passed'), ('layout', 'not_proven'), ('native-editability', 'not_proven'),
    ('playback', 'not_proven'), ('target-application', 'not_proven')]
files = read(fixture / 'files.json')
assert len(files) == 13
for item in files:
    record = entry(fixture / item['file'], fixture)
    assert str(record['byteLength']) == item['content']['byte_length']
    assert record['sha256'] == item['content']['sha256']
assert product_git('check-ignore', '--no-index', '--non-matching', '-v', str((fixture / '011.pptx').relative_to(product))).decode().split('\t')[0].endswith('!*.pptx')

tests_log = (stage / 'rust-tests-3.log').read_text()
rust_tests = re.findall(r'^test (.+) \.\.\. ok$', tests_log, re.M)
assert len(rust_tests) == 71 and 'FAILED' not in tests_log
assert 'Doc-tests musterwork_agent_runtime_artifact' in tests_log
ts_all = (stage / 'ts-tests-1.log').read_text()
assert re.search(r'Tests\s+624 passed \| 76 skipped', ts_all)
assert re.search(r'Test Files\s+65 passed \| 3 skipped', ts_all)
ts_final = (stage / 'ts-tests-2.log').read_text()
assert re.search(r'Tests\s+96 passed', ts_final) and 'failed' not in ts_final
assert not (stage / 'ts-check-1.log').read_bytes()
assert not (stage / 'fmt-1.log').read_bytes()
clippy = (stage / 'clippy-4.log').read_text()
assert 'Finished `dev` profile' in clippy and 'error:' not in clippy
assert 'crates/domains/capability/src/browser.rs:337' in (stage / 'clippy-1.log').read_text()
assert 'crates/domains/artifact/src/lib.rs:353' in (stage / 'clippy-2.log').read_text()
assert 'presentation_office_manifest/validate.rs:74' in (stage / 'clippy-3.log').read_text()
# Confirm the first two failures refer to existing product code. Compare bytes
# in memory only; no private source or patch is saved in this repository.
browser = 'apps/agent-runtime/crates/domains/capability/src/browser.rs'
assert product_git('show', f'{head}:{browser}') == (product / browser).read_bytes()
domain = 'apps/agent-runtime/crates/domains/artifact/src/lib.rs'
old_lines = product_git('show', f'{head}:{domain}').splitlines()
new_line = (product / domain).read_bytes().splitlines()[352]
assert new_line in old_lines

environment = read(stage / 'environment.json')
assert environment['productHead'] == head
for record in environment['productLocks']:
    assert entry(product / record['path'], product) == record
    assert hashlib.sha256(product_git('show', f"{head}:{record['path']}")).hexdigest() == record['sha256']
for record in environment['kernelLocks']:
    assert entry(record['path']) == record
    assert prior[record['path']] == record
binary_paths = re.findall(r'Running unittests src/lib.rs \((target/debug/deps/[^)]+)\)', tests_log)
assert len(binary_paths) == 2

report = dict(
    format='musteroffice.product-office-manifest-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), localLinksChecked=links, kernelRuntimeSourcesUnchanged=True,
    productHead=head, productBaselineHashes=baseline['files'],
    productPrivateFiles=[entry(product / p, product) for p in sorted(private_sources)],
    productTestExecutables=[entry(product / 'apps/agent-runtime' / p, product) for p in binary_paths],
    environment=entry(stage / 'environment.json'), rustTests=71, rustTestNames=rust_tests,
    typescript=dict(completeSuitePassed=624, completeSuiteSkipped=76, finalNativeSuitePassed=96,
                    strictProductionTypes=True),
    sharedCases=dict(metadata=62, rawJson=14, capacity=4), actualAssets=12, bundleReferences=13,
    lint=dict(productionLibrariesPassed=True, allTargetsPassed=False,
              dependencyFailure='existing capability collapsible_if',
              testFailure='existing domain test unwrap_used',
              ownProductionFailureFixed='native validator collapsible_if'),
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'Read-compatible product metadata and Rust file resolution only; no native producer, atomic Runtime commit or user-visible route switch yet.',
        'Metadata validity and root integrity do not prove bundle/resource bytes or evidence substance; producer must inspect final stored bytes using the shared kernel.',
        'Five producer claims are preserved without an aggregate editable/QA pass. Only structure passed for the actual owned fixture.',
        'Product TypeScript runs under Node in this stage; Web/Desktop Viewer/Player and browser production integration are unaccepted.',
        'All-targets Clippy did not pass existing product diagnostics; the final strict check covers only selected production libraries with --no-deps.',
        '256-page container capacity is not an increase to product authoring/source-preview limits or proof of rendering performance.',
        'No new external dependencies or kernel binary changes; no new installer, performance or complete advanced-content claim.',
        'Full Office/WPS roundtrip, advanced content, historical migration and E0-E3 replacement acceptance remain open.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(private_sources), links=links,
                     rustTests=71, nativeTsTests=96, evidence=entry(output))))
