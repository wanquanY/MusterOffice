"""Bind actual product SDK/content bridging without importing private source."""
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
stage = Path('.codex-work/product-sdk-bridge')
output = Path('docs/reviews/evidence/2026-09-27-product-sdk-bridge-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-27-product-content-range-verification.json')
SDK = 'f3d44dca6f1f2632832eb3327b71d38915c354ab89143d578ec5618b0470471d'


def read(path):
    return json.loads(Path(path).read_text())


def entry(path, base=Path('.')):
    path = Path(path)
    sha = hashlib.sha256()
    length = 0
    with path.open('rb') as stream:
        while data := stream.read(1 << 20):
            sha.update(data)
            length += len(data)
    return dict(path=path.relative_to(base).as_posix(), byteLength=length, sha256=sha.hexdigest())


assert entry(parent_path)['sha256'] == '82fb84c07b43582cda7b7bfca8ee646a2c3b23a4c92e762a631e47bd6dd8e303'
parent = read(parent_path)
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {name for name, value in prior.items() if entry(name) != value}
assert changed == {'docs/README.md', 'docs/implementation/progress.md', 'tools/sdk/build.py'}, changed
added = {'docs/implementation/product-sdk-bridge.md', 'tools/verification/product-sdk-build-layout.py',
         'tools/verification/product-sdk-bridge-evidence.py'}
sources = set(prior) | added
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in {'.rs', '.py', '.mjs', '.ts', '.cpp', '.h'}:
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

private = read(stage / 'product-source.json')['files']
for item in private:
    assert entry(product / item['path'], product) == item, item['path']
    if '/infrastructure/musteroffice/' in item['path'] and item['path'].endswith('.rs'):
        assert len((product / item['path']).read_text().splitlines()) < 2000

manifest = read(stage / 'sdk/sdk-manifest.json')
assert entry(stage / 'sdk/sdk-manifest.json')['sha256'] == SDK
assert len(manifest['libraries']) == 22 and len(manifest['files']) == 434
vendored = product / 'apps/agent-runtime/vendor/musteroffice' / SDK
assert (vendored / 'sdk-manifest.json').read_bytes() == (stage / 'sdk/sdk-manifest.json').read_bytes()
sdk_changes = []
old_sdk = Path('.codex-work/embedded-sdk/sdk-final')
for row in manifest['files']:
    for root in [stage / 'sdk', stage / 'sdk-repeat', vendored]:
        assert entry(root / row['path'], root) == row, row['path']
    if (old_sdk / row['path']).read_bytes() != (stage / 'sdk' / row['path']).read_bytes():
        sdk_changes.append(row['path'])
assert sdk_changes == sorted(f'crates/{name}/Cargo.toml' for name in manifest['libraries'])
assert (stage / 'sdk.tar.gz').read_bytes() == (stage / 'sdk-repeat.tar.gz').read_bytes()
lock = read(product / 'components/musteroffice/lock.json')
assert lock['sdkManifestSha256'] == SDK

environment = read(stage / 'environment.json')
assert entry('Cargo.lock') == environment['coreLock']
assert entry(product / 'apps/agent-runtime/Cargo.lock', product) == environment['productLock']
worker = environment['worker']
assert entry(worker['path']) == worker
assert worker['sha256'] == '1151581c9b689d8329ceec6100aed4103d1f78e38ac178c894ebaf63bfede50f'
assert lock['workers']['darwin-arm64'] == {k: worker[k] for k in ['sha256', 'byteLength']}


def rust_tests(name, expected, allow_failure=False):
    log = (stage / name).read_text()
    names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
    assert len(names) == expected, (name, len(names))
    if not allow_failure:
        assert 'FAILED' not in log and 'error:' not in log
    paths = re.findall(r'Running [^\n]+ \((target/debug/deps/[^)]+)\)', log)
    return names, [entry(product / 'apps/agent-runtime' / p, product) for p in paths]


