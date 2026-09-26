"""Bind delivery computation to actual outputs and scoped verification evidence."""
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/delivery-pipeline')
parent_path = Path('docs/reviews/evidence/2026-09-26-job-results-verification.json')
output = Path('docs/reviews/evidence/2026-09-26-delivery-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(parent_path)['sha256'] == '08eb455442bed35c3d635d2d4cb4f20ebadd8f0cd160b57f4a6b900ff381a9ad'
parent = json.loads(parent_path.read_text())
prior = {r['path']: r for r in parent['sourceFiles']}
allowed = set('''Cargo.lock
Cargo.toml
README.md
crates/mo-harfbuzz-sys/tests/source_text_page.rs
crates/mo-image/Cargo.toml
crates/mo-image/src/lib.rs
crates/mo-kernel-api/tests/pptx_text_page.rs
crates/mo-native-io/tests/spool.rs
crates/mo-pptx/src/definitions.rs
crates/mo-pptx/src/text.rs
crates/mo-pptx/src/write.rs
crates/mo-pptx/tests/stream_export.rs
docs/README.md
docs/implementation/dependencies.md
docs/implementation/development.md
docs/implementation/progress.md
tools/mo-cli/Cargo.toml
tools/mo-cli/src/worker.rs
tools/mo-cli/tests/stream_export.rs
tools/mo-contract-codegen/Cargo.toml
tools/mo-contract-codegen/src/main.rs
tools/test-support/source_text_page.rs'''.splitlines())
changed = {p for p, r in prior.items() if entry(p) != r}
assert changed == allowed, (changed-allowed, allowed-changed)
added = set('''crates/mo-image/src/png.rs
crates/mo-image/tests/png.rs
crates/mo-pptx/tests/export_text.rs
docs/implementation/presentation-delivery.md
tools/verification/delivery-workspace.py
tools/verification/delivery-native.py
tools/verification/delivery-reference.py
tools/verification/delivery-parity.mjs
tools/verification/delivery-style-delta.py
tools/verification/delivery-evidence.py'''.splitlines())
for directory in ['crates/mo-presentation-delivery', 'crates/mo-native-render',
                  'crates/mo-native-worker', 'fixtures/presentations/delivery']:
    added |= {str(p) for p in Path(directory).rglob('*') if p.is_file()}
for stem in ['presentation-delivery-settings', 'presentation-delivery-bundle', 'presentation-renderer-identity']:
    added.add(f'contracts/generated/{stem}.schema.json')
    added.add(f'packages/contracts/src/generated/{stem}.ts')
assert not added & set(prior)
sources = set(prior) | added
for path in sources:
    p = Path(path)
    if p.suffix in ['.rs', '.py', '.mjs', '.ts', '.cpp', '.h']:
        assert len(p.read_text().splitlines()) <= 2000, path
for path, record in prior.items():
    if path.startswith(('contracts/generated/', 'packages/contracts/src/generated/')):
        assert entry(path) == record

history = json.loads((root/'workspace.json').read_text())
checks = {c['name']: c for c in history}
assert set(checks) == {'fmt', 'tests', 'clippy', 'native-build', 'example-build', 'schema-check',
                      'types-check', 'pure-delivery-wasm', 'rust-wasm', 'bindgen', 'native-tree', 'wasm-tree'}
assert all(c['exitCode'] == 0 for c in checks.values())
for check in checks.values():
    assert not any(bad in Path(check['log']).read_text() for bad in ['error:', 'FAILED', 'Traceback', 'AssertionError'])
tests = re.findall(r'^test (.+) \.\.\. ok$', Path(checks['tests']['log']).read_text(), re.M)
assert len(tests) == 819 and set(parent['rustTestNames']) <= set(tests)
assert len(set(tests)-set(parent['rustTestNames'])) == 9
assert len(re.findall(r'^check contracts/generated/', Path(checks['schema-check']['log']).read_text(), re.M)) == 107
assert len(re.findall(r'^check .+\.ts$', Path(checks['types-check']['log']).read_text(), re.M)) == 107
tree = Path(checks['wasm-tree']['log']).read_text()
assert all(name not in tree for name in ['mo-native-render', 'mo-native-worker', 'rusqlite', 'mo-standard-host'])

native = json.loads((root/'native.json').read_text())
for record in [native['worker'], native['example']]:
    assert entry(record['path']) == record
assert len(native['calls']) == 3 and all(c['exitCode'] == 0 for c in native['calls'])
for call in native['calls']:
    assert entry(call['log']['path']) == call['log']
integration = re.findall(r'^test (.+) \.\.\. ok$', (root/'native-integration-final.log').read_text(), re.M)
assert set(integration) == {'actual_two_page_delivery_and_hidden_page_cover_every_bound_artifact',
                            'failed_or_mismatched_components_never_return_a_complete_candidate'}
reference = json.loads((root/'final/reference.json').read_text())
assert len(reference['artifacts']) == 12 and len(reference['pages']) == 2
assert len(reference['schemasChecked']) == 10 and reference['nativeObjects'] == 15 and reference['nativeTextRuns'] == 8
for record in [*reference['artifacts'], reference['fixture'], reference['bundle'], reference['independentProgram']]:
    assert entry(record['path']) == record
parity = json.loads((root/'parity.json').read_text())
assert (parity['authoredCases'], parity['sourceCases'], parity['editorCases']) == (17, 33, 22)
assert (parity['previousAuthoredRegenerated'], parity['previousEditedByteIdentical'],
        parity['previousSourceResponsesIdentical'], parity['previousEditorResponsesIdentical']) == (7, 12, 12, 22)
assert len(parity['pages']) == 2
for record in parity['artifacts'] + list(parity['reports'].values()):
    assert entry(record['path']) == record
for page in parity['pages']:
    for record in page.values():
        assert entry(record['path']) == record
delta = json.loads((root/'style-delta.json').read_text())
assert len(delta['cases']) == 7
for case in delta['cases']:
    assert case['changedParts'] == ['ppt/presentation.xml', 'ppt/slideMasters/slideMaster2.xml', 'ppt/slides/slide1.xml', 'ppt/slides/slide2.xml']
    assert len(case['xsdParts']) == 10
    for key in ['before', 'after']:
        assert entry(case[key]['path']) == case[key]
for key in ['rustWasm', 'rustWasmGlue']:
    record = parent['unchangedKernelArtifacts'][key]
    assert entry(record['path']) == record

artifacts = {str(p) for p in root.rglob('*') if p.is_file() and p.suffix not in ['.md', '.pyc']}
for record in parity['reports'].values():
    suite = json.loads(Path(record['path']).read_text())
    if suite.get('artifactDirectory'):
        artifacts |= {str(p) for p in Path(suite['artifactDirectory']).rglob('*') if p.is_file()}
artifacts |= {r['path'] for r in parity['artifacts']}
artifacts |= {native['example']['path'], str(parent_path)}
# Artifact collection intentionally excludes this evidence's own output.
assert str(output) not in artifacts
links = 0
for name in sources:
    p = Path(name)
    if p.suffix != '.md':
        continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (p.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

report = dict(
    format='musteroffice.delivery-verification/1', previousEvidence=entry(parent_path),
    scope='Actual private model/PPTX/resource/font/PNG/quality delivery, bounded native transport and corrected matching-level defaults. Not public export or full presentation acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added), removedSources=[],
    workspaceChecks=list(checks.values()), workspaceAttemptHistory=history, rustTestNames=tests, explicitNativeIntegrationTests=integration,
    checks=dict(rustTests=len(tests), newRustTests=9, explicitNativeIntegrationTests=2, schemas=107, previousSchemasUnchanged=104,
                strictClippy=True, typescript=True, pureDeliveryWasmCompile=True, completeDeliveryWasmExecution=False,
                actualDeliveryFiles=12, actualPages=2, actualPageNativeWasmPairs=2, authoredCalls=17, sourceCalls=33, editorCalls=22,
                changedWriterFilesIndependentlyChecked=7, xsdParts=10, nativeObjects=15, nativeTextRuns=8,
                publicExport=False, fullFeatureAcceptance=False, officeWpsNewAcceptance=False, localLinks=links),
    reports={name:entry(root/path) for name, path in dict(native='native.json', reference='final/reference.json', parity='parity.json', styleDelta='style-delta.json').items()},
    artifacts=[entry(p) for p in sorted(artifacts)],
    changesExplained=[
        'Writer defaults now use lvl1pPr for the zero-level authored paragraphs. Actual source rendering first exposed missing inherited font declarations.',
        'Prior expected PPTX bytes and missing-paint fixtures were corrected with the writer. Assertions still require exact diagnostic locations and no component calls on invalid input.',
        'Early candidate bound featureRegistrySha256 to delivery configuration. Final candidate binds an explicitly partial delivery-composition registry; renderer identity is separately bound in its context.',
        'The independent verifier initially shadowed its file-digest helper with a registry loop variable. Fixed only the verifier, retained failed logs/candidate, then repeated the real native pipeline.',
        'Workspace test run began before the final registry metadata change. Subsequent native build, explicit real-worker integration and independent final-file checks exercise that final implementation.',
    ],
    limitations=[
        'Full v0.4, E0-E3 and Musterwork replacement remain incomplete; the goal stays active.',
        'These are private calculation outputs, not published assets, successful export jobs or a product handoff. Standard/EmbeddedHost must atomically bind accepted request, executor, fence, cancellation and assets.',
        'Only OPC/ZIP/XML graph/digest structure is claimed passed at runtime; layout, native editing, playback and target-application claims are not_proven. XSD is an independent stage check.',
        'The two-page owned fixture uses eight A A runs in a synthetic triangle-glyph font; it is not commercial typography or full-slide fidelity evidence.',
        'The delivery-composition registry is explicitly partial and not the complete locked native feature catalogue required by the design.',
        'Full candidate execution is Native-only in this evidence. The pure core compiles for WASM, and actual individual page rendering executes on both targets.',
        'Only a trusted direct worker process is killed/reaped. Producer callback bounds are a host responsibility; executable prehashing is not protection against same-user concurrent replacement.',
        'Workers may hold whole packages/fonts/pixels. Chunked transport/PNG output does not imply constant total RSS or system-wide resource limits.',
        'No new performance, FPS, packaged application size, cross-platform release or Office/WPS editing acceptance is claimed.',
        'Complete effects, animation, transitions, media, SmartArt, equations, Agent interfaces and Musterwork migration remain required.',
    ],
)
encoded = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == encoded, 'sealed evidence changed'
else:
    with output.open('x') as stream:
        stream.write(encoded)
print(json.dumps(dict(evidence=entry(output), sourceFiles=len(sources), checks=report['checks']), ensure_ascii=False))
