"""Bind native MCP recovery, pipe shutdown, cancellation and business regressions."""
import hashlib
import json
from pathlib import Path
import re
import sqlite3
from contextlib import closing
import sys
from urllib.parse import unquote

root = Path('.codex-work/mcp-recovery')
output = Path('docs/reviews/evidence/2026-09-26-mcp-recovery-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-mcp-response-metadata-verification.json')


def entry(path):
    p = Path(path); raw = p.read_bytes()
    return dict(path=str(p), byteLength=len(raw), sha256=hashlib.sha256(raw).hexdigest())


def read(path): return json.loads(Path(path).read_text())


assert entry(parent_path)['sha256'] == '8798b73ea0ab0d02c65e0b6ade62de75d7c7e5e71ec7ea342ca57cf0d5026ce9'
parent = read(parent_path); prior = {e['path']:e for e in parent['sourceFiles']}
changed = {p for p, value in prior.items() if entry(p) != value}
expected_changes = {'docs/implementation/progress.md', 'docs/implementation/mcp-stdio.md',
    'tools/mo-mcp/README.md', 'tools/mo-mcp/Cargo.toml', 'tools/mo-mcp/Cargo.lock',
    'tools/mo-mcp/check_transport.py', 'tools/mo-mcp/src/main.rs', 'tools/mo-mcp/src/resources.rs',
    'tools/mo-mcp/src/transport.rs', 'tools/mo-mcp/src/transport/tests.rs'}