names, binaries = rust_tests('bridge-tests-6.log', 83)
real_names, real_binaries = rust_tests('real-export-final.log', 1)
architecture_names, architecture_binaries = rust_tests('architecture-tests.log', 27, True)
assert '27 passed; 2 failed; 1 ignored' in (stage / 'architecture-tests.log').read_text()
assert '15' in (stage / 'preparation-tests-final.log').read_text()
assert re.search(r'pass 15\b', (stage / 'preparation-tests-final.log').read_text())
assert re.search(r'fail 0\b', (stage / 'preparation-tests-final.log').read_text())
assert 'Ran 6 tests' in (stage / 'sdk-verifier-tests.log').read_text()
assert '\nOK\n' in (stage / 'sdk-verifier-tests.log').read_text()
for name in ['clippy-final.log', 'workspace-check-final.log', 'windows-check.log']:
    log = (stage / name).read_text()
    assert 'Finished `dev` profile' in log and 'error:' not in log, name
assert 'warning:' not in (stage / 'clippy-final.log').read_text()
assert not (stage / 'fmt-final.log').read_bytes()

layout = read(stage / 'source-only-build-final.json')
assert layout['sdkLibraries'] == 22 and layout['nativeAdapterLibraryCompiled']
assert layout['noIgnoredSdkOrWorker'] and layout['noCoreCheckoutPath']
for item in layout['sourceFiles']:
    assert entry(product / item['path'], product) == item, item['path']
assert entry(product / layout['log']['path'], product) == layout['log']
graph = read(stage / 'sdk-product-graph.json')
assert len(graph['registryPackages']) == 50 and not graph['sdkLibrariesAreProductMembers']
assert graph['sdkLibraries'] == manifest['libraries']
baseline = read(stage / 'baseline-findings.json')
for item in baseline['unchangedSources']:
    assert entry(product / item['path'], product)['sha256'] == item['sha256']

actual = stage / 'actual-final'
previous = Path('.codex-work/embedded-sdk/actual-product')
files = read(actual / 'files.json')
assert len(files) == 12 and files == read(previous / 'files.json')
for item in files:
    path = actual / item['file']
    assert path.read_bytes() == (previous / item['file']).read_bytes()
    assert entry(path)['sha256'] == item['asset']['sha256']
    assert path.stat().st_size == int(item['asset']['byteLength'])
assert (actual / 'receipt.json').read_bytes() == (previous / 'receipt.json').read_bytes()
assert read(actual / 'inspection.json') == read(previous / 'inspection.json')
windows_meta = list((product / 'apps/agent-runtime/target/x86_64-pc-windows-msvc/debug/deps').glob('libmusterwork_agent_musteroffice-*.rmeta'))
assert windows_meta

report = dict(
    format='musteroffice.product-sdk-bridge-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed), addedSources=sorted(added),
    localLinksChecked=links, productPrivateFiles=private, environment=entry(stage / 'environment.json'),
    sdkManifestSha256=SDK, sdkLibraries=22, sdkChangedFiles=sdk_changes,
    sdkComputingSourcesAndDataUnchanged=True, workerAndKernelLockUnchanged=True,
    actualProductSdkRegistryPackages=graph['registryPackages'], productDependencyDelta=read(stage / 'dependency-delta.json'),
    sourceOnlyProductBuild=entry(stage / 'source-only-build-final.json'),
    tests=dict(bridge=5, content=26, artifact=52, names=names, realWorker=1, realNames=real_names,
               preparation=15, sdkVerifier=6, architecturePassed=27, architectureFailed=2, architectureIgnored=1,
               architectureNames=architecture_names, scopedAdapterAllTargetsStrictClippy=True,
               productWorkspaceAllTargetsCompile=True, wholeProductTestSuite=False),
    baselineFindings=baseline, executables=binaries + real_binaries + architecture_binaries,
    windowsMetadata=[entry(p, product) for p in sorted(windows_meta)],
    actualFiles=read(stage / 'file-comparison-final.json'),
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'The real path is SQLite -> SDK/worker -> SQLite with final inspection, not product Invocation/Artifact commit or UI routing.',
        'Input-set and 64 KiB output bounds do not implement aggregate job, renderer, FD or disk quotas or host-crash spool ownership.',
        'Windows evidence compiles the adapter SDK library; it does not execute a Windows worker or SQLite file operations.',
        'The product host resolves a recorded 50-package feature graph, not the standalone SDK 48-package lock graph.',
        'Workspace compilation contains baseline test warnings. Architecture has two verified baseline-source failures; broad dependency lint is not fully passing.',
        'No Docker build, installer, overall performance/RSS, complete advanced content, Office/WPS editing, history or E0-E3 acceptance.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(private), links=links, assets=12,
                      tests=83, realWorkerTests=1, sourceOnlyBuild=True, architectureFailures=2)))
