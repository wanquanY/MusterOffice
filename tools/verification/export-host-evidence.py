"""Seal source, binary, public-byte and lifecycle evidence for the export owner."""
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/export-host')
parent_path = Path('docs/reviews/evidence/2026-09-26-delivery-verification.json')
output = Path('docs/reviews/evidence/2026-09-26-export-host-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(parent_path)['sha256'] == '0526033a5f818a1bb0a91a1915b204fa9f5a94a91dbb386ac36158de5a94b6cb'
parent = json.loads(parent_path.read_text())
prior = {r['path']:r for r in parent['sourceFiles']}
allowed = set('''Cargo.lock
README.md
crates/mo-operation-service/Cargo.toml
crates/mo-operation-service/src/compute.rs
crates/mo-operation-service/src/contract.rs
crates/mo-operation-service/src/lib.rs
crates/mo-presentation-delivery/src/build.rs
crates/mo-presentation-delivery/src/contract.rs
crates/mo-standard-host/Cargo.toml
crates/mo-standard-host/src/assets/reader.rs
crates/mo-standard-host/src/assets/storage.rs
crates/mo-standard-host/src/db.rs
crates/mo-standard-host/src/jobs.rs
crates/mo-standard-host/src/lib.rs
crates/mo-standard-host/src/results.rs
crates/mo-standard-host/src/results/storage.rs
crates/mo-standard-host/tests/resource_recovery.rs
docs/README.md
docs/design/agent-interfaces.md
docs/implementation/dependencies.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-host/Cargo.toml
tools/mo-host/src/main.rs'''.splitlines())
changed_contracts = ['host-request', 'host-response', 'operation-job', 'operation-request', 'upload-info']
for stem in changed_contracts:
    allowed.add(f'contracts/generated/{stem}.schema.json')
    allowed.add(f'packages/contracts/src/generated/{stem}.ts')
changed = {p for p,r in prior.items() if entry(p) != r}
assert changed == allowed, (changed-allowed, allowed-changed)
added = set('''crates/mo-operation-service/src/export.rs
crates/mo-operation-service/tests/export_contract.rs
crates/mo-standard-host/src/exports.rs
crates/mo-standard-host/src/results/published.rs
crates/mo-standard-host/tests/exports.rs
docs/architecture/agent-integration.md
docs/implementation/export-host.md
tools/verification/export-host-reference.py
tools/verification/export-host-workspace.py
tools/verification/export-host-parity.py
tools/verification/export-host-evidence.py'''.splitlines())
assert not added & set(prior)
sources = set(prior) | added
for path in sources:
    p = Path(path)
    if p.suffix in ['.rs','.py','.mjs','.ts','.cpp','.h']:
        assert len(p.read_text().splitlines()) <= 2000, path

history = json.loads((root/'workspace.json').read_text())
checks = {c['name']:c for c in history}
assert set(checks) == {'fmt','tests','clippy','native-build','schema-check','types-check',
    'pure-operation-wasm','rust-wasm','bindgen','native-integration','export-integration','native-tree','wasm-tree'}
assert all(c['exitCode'] == 0 for c in checks.values())
for c in checks.values():
    assert not any(bad in Path(c['log']).read_text() for bad in ['error:','FAILED','Traceback','AssertionError'])
tests = re.findall(r'^test (.+) \.\.\. ok$',Path(checks['tests']['log']).read_text(),re.M)
assert len(tests) == 821 and set(parent['rustTestNames']) <= set(tests)
assert len(set(tests)-set(parent['rustTestNames'])) == 2
explicit = {}
for kind,count in [('native',2),('export',7)]:
    c = checks[kind+'-integration']
    assert entry(c['worker']['path'])['sha256'] == c['worker']['sha256']
    explicit[kind] = re.findall(r'^test (.+) \.\.\. ok$',Path(c['log']).read_text(),re.M)
    assert len(explicit[kind]) == count
assert len(re.findall(r'^check contracts/generated/',Path(checks['schema-check']['log']).read_text(),re.M)) == 107
assert len(re.findall(r'^check .+\.ts$',Path(checks['types-check']['log']).read_text(),re.M)) == 107
tree = Path(checks['wasm-tree']['log']).read_text()
assert all(name not in tree for name in ['rusqlite','mo-standard-host','mo-native-render','mo-native-worker'])

reference = json.loads((root/'reference-4/reference.json').read_text())
assert len(reference['calls']) == 31 and len(reference['migrations']) == 2
assert (reference['publicAssets'],reference['pages']) == (12,2)
assert (reference['validatedRequests'],reference['validatedResponses']) == (12,16)
for record in [reference['cli'],reference['worker'],reference['reference'],reference['database'],*reference['schemas']]:
    assert entry(record['path']) == record
for c in reference['calls']:
    assert c['exitCode'] == 0
    for key in ['stdin','stdout','stderr']:
        if key in c: assert entry(c[key]['path']) == c[key]
for m in reference['migrations']:
    assert m['allRowsUnchanged'] and m['existingTables'] == 8
    assert entry(m['before']['path']) == m['before']
    assert entry(m['after']['path']) == m['after']
independent = json.loads(Path(reference['reference']['path']).read_text())
assert len(independent['artifacts']) == 12 and len(independent['pages']) == 2
assert len(independent['schemasChecked']) == 10
assert (independent['nativeObjects'],independent['nativeTextRuns']) == (15,8)
for r in [*independent['artifacts'],independent['fixture'],independent['bundle'],independent['independentProgram']]:
    assert entry(r['path']) == r

parity_ref = json.loads((root/'parity-reference.json').read_text())
assert parity_ref['calls'] == {'authored':17,'source':33,'editor':22}
assert parity_ref['pagePairs'] == 2 and parity_ref['allPreviousCasesUnchanged']
for key in ['previous','frozenProgram','replayProgram','current']:
    r = parity_ref[key]; assert entry(r['path']) == r
parity = json.loads(Path(parity_ref['current']['path']).read_text())
for r in parity['artifacts'] + list(parity['reports'].values()):
    assert entry(r['path']) == r
for p in parity['pages']:
    for r in p.values(): assert entry(r['path']) == r
# The parity replay read reference-3 before the verifier's DB-close correction.
# Bind that actual page input to the final independently checked public bytes.
old_files = json.loads((root/'reference-3/candidate/files.json').read_text())
final_files = json.loads((root/'reference-4/candidate/files.json').read_text())
assert old_files == final_files
for f in final_files:
    a = root/'reference-3/candidate'/f['file']
    b = root/'reference-4/candidate'/f['file']
    assert entry(a)['sha256'] == entry(b)['sha256'] == f['asset']['sha256']
for name in ['mo_wasm.js','mo_wasm_bg.wasm']:
    frozen = f'.codex-work/delivery-pipeline/wasm-node/{name}'
    assert entry(frozen) == next(r for r in parent['artifacts'] if r['path'] == frozen)

links = 0
for name in sources:
    p = Path(name)
    if p.suffix != '.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'): continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            dest = (p.parent/target).resolve()
            assert dest.exists() or dest == output.resolve(), (name,target)
            links += 1
