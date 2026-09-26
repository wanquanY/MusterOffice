"""Seal reusable manifest layout evidence separately from the old runtime release."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path('.codex-work/manifest-layout')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-native-fonts-library-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-manifest-layout-library-verification.json')


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


assert entry(PREVIOUS)['sha256'] == '1110756f71e20f52cbaf1e0c9a636e7f52d5c03a589e2ebe2331d4c2bef13cde'
old = json.loads(PREVIOUS.read_text())
changed = [p['path'] for p in old['sourceFiles'] if entry(p['path']) != p]
allowed = {
    'Cargo.lock', 'README.md', 'docs/README.md', 'docs/implementation/development.md',
    'docs/implementation/native-fonts.md', 'docs/implementation/progress.md',
    'crates/mo-harfbuzz-sys/Cargo.toml', 'crates/mo-text/src/cascade/context.rs',
    'crates/mo-text/src/cascade/mod.rs', 'crates/mo-text/src/cascade/types.rs',
    'crates/mo-text/src/fallback/mod.rs', 'crates/mo-text/src/flow/mod.rs',
    'crates/mo-text/src/lib.rs', 'crates/mo-text/src/lines.rs',
    'crates/mo-text/src/manifest/mod.rs', 'crates/mo-text/src/manifest/tests.rs',
    'crates/mo-text/src/paragraph.rs', 'crates/mo-text/src/scene/mod.rs',
    'crates/mo-text/src/scene/tests.rs',
}
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = [
    'crates/mo-text/src/resources.rs', 'crates/mo-text/src/resources_tests.rs',
    'crates/mo-text/src/manifest/binding.rs', 'crates/mo-text/src/manifest/prepared.rs',
    'crates/mo-text/src/manifest/prepared_tests.rs', 'crates/mo-text/src/scene/test_support.rs',
    'crates/mo-harfbuzz-sys/examples/manifest_paths.rs',
    'tools/verification/manifest-layout-reference.py', 'tools/verification/manifest-layout-baseline.py',
    'tools/verification/manifest-layout-regressions.py', 'tools/verification/manifest-layout-evidence.py',
    'docs/implementation/manifest-layout.md',
]
sources = {p['path'] for p in old['sourceFiles']} | set(added)
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
for p in old['previousReleaseArtifactsVerifiedUnchanged'].values():
    assert entry(p['path']) == p
for p in old['standardInputs']:
    assert entry(p['path']) == p
for p in old['sourceFiles']:
    if p['path'].startswith(('contracts/', 'packages/')):
        assert entry(p['path']) == p
# Prove exact lock delta, including the absence of any external version changes.
lock = Path('Cargo.lock').read_text()
a = lock.index('name = "mo-harfbuzz-sys"')
b = lock.index('[[package]]', a)
section = lock[a:b]
assert section.count(' "mo-common",\n') == section.count(' "mo-geometry",\n') == 1
restored = lock[:a] + section.replace(' "mo-common",\n', '').replace(' "mo-geometry",\n', '') + lock[b:]
assert hashlib.sha256(restored.encode()).hexdigest() == next(p['sha256'] for p in old['sourceFiles'] if p['path'] == 'Cargo.lock')
logs = ['text-tests.log', 'workspace-tests.log', 'clippy.log', 'wasm-check.log',
        'fmt.log', 'schema-check.log', 'native-paths-build.log', 'native-worker-build.log',
        'native-shaping-build.log', 'reference.log', 'regressions.log', 'baseline-reproduction.log']
for name in logs:
    text = (ROOT / name).read_text()
    assert not any(s in text for s in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
assert len(tests) == 434
new_tests = [s for s in tests if s.startswith(('resources::tests::', 'manifest::tests::prepared_tests::'))]
assert len(new_tests) == 6
assert entry(ROOT / 'native-shaping.json')['sha256'] == old['independentManifestReference']['result']['sha256']
baseline = ROOT / 'previous/frozen-runtime-paths.json'
assert json.loads(baseline.read_text()) == json.loads((ROOT / 'baseline-reproduction.json').read_text())
reference = json.loads((ROOT / 'reference.json').read_text())
assert reference['counts'] == dict(operations=20, errors=2, evaluated=18, scenes=14,
                                  paths=22, glyphs=42, originCoordinates=84, bounds=36)
for p in reference['inputs']:
    assert entry(p['path']) == p
regressions = json.loads((ROOT / 'regressions.json').read_text())
assert regressions['counts'] == dict(itemizationAndParagraph=34, mixedFont=17, lineShaping=34,
                                    lineGeometry=35, paragraphLayout=37, paragraphPaths=50)
assert regressions['total'] == len(regressions['cases']) == 207
for p in regressions['previousReports'] + [regressions['worker'], regressions['previousEvidence']]:
    assert entry(p['path']) == p
for c in regressions['cases']:
    for key in ['request', 'bundle', 'priorResponse', 'response']:
        assert entry(c[key]['path']) == c[key]

report = dict(
    format='musteroffice.manifest-layout-library-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Shared verified font resources and prepared manifest shaping/layout/paths. Current Native execution and old Native response regressions; no new runtime operation or WASM execution.',
    checks=dict(rustTests=434, newRustTests=6, strictClippy=True, rustfmt=True,
                wasmTargetCompilation=['mo-text'], unchangedSchemas=68, schemaGenerationCheck=True,
                unchangedTypescript=True, realNativeManifestPathRequests=20, exactNativeRegressionRequests=207,
                priorNativeManifestShapingBytesUnchanged=True, newRuntimeParityBatches=0,
                previousFrozenRuntimeParityBatches=10064),
    changedPreviousSources=changed, addedSources=added, sourceFiles=[entry(p) for p in sorted(sources)],
    validationLogs=[entry(ROOT / p) for p in logs], rustTestNames=tests, newTestNames=new_tests,
    reference=entry(ROOT / 'reference.json'), independentReference=reference,
    nativeRegressions=entry(ROOT / 'regressions.json'), regressionEvidence=regressions,
    baselineReproduction=entry(ROOT / 'baseline-reproduction.json'),
    currentNativeShaping=entry(ROOT / 'native-shaping.json'),
    developmentArtifacts=[entry(p) for p in ['target/debug/examples/manifest_paths',
                                            'target/debug/examples/manifest_shape', 'target/debug/mo-text-worker']],
    nativeComponentLibrary=entry('.codex-work/harfbuzz/release/libmo_harfbuzz.a'),
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],
    dependencyChanges='Only mo-common and mo-geometry existing local development dependencies added for the example; exact lock reconstruction verified.',
    limitations=[
        'No source PPTX text-to-page compiler, automatic native font slot/language classification, page preflight or full FontManifest delivery.',
        '20 owned Native requests verify shared resource continuity, not office typography. Fraction checks cover emitted origins and linear path bounds.',
        '207 ordinary existing Native requests replayed. Separate historical component fault-injection scenarios were not rerun; new failure/cancellation tests use controlled backends.',
        'No new WASM runtime execution; target compilation and historical 10064 batches do not certify current prepared-manifest runtime parity.',
        'Resource sharing removes repeat verification across paragraphs. No new performance multiplier, installer size, page pixels, Office/WPS round trip or Musterwork acceptance is claimed.',
    ],
)
raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as file:
        file.write(raw)
print(json.dumps(dict(checks=report['checks'], sourceFiles=len(sources),
                     evidence=entry(OUTPUT) if '--seal' in sys.argv else None), indent=2))
