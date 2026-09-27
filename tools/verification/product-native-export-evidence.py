"""Bind the actual product export producer to tests and immutable output bytes.

Private product source stays in that repository. Current core work outside this
adapter stage is not implicitly covered by historical core verification.
"""
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
stage = Path('.codex-work/product-native-export')
parent_path = Path('docs/reviews/evidence/2026-09-27-product-native-mutations-verification.json')
output = Path('docs/reviews/evidence/2026-09-27-product-native-export-verification.json')
sdk = 'f3d44dca6f1f2632832eb3327b71d38915c354ab89143d578ec5618b0470471d'


def read(path):
    return json.loads(Path(path).read_text())


def entry(path, base=Path('.')):
    path = Path(path)
    digest = hashlib.sha256()
    count = 0
    with path.open('rb') as stream:
        while block := stream.read(1 << 20):
            digest.update(block)
            count += len(block)
    return dict(path=path.relative_to(base).as_posix(), byteLength=count, sha256=digest.hexdigest())


assert entry(parent_path)['sha256'] == '2ea55954f90d2c5d3a22b6187723bccb92da71ffa043944a03dcb5324d9f2771'
parent = read(parent_path)
explicit = set(read(stage / 'changed-product-files.json'))
prior = {v['path']: v for v in parent['productPrivateFiles']}
changed = {name for name, value in prior.items() if entry(product / name, product) != value}
assert changed <= explicit, changed - explicit
private_names = sorted(set(prior) | explicit)
for name in explicit:
    if name.endswith('.rs'):
        assert len((product / name).read_text().splitlines()) < 2000, name

vendor = product / 'apps/agent-runtime/vendor/musteroffice' / sdk
manifest_path = vendor / 'sdk-manifest.json'
assert entry(manifest_path, product)['sha256'] == sdk
sdk_manifest = read(manifest_path)
for row in sdk_manifest['files']:
    assert entry(vendor / row['path'], vendor) == row, row['path']
environment = read(stage / 'environment.json')
old_environment = read(parent['environment']['path'])
assert entry(product / 'apps/agent-runtime/Cargo.lock', product) == environment['productLock'] == old_environment['productLock']
assert entry(environment['worker']['path']) == environment['worker']
assert environment['worker']['sha256'] == '1151581c9b689d8329ceec6100aed4103d1f78e38ac178c894ebaf63bfede50f'
assert environment['performanceMeasurement'] is False


def tests(name, expected):
    log = (stage / name).read_text()
    assert 'FAILED' not in log and 'error:' not in log, name
    names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
    assert len(names) == expected and 'test result: ok.' in log, (name, len(names))
    binaries = re.findall(r'Running [^\n]+ \((target/debug/deps/[^)]+)\)', log)
    return names, [entry(product / 'apps/agent-runtime' / p, product) for p in binaries]


export_names, export_bins = tests('test-real-budget-final.log', 5)
native_names, native_bins = tests('native-mutation-regression.log', 11)
library_names, library_bins = tests('library-tests.log', 82)
bridge_names, bridge_bins = tests('real-bridge-regression.log', 1)
for name in ['clippy-scoped.log', 'workspace-check.log']:
    log = (stage / name).read_text()
    assert 'Finished `dev` profile' in log and 'error:' not in log, name
assert 'warning:' not in (stage / 'clippy-scoped.log').read_text()
assert not (stage / 'format-and-diff-check.log').read_bytes()
assert 'clippy::collapsible_if' in (stage / 'clippy.log').read_text()
lint = read(stage / 'unchanged-dependency-lint.json')
assert entry(product / lint['path'], product)['sha256'] == lint['sha256']
assert '2 passed; 2 failed' in (stage / 'test-real-first.log').read_text()
assert '4 passed; 0 failed' in (stage / 'test-real-final.log').read_text()

