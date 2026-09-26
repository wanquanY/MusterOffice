"""Seal the actual WPS miter investigation without claiming a production fix."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('.codex-work/miter-compat')
OUTPUT = Path('docs/reviews/evidence/2026-09-24-miter-compatibility-observations.json')


def read(p):
    return json.loads(Path(p).read_text())


def entry(p):
    p = Path(p)
    data = p.read_bytes()
    return {'path': str(p), 'byteLength': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


previous = 'docs/reviews/evidence/2026-09-24-stroke-author-verification.json'
prior = read(previous)
assert entry(previous)['sha256'] == 'cd4e7b42849c1acc40b59b3f44b028af75bb3fbc8638f0eaf752e6d1d60c7a4c'
for a in prior['artifacts'].values():
    assert entry(a['path']) == a, a['path']
changed = []
for source in prior['sourceFiles']:
    if entry(source['path']) != source:
        changed.append(source['path'])
assert set(changed) == {'docs/implementation/stroke-author.md', 'docs/implementation/progress.md'}, changed

inputs = read(ROOT / 'inputs.json')
observed = read(ROOT / 'observations.json')
independent = read(ROOT / 'independent.json')
captures = read(ROOT / 'wps-captures.json')
assert len(inputs['files']) == len(observed['cases']) == len(independent) == len(captures) == 5
assert observed['inputs'] == entry(ROOT / 'inputs.json')
assert observed['independentValidation'] == entry(ROOT / 'independent.json')
assert inputs['nativeWasmPptxAndPixelsEqual']
for k in ['nativeCli', 'rustWasm', 'rasterWasm']:
    assert inputs[k] == prior['artifacts'][k]
for c, report, validated in zip(inputs['files'], observed['cases'], independent, strict=True):
    assert c['name'] == report['name'] == validated['name']
    assert len(c['mapping']) == len(report['mapping']) == 12
    for k in ['page', 'pixels', 'export', 'pptx', 'plan']:
        assert entry(c[k]['path']) == c[k], (c['name'], k)
    assert validated['result'] == 'passed' and not validated['differences']
    assert validated['fileSha256'] == c['pptx']['sha256']
    assert validated['requestSha256'] == c['export']['sha256']
    assert validated['nativeObjectsCompared'] == 12 and len(validated['schemasChecked']) == 6
    for k in ['kernelPreview', 'wpsCanvas']:
        assert entry(report[k]['path']) == report[k]
    capture = next(v for v in captures if v['name'] == c['name'])
    assert capture['pptxSha256'] == c['pptx']['sha256']
    # Bind the analysis to the exact original capture, without publishing UI chrome.
    capture['screenshotSha256'] = entry(capture['screenshotPath'])['sha256']
    for m, sample in zip(c['mapping'], report['mapping'], strict=True):
        assert all(sample[k] == v for k, v in m.items())
        for backend in ['kernel', 'wps']:
            assert sample[backend]['pixels'] > 0
assert observed['coordinateUnitStudy']['nativePixelsExactlyInvariant']

paths = {s['path'] for s in prior['sourceFiles']}
paths.update(['tools/verification/miter-compat-fixtures.mjs', 'tools/verification/miter-compat-observations.py',
              'tools/verification/miter-compat-evidence.py', 'docs/implementation/miter-compatibility.md'])
for p in paths:
    if Path(p).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000
result = {
    'format': 'musteroffice.miter-compatibility-investigation/1',
    'previousEvidence': entry(previous), 'productionArtifacts': prior['artifacts'],
    'productionCodeAndArtifactsUnchanged': True,
    'previousProductionChecks': {'rustTests': 252, 'mainLogicalBatches': 2549, 'rerunInThisInvestigation': False},
    'newChecks': {'physicalEmuDocuments': 5, 'nativeEditablePaths': 60,
                  'nativeWasmPptxAndPixelComparisons': 5, 'independentFileChecks': 5, 'officialXsdPartChecks': 30,
                  'wpsCanvasesObserved': 5, 'wpsPathSilhouettesMeasured': 60,
                  'pathCoordinateUnitsNativePixelInvariant': True},
    'inputs': inputs, 'independentValidation': independent, 'observations': observed,
    'captures': captures, 'sourceFiles': [entry(p) for p in sorted(paths)],
    'findings': [
        'Changing only integer path coordinate units preserves exact native pixel tiles and causes no unit-multiplier-sized WPS difference in the observed canvases.',
        'WPS shows intermediate truncated joins in a fixed-width limit series. Changing a scalar limit in a full-miter-or-bevel backend cannot express those intermediate shapes.',
        'Pen-width and angle studies require further target-profile investigation; no universal coefficient or WPS-internal algorithm is inferred.',
    ],
    'gateStatus': {'fullMiterFidelity': 'notPassed', 'powerPoint': 'notTested', 'editRoundTrip': 'notTested', 'musterworkReplacement': 'notReady'},
    'scope': 'Owned native PPTX and actual WPS canvas investigation. No production geometry rewrite, acceptance-threshold relaxation, performance/size improvement or complete compatibility claim.',
}
raw = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
summary = {'checks': result['newChecks'], 'sources': len(paths), 'productionArtifactsUnchanged': len(prior['artifacts'])}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
    summary['evidence'] = entry(OUTPUT)
print(json.dumps(summary, indent=2))
