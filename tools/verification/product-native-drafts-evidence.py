"""Bind native product draft transactions without copying private source."""
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
stage = Path('.codex-work/product-native-drafts')
parent_path = Path('docs/reviews/evidence/2026-09-27-product-sdk-bridge-verification.json')
output = Path('docs/reviews/evidence/2026-09-27-product-native-drafts-verification.json')


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


assert entry(parent_path)['sha256'] == '65f9270bce7546a02efc27645a355791ebd7deeffc37fa31c3f86cca938748ec'
parent = read(parent_path)
prior = {v['path']: v for v in parent['sourceFiles']}
changed = {name for name, value in prior.items() if entry(name) != value}
assert changed == {'docs/README.md', 'docs/implementation/progress.md',
                   'docs/design/implementation/musterwork-adapter-spec.md'}, changed
added = {'docs/implementation/product-native-drafts.md', 'tools/verification/product-native-drafts-evidence.py'}
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
    assert len((product / p).read_text().splitlines()) <= 2000, p


def tests(name):
    text = (stage / name).read_text()
    assert 'FAILED' not in text and 'error:' not in text, name
    names = re.findall(r'^test (.+) \.\.\. ok$', text, re.M)
    assert names and 'test result: ok.' in text, name
    binaries = re.findall(r'Running [^\n]+ \((target/debug/deps/[^)]+)\)', text)
    return names, [entry(product / 'apps/agent-runtime' / p, product) for p in binaries]


native, native_bins = tests('sqlite-tests-final.log')
assert len(native) == 44 and sum('office_drafts::' in n for n in native) == 5
assert '44 passed; 0 failed; 3 ignored' in (stage / 'sqlite-tests-final.log').read_text()
libraries, lib_bins = tests('library-tests-final.log')
for name in ['clippy-final.log', 'workspace-check-final.log']:
    log = (stage / name).read_text()
    assert 'Finished `dev` profile' in log and 'error:' not in log, name
assert 'warning:' not in (stage / 'clippy-final.log').read_text()
assert not (stage / 'fmt-final.log').read_bytes()
assert 'Invalid presentation draft transition' in (stage / 'sqlite-tests-2.log').read_text()
assert '3 passed; 1 failed' in (stage / 'sqlite-tests-2.log').read_text()

# No new dependency selection, schema migration or kernel computation changes.
environment = read(stage / 'environment.json')
assert entry('Cargo.lock') == environment['coreLock']
assert entry(product / 'apps/agent-runtime/Cargo.lock', product) == environment['productLock']
previous_environment = read(parent['environment']['path'])
assert environment['coreLock'] == previous_environment['coreLock']
assert environment['productLock'] == previous_environment['productLock']
assert entry(previous_environment['worker']['path']) == previous_environment['worker']

report = dict(
    format='musteroffice.product-native-drafts-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed), addedSources=sorted(added),
    localLinksChecked=links, productPrivateFiles=private, explicitProductChanges=sorted(explicit),
    changedPriorProductFiles=sorted(changed_private), environment=entry(stage / 'environment.json'),
    tests=dict(sqlitePassed=len(native), sqliteIgnored=3, nativeDraftTests=5, sqliteNames=native,
               libraryPassed=len(libraries), libraryNames=libraries, scopedStrictClippy=True,
               productWorkspaceAllTargetsCompile=True, wholeProductTestSuite=False),
    contracts=['presentation-draft-state/2', 'presentation-authoring-checkpoint/2'],
    databaseMigrationAdded=False, originalAttemptToolLedgerOwner=True,
    kernelAndWorkerComputingCodeUnchanged=True, productLockUnchanged=True,
    executables=native_bins + lib_bins,
    artifacts=[entry(p) for p in sorted(stage.rglob('*')) if p.is_file()],
    limitations=[
        'SQLite private native draft begin/advance/abandon commit is implemented; production native generation and Artifact export commit are still pending.',
        'Metadata and ContentRef authorization checks do not prove SnapshotRecord semantic correctness or actual resource closure; production preparation must verify bytes.',
        'The edit persistence test retains the native snapshot and changes host revision; it is not a real kernel edit execution.',
        'Legacy shared authoring tools reject native operations. Native tool successor, shared Home authorization and PostgreSQL native writes are not implemented.',
        'No new job owner; Invocation quotas and worker spool crash recovery remain incomplete.',
        'No browser export test, Viewer/Player, full advanced content, Office/WPS editing, platform execution, installer or E0-E3 acceptance.',
    ],
)
serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
if args.check:
    assert output.read_text() == serialized
else:
    with output.open('x') as stream:
        stream.write(serialized)
print(json.dumps(dict(coreSources=len(sources), productFiles=len(private), links=links,
                      sqlitePassed=len(native), nativeTests=5, libraryPassed=len(libraries))))
