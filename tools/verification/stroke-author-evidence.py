"""Bind authored stroke/PPTX work to real artifacts and retained app differences.

Default validates only. --seal creates a new immutable milestone, never replaces.
"""
import hashlib
import json
import platform
import re
import sys
from pathlib import Path

ROOT = Path('.codex-work/stroke-author')
OUTPUT = Path('docs/reviews/evidence/2026-09-24-stroke-author-verification.json')


def read(p):
    return json.loads(Path(p).read_text())


def sha(b):
    return hashlib.sha256(b).hexdigest()


def entry(p):
    p = Path(p)
    b = p.read_bytes()
    return {'path': str(p), 'byteLength': len(b), 'sha256': sha(b)}


previous = 'docs/reviews/evidence/2026-09-24-stroke-raster-verification.json'
prior = read(previous)
assert entry(previous)['sha256'] == '98f5dda16a0eb1dacdae923e82fb911c78491318fe400c1bb3b42abebcb408d1'
artifacts = {k: entry(v['path']) for k, v in prior['artifacts'].items()}
for k in artifacts.keys() - {'nativeCli', 'nativeRasterWorker', 'rustWasm'}:
    assert artifacts[k] == prior['artifacts'][k], k


def hashes(r, raster=False):
    aliases = {'nativeSha256': 'nativeCli', 'nativeCliSha256': 'nativeCli',
               'nativeWorkerSha256': 'nativeRasterWorker' if raster else 'nativeWorker',
               'wasmSha256': 'rustWasm', 'rustWasmSha256': 'rustWasm', 'wasmKernelSha256': 'rustWasm',
               'componentSha256': 'rasterWasm' if raster else 'cppWasm',
               'componentWasmSha256': 'cppWasm', 'adapterSha256': 'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256', 'nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256', 'rustWasmSha256', 'wasmKernelSha256'])
    for k, a in aliases.items():
        if k in r:
            assert r[k] == artifacts[a]['sha256'], k


def case_files(r):
    for c in r.get('cases', []):
        for k in ['request', 'canonicalRequest', 'response', 'plan', 'source', 'font', 'bundle', 'frame', 'pixels', 'scene', 'pptx', 'export']:
            if k + 'Path' in c:
                assert entry(c[k + 'Path'])['sha256'] == c[k + 'Sha256'], (c['name'], k)


regressions = {}
for name in ['document', 'opc', 'source', 'color', 'font', 'export']:
    p = ROOT / (name + '-regression.json')
    r = read(p)
    old = prior['regressionReports'][name]['report']
    assert entry(old['path']) == old
    assert r['cases'] == read(old['path'])['cases'], name
    hashes(r)
    case_files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r['cases']), 'semanticResultsUnchanged': True}

historical = {
    'shaping': ('text-shaping', 'textReport'), 'unicode': ('unicode-text', 'unicodeReport'),
    'cascade': ('font-cascade', 'cascadeReport'), 'bidi': ('bidi', 'bidiReport'),
    'bidiConformance': ('bidi', 'conformanceReport'),
    'itemizationAndParagraph': ('font-fallback', 'updatedItemizationReport'),
    'mixedFont': ('font-fallback', 'fallbackReport'), 'lineBreaking': ('line-break', 'lineBreakReport'),
    'fontMetrics': ('font-metrics', 'metricsReport'), 'lineShaping': ('line-shaping', 'lineShapeReport'),
    'lineGeometry': ('line-geometry', 'lineGeometryReport'), 'paragraphLayout': ('paragraph-layout', 'paragraphLayoutReport'),
    'fontOutlines': ('font-outlines', 'outlinesReport'), 'paragraphPaths': ('paragraph-paths', 'paragraphPathsReport'),
}
for name, (milestone, key) in historical.items():
    p = prior['regressionReports'][name]['report']['path']
    r = read(p)
    old = read('docs/reviews/evidence/2026-09-24-' + milestone + '-verification.json')[key]
    field = 'batches' if name == 'bidiConformance' else 'cases'
    assert r[field] == old[field], name
    hashes(r)
    case_files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r[field]), 'semanticResultsUnchanged': True}

angles = read('docs/reviews/evidence/2026-09-24-static-rotation-export-verification.json')
for name, p, key in [('placement', '.codex-work/page-placement/parity.json', 'pagePlacementReport'),
                     ('groups', '.codex-work/angle-export/parity.json', 'groupPlacementReport'),
                     ('angles', '.codex-work/angle-export/angle-parity.json', 'angleExportReport'),
                     ('angleSource', '.codex-work/angle-export/source-parity.json', 'angleSourceReport')]:
    r = read(p)
    assert r['cases'] == angles[key]['cases'], name
    hashes(r, name in ['placement', 'groups'])
    case_files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r['cases']), 'semanticResultsUnchanged': True}