artifacts = {str(p) for p in root.rglob('*') if p.is_file() and p.suffix not in ['.md','.pyc']}
for r in parity['reports'].values():
    suite = json.loads(Path(r['path']).read_text())
    if suite.get('artifactDirectory'):
        artifacts |= {str(p) for p in Path(suite['artifactDirectory']).rglob('*') if p.is_file()}
artifacts |= {r['path'] for r in parity['artifacts']}
artifacts |= {reference['cli']['path'],reference['worker']['path'],str(parent_path)}
assert str(output) not in artifacts
report = dict(format='musteroffice.export-host-verification/1',previousEvidence=entry(parent_path),
    scope='Actual standard-host export jobs, request-bound private computation, atomic durable public assets, historical revisions and lifecycle; not complete presentation or Musterwork acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),
    addedSources=sorted(added),removedSources=[],
    sourceInventoryNote='agent-integration.md existed before this stage and is newly bound here after its implementation-status update.',
    workspaceChecks=list(checks.values()),workspaceAttemptHistory=history,rustTestNames=tests,explicitIntegrationTests=explicit,
    checks=dict(rustTests=821,newRustTests=2,explicitNativeIntegrationTests=2,explicitExportIntegrationTests=7,
        schemas=107,changedSchemas=5,unchangedSchemas=102,strictClippy=True,typescript=True,
        pureOperationWasmCompile=True,completeExportWasmExecution=False,releaseCalls=31,
        realOldDatabaseMigrations=2,wireRequestsValidated=12,wireResponsesValidated=16,
        publishedAssets=12,pages=2,xsdParts=10,nativeObjects=15,nativeTextRuns=8,
        authoredCalls=17,sourceCalls=33,editorCalls=22,pageNativeWasmPairs=2,
        previousCasesUnchanged=True,publicExportDraft=True,fullFeatureAcceptance=False,
        officeWpsNewAcceptance=False,musterworkIntegration=False,localLinks=links),
    reports=dict(reference=entry(root/'reference-4/reference.json'),parity=entry(root/'parity-reference.json')),
    artifacts=[entry(p) for p in sorted(artifacts)],
    changesExplained=[
        'Export now uses the same request/job owner; published assets reference immutable existing SQL chunks and commit with receipt in one transaction.',
        'Export pins a historical immutable revision and does not conflict with subsequent head edits. Repeat content reuses public backing and releases duplicate private reservations.',
        'PreviewRenderer Send is required only for native StandardHost configuration, preserving WASM and core portability.',
        'Transparent I/O wrappers initially hid typed failures; explicit wrapper matching preserves original cancellation, budget and resource errors.',
        'Mutation receipts retain their prior JSON shape; the export alternative is untagged and rejects ambiguous mixed receipts. Failure detail is optional and omitted when absent.',
        'The first CLI migration verifier incorrectly assumed committed heads in resource-owner fixtures. It now checks their actual assets and unchanged original rows; failed directory retained.',
        'Evidence first detected post-report SQLite checkpoint changes: the verifier used Connection context management without closing. It now explicitly closes all read connections before recording database digests; final reference-4 public bytes equal the reference-3 files used in actual parity replay.',
        'Additional explicit tests advance the host clock across the initial lease and cancel/expire after private bytes exist. They exercise real computation and are not timing benchmarks.',
    ],
    limitations=[
        'Full v0.4, E0-E3, Agent adapters and Musterwork replacement remain incomplete; the goal stays active.',
        'The operation profile and generated schemas are draft implementations, not the complete design envelope, MCP server, Skill, Plugin, SDK or public release.',
        'Queued CLI jobs require an explicit operator run command. There is no autonomous scheduler or complete host admission/supervision implementation.',
        'A storage failure rolls back publication but may leave Running/private state until cancellation or expiry. Automatic job replay and crash/power-loss recovery are not claimed.',
        'Complete WASM export owner execution is unverified. The pure operation core compiles for WASM and existing file/page calls execute on both targets.',
        'The two-page input uses a synthetic owned font and eight A A runs. It is not commercial text or slide-fidelity proof.',
        'Only OPC/ZIP/XML graph/digest structure passes in runtime claims; layout, native-editing roundtrip, playback and target-application quality remain not_proven. XSD is independently checked.',
        'Published bytes remain quota-accounted without expiry. Full deletion, retention/GC and multi-client resource lifecycle still require implementation.',
        'Worker package/font/pixel allocations remain; chunked transport does not prove constant RSS or a hard system-wide resource budget.',
        'No new Office/WPS interaction, performance, FPS, final package size or cross-platform release acceptance is claimed.',
        'Complete effects, animations/transitions, media, SmartArt, equations and all original requirements remain in scope.',
    ])
encoded = json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == encoded, 'sealed evidence changed'
else:
    with output.open('x') as f: f.write(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),checks=report['checks']),ensure_ascii=False))
