"""Bind actual product kernel mutations and same-owner transaction evidence."""
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
stage = Path('.codex-work/product-native-mutations')
parent_path = Path('docs/reviews/evidence/2026-09-27-product-native-drafts-verification.json')
output = Path('docs/reviews/evidence/2026-09-27-product-native-mutations-verification.json')


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


assert entry(parent_path)['sha256'] == '0cb403fb4c55adbafeab2fa9f01e6f47232ac7bd072a365f4dc1973fc832124f'
parent = read(parent_path)
prior = {v['path']: v for v in parent['sourceFiles']}
inherited = set(read(stage / 'observed-source-delta.json')['coreBeforeDocumentationEdits'])
assert inherited == {'tools/experiments/mcp-sdk-probe/Cargo.lock', 'tools/mo-mcp/Cargo.lock'}
owned_changes = {'README.md', 'docs/README.md', 'docs/implementation/progress.md',
                 'docs/design/implementation/musterwork-adapter-spec.md'}
changed = {name for name, value in prior.items() if entry(name) != value}
assert changed == owned_changes | inherited, changed
added = {'docs/implementation/product-native-mutations.md',
         'tools/verification/product-native-mutations-evidence.py'}
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

previous_private = {v['path']: v for v in parent['productPrivateFiles']}
explicit = set(read(stage / 'changed-product-files.json'))
private_names = set(previous_private) | explicit
private = [entry(product / p, product) for p in sorted(private_names)]
changed_private = {p for p, old in previous_private.items() if entry(product / p, product) != old}
assert changed_private <= explicit, changed_private - explicit
for p in explicit:
    if Path(p).suffix == '.rs':
        assert len((product / p).read_text().splitlines()) <= 2000, p


def tests(name):
    log = (stage / name).read_text()
    assert 'FAILED' not in log and 'error:' not in log, name
    names = re.findall(r'^test (.+) \.\.\. ok$', log, re.M)
    assert names and 'test result: ok.' in log, name
    binaries = re.findall(r'Running [^\n]+ \((target/debug/deps/[^)]+)\)', log)
    return names, [entry(product / 'apps/agent-runtime' / p, product) for p in binaries]


native, native_bins = tests('native-tests-final.log')
assert len(native) == 11 and sum('office_drafts::computed::' in n for n in native) == 6
assert '11 passed; 0 failed; 0 ignored' in (stage / 'native-tests-final.log').read_text()
broader = (stage / 'sqlite-tests-before-cas-fix.log').read_text()
assert '49 passed; 1 failed; 3 ignored' in broader
assert 'left: InvalidCommand' in broader and 'right: Conflict' in broader
broader_failures = re.findall(r'^test (.+) \.\.\. FAILED$', broader, re.M)
assert broader_failures == ['presentation_authoring::office_drafts::computed::computed_create_and_atomic_edit_commit_real_models_receipts_and_recovery']
assert broader_failures[0] in native
libraries, lib_bins = tests('library-tests-final.log')
assert len(libraries) == 82
for name in ['clippy-final.log', 'workspace-check-final.log']:
    log = (stage / name).read_text()
    assert 'Finished `dev` profile' in log and 'error:' not in log, name
assert 'warning:' not in (stage / 'clippy-final.log').read_text()
assert not (stage / 'fmt-final.log').read_bytes()
assert 'argument #1' in (stage / 'tests-first.log').read_text()
assert '0 passed; 6 failed' in (stage / 'tests-second.log').read_text()
assert 'Authorization' in (stage / 'tests-third.log').read_text()
assert '3 passed; 3 failed' in (stage / 'tests-third.log').read_text()

environment = read(stage / 'environment.json')
assert entry('Cargo.lock') == environment['coreLock']
assert entry(product / 'apps/agent-runtime/Cargo.lock', product) == environment['productLock']
previous_environment = read(parent['environment']['path'])
assert environment['coreLock'] == previous_environment['coreLock']
dependencies = read(stage / 'dependency-delta.json')
assert dependencies['registryUnchangedSinceSdkBridge'] is True
assert dependencies['newExternalComponent'] is False

report = dict(
    format='musteroffice.product-native-mutations-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    inheritedCoreChanges=sorted(inherited), addedSources=sorted(added), localLinksChecked=links,
    productPrivateFiles=private, explicitProductChanges=sorted(explicit),
    changedPriorProductFiles=sorted(changed_private), environment=entry(stage / 'environment.json'),
    dependencyDelta=entry(stage / 'dependency-delta.json'),
    tests=dict(finalNativePassed=len(native), finalNativeIgnored=0, computationTests=6, nativeNames=native,
               broaderBeforeCasFix=dict(passed=49, failed=1, ignored=3, resolvedFailures=broader_failures),
               libraryPassed=len(libraries), libraryNames=libraries, realWorkerTestRun=False,
               scopedProductionStrictClippy=True, productWorkspaceAllTargetsCompile=True,
               wholeProductTestSuite=False),
    contract='presentation-authoring-checkpoint/3', nativeDraftContractUnchanged=True,
    originalAttemptToolLedgerOwner=True, actualKernelCreateAndApply=True,
    databaseMigrationAdded=False, defaultProductRouteChanged=False,
    kernelSdkAndWorkerComputingCodeUnchanged=True,
    executables=native_bins + lib_bins,
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'Real SDK preparation and SQLite computed draft commits are implemented; production tool wiring and shared Home authorization are pending.',
        'The transaction fixture uses the historical test tool identity; it does not prove a production native Agent route.',
        'Prepared private writes are not committed outputs. Invocation retention, cancellation during writes and crash orphan recovery require the original Runtime owner.',
        'Compute semaphore and material bounds do not establish whole-operation RSS, disk or handle budgets.',
        'Native export candidate/Artifact commit, PostgreSQL native writes, Viewer/Player and history migration remain incomplete.',
        'Full advanced content, Office/WPS editing, cross-platform execution, performance, installer and E0-E3 acceptance remain open.',
        'Two core MCP lock files already differed from the parent evidence in the clean starting checkout; this stage does not claim to revalidate those adapters.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(private), links=links,
                      finalNativePassed=len(native), computationTests=6, libraryPassed=len(libraries))))