def old_page_metadata(v):
    if isinstance(v, list):
        return [old_page_metadata(x) for x in v]
    if isinstance(v, str):
        return v.replace('author-page-solid-paths-certified-cpu-v2-draft', 'author-page-solid-paths-certified-cpu-v1-draft').replace('UnresolvedStrokeParameters', 'StrokePaint').replace('unresolvedStrokeParameters', 'strokePaint')
    if not isinstance(v, dict):
        return v
    out = {}
    for k, x in v.items():
        if k == 'authorMiterLimitErrorBound':
            assert x == '0'
        elif k == 'paint':
            assert x == 'fill'
        else:
            out[k] = old_page_metadata(x)
    return out


reports = {}
for name in ['path', 'scene', 'page']:
    p = Path('.codex-work') / ('page-render' if name == 'page' else name + '-raster') / 'parity.json'
    r = read(p)
    old = prior[name + ('RenderReport' if name == 'page' else 'RasterReport')]
    hashes(r, True)
    case_files(r)
    assert r['exactPixelAndMetadataEquality']
    if name != 'page':
        assert r['cases'] == old['cases'], name
    else:
        lookup = {c['name']: c for c in r['cases']}
        for c in old['cases']:
            n = lookup[c['name']]
            for k in ['requestSha256', 'status', 'pixelsSha256', 'frameSha256']:
                assert n.get(k) == c.get(k), (c['name'], k)
            for k in ['response', 'plan']:
                normalized = old_page_metadata(read(n[k + 'Path']))
                assert sha(json.dumps(normalized, ensure_ascii=False, separators=(',', ':')).encode()) == c[k + 'Sha256'], (c['name'], k)
        assert len(r['cases']) == 131 and len(old['cases']) == 74 and r['exactPlanEquality']
    reports[name] = r
    regressions[name + 'Raster'] = {'report': entry(p), 'batches': len(r['cases']), 'oldRequestsStatusesPixelsAndFramesUnchanged': True}
assert sum(v['batches'] for v in regressions.values()) == 2549

references = {}
for name in ['path', 'scene', 'page']:
    p = Path('.codex-work') / ('page-render' if name == 'page' else name + '-raster') / 'reference.json'
    r = read(p)
    lookup = {c['name']: c for c in reports[name]['cases']}
    for c in r['cases']:
        assert c['frameSha256'] == lookup[c['name']]['frameSha256']
    references[name] = r

external = read(ROOT / 'external/inputs.json')
hashes(external)
case_files(external)
assert len(external['cases']) == 48 and external['nativeWasmPptxEqual'] and external['nativeWasmPixelsAndMetadataEqual']
independent = read(ROOT / 'external/independent.json')
observed = read(ROOT / 'external/observations.json')
assert observed['inputs'] == entry(ROOT / 'external/inputs.json')
assert observed['independentFileValidation'] == entry(ROOT / 'external/independent.json')
assert len(independent) == len(observed['cases']) == 48
for c, validated, app in zip(external['cases'], independent, observed['cases'], strict=True):
    assert validated['name'] == app['name'] == c['name']
    assert validated['result'] == 'passed' and not validated['differences']
    assert validated['fileSha256'] == c['pptxSha256']
    assert validated['requestSha256'] == c['exportSha256']
    for k in ['pptx', 'pdf', 'kernelPreview', 'libreOfficePreview']:
        assert entry(app[k]['path']) == app[k]
    for k, v in app['checks'].items():
        if k != 'lastExplicitPdfMiterRatioMatches' and isinstance(v, bool):
            assert v, (c['name'], k)
assert sum(c['checks'].get('lastExplicitPdfMiterRatioMatches') is False for c in observed['cases']) == 11

grid = read(ROOT / 'grid/inputs.json')
wps = read(ROOT / 'grid/wps-observation.json')
for k in ['page', 'export', 'pptx', 'plan', 'pixels', 'nativeCli', 'rustWasm']:
    assert entry(grid[k]['path']) == grid[k]
assert grid['nativeCli'] == artifacts['nativeCli'] and grid['rustWasm'] == artifacts['rustWasm']
assert grid['nativeWasmPptxEqual'] and grid['nativeWasmPixelsAndMetadataEqual'] and len(grid['mapping']) == 12
for k in ['inputs', 'nativePptx', 'editableXmlValidation', 'canvas', 'kernelPreview']:
    assert entry(wps[k]['path']) == wps[k]
assert wps['nativePptx'] == grid['pptx'] and wps['foregroundPreserved']
grid_validation = read(wps['editableXmlValidation']['path'])
assert grid_validation['result'] == 'passed' and grid_validation['fileSha256'] == grid['pptx']['sha256']

