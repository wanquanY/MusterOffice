"""Bind native scheduling, genuine migrations and runtime checks to sources."""
import gzip
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/native-scheduler')
output = Path('docs/reviews/evidence/2026-09-26-native-scheduler-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-mcp-sdk-evaluation.json')
discovery_path = Path('docs/reviews/evidence/2026-09-26-operation-discovery-verification.json')

def entry(path):
    p = Path(path); data = p.read_bytes()
    return dict(path=str(p), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

assert entry(parent_path)['sha256'] == '99db7c101b35aeddda0f8977fa7d3b15062ce55a578e869457c6fd19eecd5773'
assert entry(discovery_path)['sha256'] == 'f7dd25e4877725b8233220e2ae09f7653634b4117cd5b9002cf7165b29d55c08'
parent = json.loads(parent_path.read_text()); discovery = json.loads(discovery_path.read_text())
prior = {r['path']: r for r in parent['sourceFiles']}
changed = {p for p, r in prior.items() if entry(p) != r}
allowed = set('''Cargo.lock
README.md
crates/mo-operation-service/src/discovery.rs
crates/mo-operation-service/src/host.rs
crates/mo-standard-host/Cargo.toml
crates/mo-standard-host/src/db.rs
crates/mo-standard-host/src/exports.rs
crates/mo-standard-host/src/jobs.rs
crates/mo-standard-host/src/lib.rs
crates/mo-standard-host/src/results.rs
crates/mo-standard-host/src/results/storage.rs
crates/mo-standard-host/tests/exports.rs
crates/mo-standard-host/tests/resource_recovery.rs
docs/README.md
docs/design/agent-interfaces.md
docs/implementation/dependencies.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-host/src/main.rs'''.splitlines())
for stem in ['host-capabilities', 'host-response']:
    allowed |= {f'contracts/generated/{stem}.schema.json', f'packages/contracts/src/generated/{stem}.ts'}
assert changed == allowed, (changed-allowed, allowed-changed)
added = set('''crates/mo-standard-host/src/execution.rs
crates/mo-standard-host/src/queue.rs
crates/mo-standard-host/src/runtime.rs
crates/mo-standard-host/src/runtime/config.rs
crates/mo-standard-host/src/runtime/scheduler.rs
crates/mo-standard-host/tests/scheduling.rs
crates/mo-standard-host/tests/scheduling/crash.rs
crates/mo-standard-host/tests/exports/runtime.rs
docs/implementation/native-scheduler.md
tools/verification/native-scheduler-workspace.py
tools/verification/native-scheduler-reference.py
tools/verification/native-scheduler-parity.py
tools/verification/native-scheduler-evidence.py'''.splitlines())
assert not added & set(prior)
sources = set(prior) | added
history = json.loads((root/'workspace.json').read_text())
checks = {c['name']: c for c in history}
assert set(checks) == {'fmt', 'tests', 'clippy', 'native-build', 'schema-check', 'types-check',
    'pure-operation-wasm', 'rust-wasm', 'bindgen', 'native-integration', 'export-integration',
    'native-tree', 'wasm-tree', 'release-build', 'final-fmt', 'final-host-tests', 'final-host-clippy'}
for c in checks.values():
    assert c['exitCode'] == 0
    assert not any(bad in Path(c['log']).read_text() for bad in ['error:', 'FAILED', 'Traceback', 'AssertionError'])

def tests(name):
    return re.findall(r'^test (.+) \.\.\. ok$', Path(checks[name]['log']).read_text(), re.M)

assert len(tests('tests')) == 837
assert set(discovery['rustTestNames']) <= set(tests('tests'))
assert len(tests('final-host-tests')) == 61
assert set(tests('final-host-tests')) <= set(tests('tests'))
assert len(tests('native-integration')) == 2 and len(tests('export-integration')) == 9
assert 'runtime::busy_real_renderer_keeps_control_available_and_sync_inside_worker_budget' in tests('export-integration')
assert 'crash::killed_scheduler_recovers_lease_and_executes_remaining_queue' in tests('final-host-tests')
for name in ['native-integration', 'export-integration']:
    assert entry(checks[name]['worker']['path'])['sha256'] == checks[name]['worker']['sha256']
assert len(re.findall(r'^check contracts/generated/', Path(checks['schema-check']['log']).read_text(), re.M)) == 109
assert len(re.findall(r'^check .+\.ts$', Path(checks['types-check']['log']).read_text(), re.M)) == 109
for name in ['native-tree', 'wasm-tree']:
    previous = next(c for c in discovery['workspaceChecks'] if c['name'] == name)
    assert Path(previous['log']).read_bytes() == Path(checks[name]['log']).read_bytes()
tree = Path(checks['wasm-tree']['log']).read_text()
assert all(n not in tree for n in ['rusqlite', 'mo-standard-host', 'mo-native-render', 'mo-native-worker'])

ref_path = root/'reference-1/reference.json'; reference = json.loads(ref_path.read_text())
assert reference['automaticExports'] == 4 and reference['modes'] == ['job', 'sync', 'auto']
assert reference['oldAssets'] == 12 and reference['allOldRowsUnchanged']
assert reference['priorQueueExecuted'] and reference['completedJobsUnchanged'] and reference['schemaPairs'] == 10
for name in ['cli', 'worker', 'export', 'exportReplay', 'currentDatabase', 'oldDatabase', 'migrated', 'schemas', 'schemaProgram', 'schemaLog']:
    r = reference[name]; assert entry(r['path']) == r
for c in reference['calls']:
    for name in ['stdin', 'stdout']: assert entry(c[name]['path']) == c[name]
for s in reference['sessions']:
    assert s['exitCode'] == 0 and s['stderr']['byteLength'] == 0
    assert entry(s['stderr']['path']) == s['stderr']
for a in reference['oldAssetReads']:
    assert entry(a['bytes']['path']) == a['bytes']
    assert a['bytes']['sha256'] == a['asset']['sha256']
export = json.loads(Path(reference['export']['path']).read_text())
assert len(export['calls']) == 31 and len(export['migrations']) == 2
assert export['publicAssets'] == 12 and export['pages'] == 2
assert entry(export['database']['path']) == export['database']
for m in export['migrations']:
    assert m['allRowsUnchanged'] and entry(m['before']['path']) == m['before'] and entry(m['after']['path']) == m['after']
for c in export['calls']:
    assert c['exitCode'] == 0
    for name in ['stdin', 'stdout', 'stderr']:
        if name in c: assert entry(c[name]['path']) == c[name]
assert entry(export['reference']['path']) == export['reference']
parity_path = root/'parity-reference.json'; parity = json.loads(parity_path.read_text())
assert parity['calls'] == dict(authored=17, source=33, editor=22)
assert parity['pagePairs'] == 2 and parity['allPreviousCasesUnchanged']
for name in ['previous', 'frozenProgram', 'replayProgram', 'current']:
    r = parity[name]; assert entry(r['path']) == r

# Only the declared runtime availability reason extends these schema documents.
old_docs = {d['id']: d['schema'] for d in json.loads(Path('.codex-work/operation-discovery/reference-1/schemas.json').read_text())}
new_docs = json.loads(Path(reference['schemas']['path']).read_text())
for d in new_docs:
    value = json.loads(json.dumps(d['schema']))
    if d['id'] in ['host-capabilities', 'host-response']:
        value['$defs']['UnavailableReason']['enum'].remove('executionUnavailable')
    assert value == old_docs[d['id']], d['id']

links = 0
for name in sources:
    p = Path(name)
    if p.suffix in ['.rs', '.py', '.ts', '.mjs', '.cpp', '.h']:
        assert len(p.read_text().splitlines()) <= 2000, name
    if p.suffix != '.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'): continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (p.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1
current_wasm = root/'wasm-node/mo_wasm_bg.wasm'
previous_wasm = Path('.codex-work/operation-discovery/wasm-node/mo_wasm_bg.wasm')
assert entry(previous_wasm) == next(r for r in discovery['artifacts'] if r['path'] == str(previous_wasm))
def size(path):
    return dict(module=entry(path), gzip9Bytes=len(gzip.compress(path.read_bytes(), compresslevel=9, mtime=0)))
artifacts = {str(p) for p in root.rglob('*') if p.is_file()}
artifacts |= {str(previous_wasm), reference['schemaProgram']['path'], reference['oldDatabase']['path'],
              reference['cli']['path'], reference['worker']['path'], checks['export-integration']['worker']['path']}
report = dict(format='musteroffice.native-scheduler-verification/1', previousEvidence=entry(parent_path),
    scope='Native durable execution over the standard owner; not complete presentation or product acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    workspaceChecks=list(checks.values()), workspaceAttemptHistory=history,
    rustTestNames=tests('tests'), finalHostTestNames=tests('final-host-tests'),
    explicitIntegrationTests=dict(native=tests('native-integration'), export=tests('export-integration')),
    reports=dict(reference=entry(ref_path), parity=entry(parity_path)),
    checks=dict(workspaceTests=837, finalHostTests=61, explicitNativeIntegrations=11, contracts=109,
        nativeWasmSchemas=10, autoScheduledExports=4, migrationCopies=3, priorPublicAssets=12,
        fileEditPairs=72, pixelPagePairs=2, nativeDependencyTreeUnchanged=True, pureWasmDependencyTreeUnchanged=True,
        strictClippy=True, localLinks=links, fullMcp=False, fullPresentation=False, musterworkReplacement=False),
    wasmModuleSize=dict(previous=size(previous_wasm), current=size(current_wasm),
        scope='Same locked release toolchain/configuration. Rust module only; excludes JS, drawing/shaping, fonts/media and desktop packaging.'),
    artifacts=[entry(p) for p in sorted(artifacts)],
    changesExplained=[
        'v5 adds derived queued/expiring indexes over existing receipts. Selection and claim share the existing immediate transaction and fence.',
        'Runtime connects all workers before admission; control connections remain separate. Sync/auto share the same execution budget. In-flight iterations finish at shutdown and queued work remains durable.',
        'Mutations, exports and private I/O now share execution cancellation/lease validation. Renewal is based on the persisted lease update time, including delayed preparation.',
        'Full workspace tests ran before two final host refinements: replay accepted requests during pool failure, and initial renewal based on persisted time. Final 61 host tests plus strict host Clippy cover both; real integrations/release execution use the refined implementation.',
        'The old v1 migration fixture used invalid placeholder JSON. It now stores a genuine submitted receipt and still checks byte-preserved migration; invalid v4 JSON is separately verified to roll back migration.',
        'An unused import was removed after the workspace build reported it; subsequent strict Clippy has no warning suppression.',
        'Only two generated schemas/TS types change for executionUnavailable. No external dependencies are added; the lock adds only an internal dev edge for actual renderer coordination.',
    ],
    limitations=[
        'Thread limits are per runtime, not per database or machine. SQLite busy waits, source snapshot decoding, component memory and hard termination require their own budgets.',
        'Sync timeout limits condition waiting, not total database-call wall time. Mutation cancellation remains cooperative at existing computation checkpoints.',
        'CLI supports native persistent scheduling but is not a production MCP server, HTTP/OAuth transport, browser owner, SDK, Skill/Plugin package or Musterwork adapter.',
        'Owned synthetic-font/limited-content fixtures cannot establish Office/WPS visual, editing or playback fidelity for general presentations.',
        'All advanced-content, native/WASM, third-party, performance, complete package and Musterwork replacement gates remain open.',
    ])
if '--dry-run' in sys.argv:
    print(json.dumps(dict(sourceFiles=len(sources), artifacts=len(artifacts), checks=report['checks'])))
    raise SystemExit(0)
elif '--check' in sys.argv:
    assert json.loads(output.read_text()) == report
else:
    with output.open('x') as out: json.dump(report, out, indent=2); out.write('\n')
print(json.dumps(dict(evidence=entry(output), sourceFiles=len(sources), artifacts=len(artifacts), checks=report['checks'])))
