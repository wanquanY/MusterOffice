"""Freeze the library-only text cascade stage without claiming new runtime parity."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path('.codex-work/text-cascade')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-text-body-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-text-cascade-library-verification.json')


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


assert entry(PREVIOUS)['sha256'] == '0665a6ffd4bc4cbd31acc335ed6974963441f6a1e26850ead40d6b5c886c5158'
old = json.loads(PREVIOUS.read_text())
changed = [p['path'] for p in old['sourceFiles'] if entry(p['path']) != p]
allowed = {'README.md', 'docs/README.md', 'docs/implementation/development.md',
           'docs/implementation/progress.md', 'crates/mo-pptx/src/source/text.rs',
           'crates/mo-pptx/src/source/inheritance.rs'}
assert set(changed) <= allowed, changed
new = ['crates/mo-pptx/src/source/text/cascade.rs',
       'crates/mo-pptx/src/source/text/cascade/types.rs',
       'crates/mo-pptx/src/source/text/cascade/properties.rs',
       'crates/mo-pptx/src/source/text/cascade/chain.rs',
       'crates/mo-pptx/tests/text_cascade.rs',
       'tools/verification/text-cascade-reference.py',
       'tools/verification/text-cascade-evidence.py',
       'docs/implementation/text-cascade.md']
sources = {p['path'] for p in old['sourceFiles']} | set(new)
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
for p in old['artifacts'].values():
    assert entry(p['path']) == p, p['path']
for p in old['standardInputs']:
    assert entry(p['path']) == p, p['path']
for p in old['sourceFiles']:
    if p['path'].startswith(('contracts/generated/', 'packages/contracts/')) or p['path'] in ['Cargo.lock', 'pnpm-lock.yaml']:
        assert entry(p['path']) == p, p['path']
logs = ['targeted-tests.log', 'workspace-tests.log', 'clippy.log', 'wasm-check.log', 'schema-check.log', 'fmt.log', 'reference.log']
for name in logs:
    text = (ROOT / name).read_text()
    assert not any(s in text for s in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
test_names = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
assert len(test_names) == 413
targeted = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'targeted-tests.log').read_text(), re.M)
assert len(targeted) == 13
reference = json.loads((ROOT / 'reference.json').read_text())
assert reference['counts'] == dict(packages=24, cascaded=18, unresolved=6, paragraphs=199,
                                  characterStyles=400, attributeValues=9789, attributeOrigins=8194,
                                  declarations=9, validParts=116, invalidParts=4)
for c in reference['cases']:
    assert entry(c['sourcePath'])['sha256'] == c['sourceSha256']
    assert entry(c['responsePath'])['sha256'] == c['responseSha256']
report = dict(
    format='musteroffice.text-cascade-library-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Rust library source paragraph/run cascade. No new CLI/WASM export, runtime parity, page text rendering or target-application acceptance.',
    checks=dict(rustTests=413, newRustTests=13, strictClippy=True, rustfmt=True,
                wasmTargetCompilation=True, schemaGenerationCheck=True, unchangedRuntimeSchemas=68,
                newRuntimeParityBatches=0, previousFrozenRuntimeParityBatches=10064),
    changedPreviousSources=changed, addedSources=new,
    sourceFiles=[entry(p) for p in sorted(sources)],
    validationLogs=[entry(ROOT / p) for p in logs],
    rustTestNames=test_names, newTestNames=targeted,
    reference=entry(ROOT / 'reference.json'), independentReference=reference,
    previousReleaseArtifactsVerifiedUnchanged=old['artifacts'],
    standardInputs=old['standardInputs'],
    limitations=[
        'Profile drawingml-text-cascade-draft-v1 states precedence explicitly; Office/WPS conflicts and placeholder paragraph mapping remain unverified.',
        'Selected whole declarations still require font/resource/color/effect/interaction resolution. No page text rendering, field evaluation, autofit or production resource binding is claimed.',
        'The new library has Native Rust tests and a WASM target compilation check. The previous 10064 logical batches certify the previous frozen runtime build, not this new cascade.',
        'Four intentionally invalid XSD probes are listed. Ignoring defPPr during cascade is not a package-validity assertion.',
        'No new release build, full installer size, product latency/RSS, Office/WPS round trip or Musterwork E0-E3 acceptance was performed.',
    ],
)
raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
print(json.dumps(dict(checks=report['checks'], sourceFiles=len(sources), changed=changed,
                     evidence=entry(OUTPUT) if '--seal' in sys.argv else None), indent=2))