contracts = read(ROOT / 'contracts.json')
assert contracts['schemas'] == 49
assert [contracts[k] for k in ['pathRasterResponses', 'sceneRasterResponses', 'pageRenderResponses', 'pageCompileResponses']] == [193, 251, 131, 131]
changed = [s['path'] for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/') and entry(s['path']) != s]
expected = ['document', 'kernel-request', 'kernel-response', 'page-compile-response', 'page-placement-request', 'page-raster-response', 'page-render-request', 'pptx-export-request', 'transaction']
assert sorted(changed) == sorted('contracts/generated/' + n + '.schema.json' for n in expected)
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'rust-tests.log').read_text(), re.M)
assert len(tests) == 252 and set(prior['rustTestNames']).issubset(tests)
for log in ['rust-tests.log', 'clippy.log', 'types-check.log', 'rustfmt.log']:
    assert 'error:' not in (ROOT / log).read_text() and 'FAILED' not in (ROOT / log).read_text()
for p in ['Cargo.lock', 'pnpm-lock.yaml']:
    assert entry(p) == next(s for s in prior['sourceFiles'] if s['path'] == p)

paths = {s['path'] for s in prior['sourceFiles']}
for base in ['components', 'crates', 'tools', 'contracts', 'packages', 'fixtures', 'docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs', '.toml', '.json', '.ts', '.mjs', '.py', '.md', '.h', '.cpp', '.bin', '.ttf', '.otf', '.ttc', '.patch', '.txt'] and '__pycache__' not in p.parts:
            paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
result = {
    'format': 'musteroffice.stroke-author-verification/1',
    'scope': 'Explicit authored solid strokes to draft page raster and native editable PPTX; target-application miter differences remain open.',
    'previousEvidence': entry(previous), 'artifacts': artifacts,
    'artifactByteDeltas': {k: v['byteLength'] - prior['artifacts'][k]['byteLength'] for k, v in artifacts.items()},
    'artifactSizeScope': 'Uncompressed incomplete development artifacts, not full kernel or Musterwork installer sizes; no performance/RSS gate claim.',
    'environment': {'platform': platform.system() + ' ' + platform.release(), 'architecture': platform.machine(), 'rust': '1.92.0', 'node': '23.5.0', 'python': platform.python_version()},
    'checks': {'rustTests': 252, 'newRustTests': 5, 'nativeWasmLogicalBatches': 2549, 'previousLogicalBatchesPreserved': 2492,
               'newPageBatches': 57, 'newRenderedPages': 48, 'newRejectedPages': 9, 'runtimeSchemas': 49, 'changedSchemas': 9, 'unchangedSchemas': 40,
               'auxiliaryNativeWasmEditablePptxAndPixelComparisons': 49, 'independentEditableFiles': 49,
               'officialXsdPartChecks': sum(len(c['schemasChecked']) for c in independent) + len(grid_validation['schemasChecked']),
               'libreOfficePagesObserved': 48, 'wpsGridObjectsObserved': 12, 'strictClippy': True, 'rustfmt': True, 'typescript': True},
    'pathRasterReport': reports['path'], 'sceneRasterReport': reports['scene'],
    'pageRenderReport': reports['page'], 'regressionReports': regressions,
    'pageContractTransition': {'oldBatches': 74, 'oldRequestStatusPixelsFrameUnchanged': True,
                             'oldPlanAndResponseHashesReconstructed': 74,
                             'changes': ['page profile v2', 'zero author miter bound on old pages', 'explicit fill source role', 'missing stroke parameters diagnostic replaces blanket unsupported stroke diagnostic']},
    'parameterAndGeometryReferences': references, 'externalInputs': external,
    'independentPptxValidation': independent, 'libreOfficeObservations': observed,
    'wpsGrid': grid, 'wpsGridValidation': grid_validation, 'wpsObservations': wps,
    'componentEvidence': {'source': entry(previous), 'batches': 174, 'referencePixels': 61445,
                          'reusedWithoutComponentBinaryOrAdapterChanges': True, 'rerunInThisMilestone': False},
    'contractReport': contracts, 'changedSchemas': changed, 'rustTestNames': tests,
    'dependencyChanges': {'externalRuntimeVersions': [], 'lockFilesUnchanged': True, 'newDevelopmentDependencies': []},
    'sourceFiles': [entry(p) for p in sorted(paths)],
    'limitations': [
        'Draft miter rendering uses Skia miter-to-bevel semantics. WPS shows visibly truncated tips for lim=400000 in three grid objects; LibreOffice uses the same explicit PDF M value across eleven different declarations. These are unresolved fidelity differences.',
        'Missing cap/join and inherited shape paints fail explicitly pending target-profile/style resolution. No dashed/compound lines, arrows, group strokes, effects, image paints or GPU strokes yet.',
        'Coordinate/parameter bounds do not certify stroke ink edges, miter threshold continuity, antialiasing, or application pixel equality.',
        'No PowerPoint, WPS/LibreOffice edit-roundtrip, Windows/Linux/browser Worker, production pool or complete CPU/RSS/performance acceptance.',
        'Full text/image pages, advanced editable objects, playback, public Agent integration and Musterwork E0-E3 remain unfinished.',
    ],
}
raw = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
summary = {'checks': result['checks'], 'sources': len(paths), 'artifactByteDeltas': result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
    summary['evidence'] = entry(OUTPUT)
print(json.dumps(summary, indent=2))