actual = stage / 'actual-budget-final'
baseline = Path('.codex-work/embedded-sdk/actual-product')
files = read(actual / 'files.json')
assert files == read(baseline / 'files.json') and len(files) == 12
projection = read(actual / 'manifest.json')
assert projection['version'] == 'presentation-artifact/4'
assert len(projection['assets']) == 12 and len(projection['pages']) == 2
for asset, row in zip(projection['assets'], files):
    path = actual / row['file']
    assert path.read_bytes() == (baseline / row['file']).read_bytes()
    assert entry(path)['sha256'] == row['asset']['sha256'] == asset['content']['sha256']
    assert str(entry(path)['byteLength']) == row['asset']['byteLength'] == asset['content']['byte_length']
    assert asset['id'] == row['asset']['id']
    assert asset['role'] == row['asset']['role']
    assert asset['content']['media_type'] == row['asset']['mediaType']
for name in ['receipt.json', 'inspection.json']:
    assert read(actual / name) == read(baseline / name)
receipt = read(actual / 'receipt.json')
assert projection['claims'] == receipt['bundle']['claims']
assert [(c['kind'], c['status']) for c in projection['claims']] == [
    ('structure', 'passed'), ('layout', 'not_proven'), ('native-editability', 'not_proven'),
    ('playback', 'not_proven'), ('target-application', 'not_proven')]
bundle = json.dumps(receipt['bundle'], separators=(',', ':'), ensure_ascii=False).encode()
assert hashlib.sha256(bundle).hexdigest() == projection['bundle']['sha256']
assert str(len(bundle)) == projection['bundle']['byte_length']
for row in files:
    assert (stage / 'bridge-regression' / row['file']).read_bytes() == (baseline / row['file']).read_bytes()

sources = ['README.md', 'docs/README.md', 'docs/implementation/progress.md',
           'docs/implementation/product-native-export.md',
           'docs/design/implementation/musterwork-adapter-spec.md',
           'tools/verification/product-native-export-evidence.py']
links = 0
for name in sources:
    path = Path(name)
    if path.suffix != '.md':
        continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            resolved = (path.parent / target).resolve()
            assert resolved.exists() or resolved == output.resolve(), (name, target)
            links += 1

report = dict(
    format='musteroffice.product-native-export-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sources], localLinksChecked=links,
    productPrivateFiles=[entry(product / p, product) for p in private_names],
    explicitProductChanges=sorted(explicit), changedPriorProductFiles=sorted(changed),
    sdkManifest=entry(manifest_path, product), environment=entry(stage / 'environment.json'),
    tests=dict(exportPassed=5, exportNames=export_names, nativeDraftPassed=11, nativeNames=native_names,
               libraryPassed=82, libraryNames=library_names, bridgePassed=1, bridgeNames=bridge_names,
               scopedProductionStrictClippy=True, fullDependencyStrictClippy=False,
               productWorkspaceAllTargetsCompile=True, wholeProductTestSuite=False),
    finalAssetCount=12, pageCount=2, identicalToHistoricalDelivery=True,
    separateClaimsPreserved=True, actualFinalStoredBytesInspected=True,
    pendingOutputReadRetainsComputeReservation=True,
    allOutputMetadataIncludedInAdmission=True, inputAliasesAndMetadataShareBudget=True,
    candidateArtifactCommitImplemented=False, productionToolRouteChanged=False,
    vendoredSdkWorkerAndProductCargoLockUnchanged=True, newExternalComponent=False,
    executables=export_bins + native_bins + library_bins + bridge_bins,
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'This is actual private export preparation, not native Artifact commit, checkpoint recovery or a production Agent route.',
        'Invocation prewrite retention, crash spool recovery and complete RSS/disk/handle budgets remain pending.',
        'Failed or cancelled computations can leave private content; deterministic identities must not be blindly deleted.',
        'Quality declarations are preserved; Office/WPS application validation, full advanced content and E0-E3 remain open.',
        'Viewer/Player, historical migration and full product replacement remain incomplete.',
        'The unchanged capability dependency has a collapsible_if lint failure; strict no-deps checks cover only the three named production libraries.',
        'Core work outside this adapter stage is not revalidated by the pinned vendored SDK or this product evidence.',
        'No installer size, cross-platform runtime or performance conclusion is established by these tests.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(productFiles=len(private_names), links=links, exportPassed=5,
                      nativePassed=11, libraryPassed=82, bridgePassed=1, assets=12)))
