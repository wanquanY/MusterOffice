"""Seal native source glyph compiler evidence without changing old releases."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path('.codex-work/source-glyphs')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-manifest-layout-library-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-source-glyph-library-verification.json')


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


assert not OUTPUT.exists(), 'Never overwrite an evidence seal'
assert entry(PREVIOUS)['sha256'] == '3d42bced6f9a39883e46c281450422a1430ce3ad8a9604ef827eec1c874ef408'
old = json.loads(PREVIOUS.read_text())
allowed = {'Cargo.lock', 'crates/mo-harfbuzz-sys/Cargo.toml',
           'crates/mo-presentation-compile/Cargo.toml', 'crates/mo-presentation-compile/src/lib.rs',
           'README.md', 'docs/README.md', 'docs/implementation/development.md',
           'docs/implementation/progress.md', 'docs/implementation/manifest-layout.md'}
changed = [p['path'] for p in old['sourceFiles'] if entry(p['path']) != p]
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = ['crates/mo-presentation-compile/src/source_text.rs',
         *['crates/mo-presentation-compile/src/source_text/'+n+'.rs'
           for n in ['types', 'assemble', 'script', 'style', 'budget']],
         'crates/mo-presentation-compile/tests/source_glyphs.rs',
         'crates/mo-harfbuzz-sys/examples/source_glyphs.rs',
         'tools/verification/source-glyph-reference.py',
         'tools/verification/source-glyph-evidence.py', 'docs/implementation/source-glyphs.md']
sources = {p['path'] for p in old['sourceFiles']} | set(added)
assert len(sources) == len(old['sourceFiles']) + len(added)
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
# Exact reconstruction proves all external dependency versions are unchanged.
lock = Path('Cargo.lock').read_text()
for package, deps in [('mo-harfbuzz-sys', ['mo-opc', 'mo-pptx', 'mo-presentation-compile']),
                      ('mo-presentation-compile', ['mo-opc', 'mo-text', 'mo-unicode'])]:
    start = lock.index('name = "'+package+'"')
    end = lock.index('[[package]]', start)
    section = lock[start:end]
    for dep in deps:
        line = ' "'+dep+'",\n'
        assert section.count(line) == 1
        section = section.replace(line, '')
    lock = lock[:start] + section + lock[end:]
assert hashlib.sha256(lock.encode()).hexdigest() == next(p['sha256'] for p in old['sourceFiles'] if p['path'] == 'Cargo.lock')
for record in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(), *old['standardInputs'],
               old['nativeComponentLibrary']]:
    assert entry(record['path']) == record, record['path']
logs = ['compiler-tests.log', 'workspace-tests.log', 'clippy.log', 'wasm-check.log',
        'fmt.log', 'schema-check.log', 'native-build.log', 'reference.log']
for name in logs:
    log = (ROOT / name).read_text()
    assert not any(s in log for s in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
new = [name for name in tests if name not in old['rustTestNames']]
assert len(tests) == 445 and len(new) == 11
schema = (ROOT / 'schema-check.log').read_text()
assert len(re.findall(r'^check contracts/generated/', schema, re.M)) == 68
reference = json.loads((ROOT / 'reference.json').read_text())
assert reference['counts'] == dict(packages=8, paragraphs=9, sourceRanges=15, fontBindings=25,
                                  shapeOperations=9, pathOperations=18, shapeErrors=3, pathErrors=6,
                                  scenes=12, glyphs=22, originCoordinates=44, pathBounds=28)
for p in [*reference['inputs'], *reference['outputs'], *reference['fontInputs'], reference['executable']]:
    assert entry(p['path']) == p
report = dict(
    format='musteroffice.source-glyph-library-verification/1',
    previousEvidence=entry(PREVIOUS),
    scope='Native PPTX source runs/cascade/script-font selection to manifest shaping and local unpainted glyph paths; not a page renderer or product replacement.',
    checks=dict(rustTests=445, newRustTests=11, clippy='passed', rustfmt='passed',
                sourceCompilerWasmTarget='compiled-only', schemasChecked=68,
                schemaAndTypeScriptDelta=False, newRuntimeParityBatches=0),
    changedPreviousSources=sorted(changed), addedSources=sorted(added),
    sourceFiles=[entry(p) for p in sorted(sources)],
    validationLogs=[entry(ROOT / n) for n in logs],
    rustTestNames=tests, newTestNames=new,
    reference=entry(ROOT / 'reference.json'), independentReference=reference,
    developmentArtifacts=dict(sourceGlyphExample=reference['executable']),
    nativeComponentLibrary=old['nativeComponentLibrary'],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],
    dependencyChanges='Only six edges to existing local crates; no external version changes. Runtime compiler now depends on mo-text/mo-unicode; remaining edges are development dependencies.',
    limitations=[
        'Source glyph selection profile is draft; no Office/WPS font-slot or layout certification.',
        'Native text-frame flow recipe, positioning, alignment, paint and page coordinate budgets remain unconnected; source pages still reject visible text.',
        'Symbol mapping, unsupported scripts, fields, nonzero spacing/baseline, caps, kumimoji, normalizeHeight and character direction override remain explicit diagnostics.',
        'The current glyph-path operation does not paint native underline/strike/fill/effects; those remain in source styles for the page compiler.',
        'The owned synthetic font corpus proves source connections/numerics, not visual quality or multilingual font coverage.',
        'Source preparation budgets span one object; downstream paragraph operations do not constitute whole-page preflight or atomic publication.',
        'No new runtime operation, Native/WASM execution comparison, release rebuild, product performance/RSS/installer measurement or E0-E3 completion.',
    ])
OUTPUT.write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(evidence=entry(OUTPUT), sources=len(sources), rustTests=len(tests), native=reference['counts'])))