assert changed == expected_changes, changed
sources = set(prior) | {'docs/implementation/mcp-recovery.md', 'tools/verification/mcp-recovery-evidence.py'}
sources |= {str(p) for p in Path('tools/mo-mcp/src').rglob('*.rs')}
sources |= {str(p) for p in Path('tools/mo-mcp').glob('*.py')}
sources |= {str(p) for p in Path('components/io-runtime').rglob('*') if p.is_file()}
links = 0
for name in sorted(sources):
    p = Path(name)
    if p.suffix in ['.rs', '.py', '.ts', '.mjs', '.cpp', '.h']: assert len(p.read_text().splitlines()) <= 2000
    if p.suffix != '.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'): continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (p.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

binary = entry(root/'final/mo-mcp')
assert binary['byteLength'] == 12545808
assert binary['sha256'] == 'fceca102c83729e5ad77570df99de6c7a479fb1ba831b3c45eace09f20850740'
reference_path = root/'reference-final/report.json'; reference = read(reference_path)
assert len(reference['calls']) == 172
assert [(s['era'],s['exportedAssets']) for s in reference['sessions']] == [('legacy',12),('modern',12)]
for name in ['reference-final', 'transport-final', 'lifecycle-final', 'cancellation-final']:
    assert read(root/name/'report.json')['binary']['sha256'] == binary['sha256'], name
old_reference = read(parent['reference']['path'])
assert reference['worker'] == old_reference['worker']
assert entry(reference['worker']['path']) == reference['worker']

transport = read(root/'transport-final/report.json')
assert len(transport['recovery']) == 18 and len(transport['cases']) == 3
assert transport['realDocumentsCreated'] == 2 and transport['binaryAuthorization'] and transport['invalidConfigRejected']
for era in ['legacy','modern']:
    frames = read(root/f'transport-final/{era}.frames.json')
    errors = [f['error']['code'] for f in frames if 'error' in f]
    assert errors.count(-32700) == 4 and errors.count(-32600) == 4 and errors.count(-32602) == 2
    assert errors.count(-32601) == (4 if era == 'legacy' else 5)

lifecycle = read(root/'lifecycle-final/report.json')
assert len(lifecycle['records']) == 4
for record in lifecycle['records']:
    assert record['exitCode'] == 1 and record['inputKeptOpen']
    assert 0 < record['elapsedSeconds'] < 16
    if record['mode'] == 'unread': assert record['elapsedSeconds'] >= 9
    path = root/'lifecycle-final'/f"{record['era']}-{record['mode']}.stderr"
    assert b'response write failed or timed out' in path.read_bytes()
before = read(root/'closed-output-before/report.json')
assert before['outputClosedInputOpen'] and before['didNotExitWithinSeconds'] == 12 and before['eventualExit'] == 1
assert entry(before['binary']) == parent['releaseBinary']

cancel = read(root/'cancellation-final/report.json')
assert len(cancel['records']) == 2
for record in cancel['records']:
    assert record['cancelledProtocolRequests'] == 11 and record['cancellationRequested'] is False
    assert record['nativePermitHeldAfterProtocolCancellation'] and record['retryAndReconnectSameReceipt']
    db = root/'cancellation-final'/f"{record['era']}.sqlite"
    with closing(sqlite3.connect(db.resolve().as_uri()+'?mode=ro&immutable=1', uri=True)) as conn:
        rows = conn.execute('SELECT request_id,info FROM jobs').fetchall()
        assert len(rows) == 1 and rows[0][0] == record['businessRequestId']
        info = json.loads(rows[0][1]); assert info['id'] == record['jobId']
        assert info['state'] == 'succeeded' and info['cancelRequested'] is False
        assert conn.execute('SELECT count(*) FROM heads').fetchone()[0] == 1
        assert conn.execute('SELECT count(*) FROM revisions').fetchone()[0] == 1
    frames = read(root/f"cancellation-final/{record['era']}.frames.json")
    requests = [json.loads(line) for line in (root/f"cancellation-final/{record['era']}.input.bin").read_bytes().splitlines() if line != b'{']
    cancelled_ids = [r['params']['requestId'] for r in requests if r['method'] == 'notifications/cancelled']
    assert len(cancelled_ids) == 11 and not set(cancelled_ids) & {r.get('id') for r in frames}
assert 'modern: unexpected EOF' in (root/'cancellation-before-fix.log').read_text()
assert 'request identity or in-flight budget' in (root/'cancellation-before-fix/modern.stderr').read_text()

files_path = root/'file-checks-final.json'; files = read(files_path)
assert files['files'] == read(parent['downloadedFiles']['path'])['files']
assert sum(len(f['checkedParts']) for f in files['files']) == 20
tests = re.findall(r'^test (.+) \.\.\. ok$', (root/'tests-3.log').read_text(), re.M)
assert len(tests) == len(set(tests)) == 18
checks = []
for name in ['tests-3','clippy-3','release-3','fmt','transport-final','lifecycle-final',
             'cancellation-final','reference-final','file-checks-final','components','components-check']:
    log = root/f'{name}.log'; text = log.read_text()
    assert not any(bad in text for bad in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
    if name in ['tests-3','clippy-3','release-3']: assert 'Finished' in text
    checks.append(dict(name=name, exitCode=0, log=entry(log)))
for name in ['reference-final','transport-final','cancellation-final']:
    for p in (root/name).glob('*.stderr'):
        if p.stem in ['oversize','unterminated','unsolicited-response','read','invalid-config']: continue
        assert not p.read_bytes(), str(p)

components = read('components/io-runtime/component.json')
metadata = read(root/'cargo-metadata.json')
registry_count = sum(p['source'] is not None for p in metadata['packages'])
assert registry_count == 110
for component in components['components']:
    for license in component['licenses']:
        assert entry(Path('components/io-runtime')/license['path'])['sha256'] == license['sha256']
report = dict(format='musteroffice.mcp-recovery-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(sources-set(prior)), sourceLinksChecked=links,
    checks=checks, adapterTestNames=tests, releaseBinary=binary, worker=reference['worker'],
    reference=entry(reference_path), wireRequests=172, recovery=entry(root/'transport-final/report.json'),
    lifecycle=entry(root/'lifecycle-final/report.json'), cancellation=entry(root/'cancellation-final/report.json'),
    previousClosedOutputFailure=entry(root/'closed-output-before/report.json'),
    previousCancellationBudgetFailure=entry(root/'cancellation-before-fix.log'),
    downloadedFiles=entry(files_path), previousFileAndPixelChecksUnchanged=True,
    ioComponents=entry('components/io-runtime/component.json'), registryPackagesIncludingTargetBranches=registry_count,
    artifacts=[entry(p) for p in sorted(root.rglob('*')) if p.is_file()],
    specifications=['https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio',
        'https://modelcontextprotocol.io/specification/2026-07-28/basic',
        'https://modelcontextprotocol.io/specification/2025-11-25/basic'],
    limitations=['Unix pipe shutdown verified only on macOS arm64; Windows blocking stdio remains unaccepted.',
        'Lifecycle samples use an idle native host. Active computation still drains; no universal exit-time or RSS guarantee.',
        'SDK cancellation test covers a writer-blocked native call and durable retries, not every renderer or failure phase.',
        'Core/contracts unchanged. No new full-workspace, Native/WASM, Office/WPS, product benchmark or package acceptance.',
        'HTTP, Tasks extension, SDK/Skill/Plugin, full advanced presentation content and Musterwork replacement remain incomplete.'])
encoded = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv: assert output.read_text() == encoded
elif '--dry-run' not in sys.argv:
    assert not output.exists(), 'never overwrite historical evidence'
    output.write_text(encoded)
print(json.dumps(dict(sourceFiles=len(sources), checks=len(checks), adapterTests=len(tests),
    wireRequests=172, recoveredFrames=18, cancellationRequests=22, output=str(output), check='--check' in sys.argv)))
