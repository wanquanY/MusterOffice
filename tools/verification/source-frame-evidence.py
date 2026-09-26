"""Seal horizontal source-frame computation independently of old runtime releases."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path('.codex-work/source-frame')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-source-glyph-library-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-source-frame-library-verification.json')


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


assert entry(PREVIOUS)['sha256'] == '6fbd4979fe21b0d5569aaac0d25616479001aa444fa140e6246004357d587d82'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'README.md', 'docs/README.md', 'docs/implementation/development.md',
    'docs/implementation/progress.md', 'docs/implementation/source-glyphs.md',
    'crates/mo-pptx/src/source/text/cascade.rs', 'crates/mo-presentation-compile/src/lib.rs',
    'crates/mo-presentation-compile/src/source_text.rs',
    'crates/mo-presentation-compile/tests/source_glyphs.rs',
    'crates/mo-text/src/flow/mod.rs', 'crates/mo-text/src/flow/types.rs',
    'crates/mo-text/src/flow/tests.rs', 'crates/mo-text/src/geometry/mod.rs',
    'crates/mo-text/src/geometry/types.rs', 'crates/mo-text/src/manifest/prepared.rs',
    'crates/mo-text/src/manifest/prepared_tests.rs', 'crates/mo-text/src/scene/build.rs',
    'crates/mo-text/src/scene/mod.rs', 'crates/mo-text/src/scene/types.rs',
    'tools/verification/manifest-layout-regressions.py',
}
changed = [p['path'] for p in old['sourceFiles'] if entry(p['path']) != p]
assert set(changed) == allowed, (set(changed)-allowed, allowed-set(changed))
added = ['crates/mo-presentation-compile/src/source_frame.rs',
         *['crates/mo-presentation-compile/src/source_frame/'+n+'.rs'
           for n in ['types', 'number', 'properties', 'backend']],
         'crates/mo-harfbuzz-sys/tests/source_frame.rs', 'tools/test-support/source_glyphs.rs',
         'tools/verification/source-frame-reference.py', 'tools/verification/source-frame-evidence.py',
         'docs/implementation/source-frame.md']
sources = {p['path'] for p in old['sourceFiles']} | set(added)
assert len(sources) == len(old['sourceFiles']) + len(added)
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
# Cargo.lock and every manifest are among previous source files. The strict
# delta above excludes them: no new local edge or external version is permitted.
for record in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(),
               *old['standardInputs'], old['nativeComponentLibrary']]:
    assert entry(record['path']) == record, record['path']
logs = ['native-tests.log', 'workspace-tests.log', 'clippy.log', 'wasm-check.log',
        'fmt.log', 'schema-check.log', 'native-worker-build.log', 'reference.log',
        'reference-native.log', 'regressions.log']
for name in logs:
    log = (ROOT / name).read_text()
    assert not any(s in log for s in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
new = [name for name in tests if name not in old['rustTestNames']]
assert len(tests) == 454 and len(new) == 9
assert len(re.findall(r'^check contracts/generated/', (ROOT / 'schema-check.log').read_text(), re.M)) == 68
reference = json.loads((ROOT / 'reference.json').read_text())
assert reference['counts'] == dict(packages=15, paragraphs=22, lines=23, glyphs=28,
                                  originCoordinates=56, pathBounds=56,
                                  fractionalRegions=1, fractionalLineHeights=3)
for record in [*reference['inputs'], *reference['outputs'], *reference['fontInputs'],
               reference['executable'], reference['executionLog']]:
    assert entry(record['path']) == record
regressions = json.loads((ROOT / 'regressions.json').read_text())
assert regressions['counts'] == dict(itemizationAndParagraph=34, mixedFont=17, lineShaping=34,
                                    lineGeometry=35, paragraphLayout=37, paragraphPaths=50)
assert regressions['total'] == len(regressions['cases']) == 207
for record in [*regressions['previousReports'], regressions['worker'], regressions['previousEvidence']]:
    assert entry(record['path']) == record
assert regressions['previousEvidence']['sha256'] == '0665a6ffd4bc4cbd31acc335ed6974963441f6a1e26850ead40d6b5c886c5158'
for case in regressions['cases']:
    for key in ['request', 'bundle', 'priorResponse', 'response']:
        assert entry(case[key]['path']) == case[key]
report = dict(
    format='musteroffice.source-frame-library-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Native horizontal shape-local unpainted text-frame flow, insets, margins, paragraph spacing and alignment. Library stage; not page text rendering or product replacement.',
    checks=dict(rustTests=454, newRustTests=9, strictClippy=True, rustfmt=True,
                sourceCompilerWasmTarget='compiled-only', schemasChecked=68,
                schemaAndTypeScriptDelta=False, exactNativeRegressionRequests=207,
                newRuntimeParityBatches=0),
    changedPreviousSources=sorted(changed), addedSources=sorted(added),
    sourceFiles=[entry(p) for p in sorted(sources)],
    validationLogs=[entry(ROOT / n) for n in logs], rustTestNames=tests, newTestNames=new,
    reference=entry(ROOT / 'reference.json'), independentReference=reference,
    nativeRegressions=entry(ROOT / 'regressions.json'), regressionEvidence=regressions,
    developmentArtifacts=dict(nativeFrameTests=reference['executable'], nativeWorker=regressions['worker']),
    nativeComponentLibrary=old['nativeComponentLibrary'],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'], dependencyChanges='None; all manifests and lockfiles unchanged.',
    limitations=[
        'Draft native frame policy; no Office/WPS layout/typography certification. Natural/100 percent line metrics, end-style strut, RTL indentation and paragraph spacing need target application verification.',
        'Only horizontal single-column, square wrapping, no autofit, top/center/bottom anchor and overflow modes accepted. Unsupported bullets/tabs/justification/hanging punctuation and source character semantics retain explicit diagnostics.',
        'Output is shape-local unpainted geometry. Native text paints/decorations/effects, world transforms, clipping and whole-page atomic publication remain unconnected; source pages still reject text.',
        'Q32 bounds cover conversion from evaluated binary64/exact insets and alignment half-rounding only; not source formula errors, line-break sensitivity or full device error budgets.',
        '15 owned synthetic-font PPTX fixtures cover simple A/empty paragraphs, exact line widths/positions and polygon bounds, not multilingual visual fidelity.',
        'Component work is charged before forwarding; aggregate glyph/path limits after each paragraph. This is not a hard RSS bound, measured performance gain, or whole-page resource limit.',
        'Fault/cancellation test cases use process isolation because Native component invalidation is process-wide and permanent. This does not implement a production process pool.',
        '207 ordinary existing Native requests replayed; separate historical allocation-fault scenarios were not all rerun. No new runtime operation, WASM execution comparison, release rebuild, installer measurement or Musterwork E0-E3 completion.',
    ])
raw = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as file:
        file.write(raw)
print(json.dumps(dict(evidence=entry(OUTPUT) if '--seal' in sys.argv else None,
                      sources=len(sources), checks=report['checks'], native=reference['counts'])))
