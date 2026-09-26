"""Bind geometry declarations to real artifacts and unchanged previous behavior.

Without --seal this verifies only. Sealing exclusively creates a new file.
Source-index regressions may add only SourceObject.geometry; existing fields,
edit candidates, color/style queries and rendered frames must remain identical.
"""
import copy
import hashlib
import json
import platform
import re
import sys
from pathlib import Path

ROOT = Path('.codex-work/source-geometry')
OUTPUT = Path('docs/reviews/evidence/2026-09-24-source-geometry-verification.json')
def read(p): return json.loads(Path(p).read_text())
def sha(b): return hashlib.sha256(b).hexdigest()
def entry(p):
    p = Path(p)
    b = p.read_bytes()
    return {'path': str(p), 'byteLength': len(b), 'sha256': sha(b)}
previous = 'docs/reviews/evidence/2026-09-24-line-color-verification.json'
assert entry(previous)['sha256'] == '525219dd130e6b38d4584e7143c9c0a2818a6b5ee758128e178fa574ffa2bc03'
prior = read(previous)
artifacts = {k: entry(v['path']) for k, v in prior['artifacts'].items()}
for k in artifacts:
    if k not in ['nativeCli', 'rustWasm']: assert artifacts[k] == prior['artifacts'][k], k

def hashes(r, raster=False):
    aliases = {'nativeSha256': 'nativeCli', 'nativeCliSha256': 'nativeCli',
               'nativeWorkerSha256': 'nativeRasterWorker' if raster else 'nativeWorker',
               'wasmSha256': 'rustWasm', 'rustWasmSha256': 'rustWasm', 'wasmKernelSha256': 'rustWasm',
               'componentSha256': 'rasterWasm' if raster else 'cppWasm', 'componentWasmSha256': 'cppWasm', 'adapterSha256': 'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256', 'nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256', 'rustWasmSha256', 'wasmKernelSha256'])
    for k, a in aliases.items():
        if k in r: assert r[k] == artifacts[a]['sha256'], k
def files(r):
    for c in r.get('cases', []):
        for k in ['request', 'canonicalRequest', 'response', 'plan', 'source', 'font', 'bundle', 'frame', 'pixels', 'scene', 'pptx', 'export', 'editRequest', 'originalSource']:
            if k + 'Path' in c: assert entry(c[k + 'Path'])['sha256'] == c[k + 'Sha256'], (c['name'], k)
def without_geometry(response):
    r = copy.deepcopy(response)
    for surface in r.get('index', r).get('surfaces', {}).values():
        for obj in surface['objects']: obj.pop('geometry', None)
    return r
def compare_index_cases(current, old):
    assert len(current) == len(old)
    changed = 0
    for a, b in zip(current, old):
        if a == b: continue
        assert a.keys() == b.keys()
        assert {k: v for k, v in a.items() if k != 'responseSha256'} == {k: v for k, v in b.items() if k != 'responseSha256'}, a['name']
        response = read(a['responsePath'])
        assert response.get('status') == 'inspected' or ('status' not in response and 'surfaces' in response)
        normalized = without_geometry(response)
        # Reconstruct the exact old Rust JSON bytes and compare the sealed
        # digest. This also works for auxiliary reports whose old files moved.
        raw = json.dumps(normalized, ensure_ascii=False, separators=(',', ':')).encode()
        assert sha(raw) == b['responseSha256'], a['name']
        changed += 1
    return changed

regressions = {}
for name in ['document', 'opc', 'source', 'color', 'font', 'export']:
    p = ROOT / (name + '-regression.json')
    r = read(p)
    old_entry = prior['regressionReports'][name]['report']
    assert entry(old_entry['path']) == old_entry
    old = read(old_entry['path'])
    normalized = copy.deepcopy(r['cases'])
    if name == 'source':
        for c in normalized:
            if 'response' in c: c['response'] = without_geometry(c['response'])
        for c in r['cases']:
            if 'source' in c: assert entry(c['source'])['sha256'] == c['sourceSha256']
        for c in r['outputs']: assert entry(c['output'])['sha256'] == c['sha256']
    assert normalized == old['cases'], name
    hashes(r)
    files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r['cases']), 'previousSemanticsUnchanged': True,
                         'sourceGeometryAdditionsOnly': name == 'source'}
historical = {
    'shaping': ('text-shaping', 'textReport'), 'unicode': ('unicode-text', 'unicodeReport'),
    'cascade': ('font-cascade', 'cascadeReport'), 'bidi': ('bidi', 'bidiReport'), 'bidiConformance': ('bidi', 'conformanceReport'),
    'itemizationAndParagraph': ('font-fallback', 'updatedItemizationReport'), 'mixedFont': ('font-fallback', 'fallbackReport'),
    'lineBreaking': ('line-break', 'lineBreakReport'), 'fontMetrics': ('font-metrics', 'metricsReport'),
    'lineShaping': ('line-shaping', 'lineShapeReport'), 'lineGeometry': ('line-geometry', 'lineGeometryReport'),
    'paragraphLayout': ('paragraph-layout', 'paragraphLayoutReport'), 'fontOutlines': ('font-outlines', 'outlinesReport'),
    'paragraphPaths': ('paragraph-paths', 'paragraphPathsReport')}
for name, (milestone, key) in historical.items():
    p = prior['regressionReports'][name]['report']['path']
    r = read(p)
    old = read('docs/reviews/evidence/2026-09-24-' + milestone + '-verification.json')[key]
    field = 'batches' if name == 'bidiConformance' else 'cases'
    assert r[field] == old[field], name
    hashes(r)
    files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r[field]), 'allPreviousCasesUnchanged': True}
for name in ['placement', 'groups', 'angles', 'angleSource', 'sourceLines', 'lineStyles', 'lineColors']:
    p = prior['regressionReports'][name]['report']['path']
    r = read(p)
    snapshot = ROOT / 'previous' / (name + '.json')
    assert entry(snapshot)['sha256'] == prior['regressionReports'][name]['report']['sha256'], name
    old = read(snapshot)
    additions = compare_index_cases(r['cases'], old['cases']) if name in ['angles', 'angleSource', 'sourceLines'] else 0
    if name not in ['angles', 'angleSource', 'sourceLines']: assert r['cases'] == old['cases'], name
    hashes(r, name in ['placement', 'groups'])
    files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r['cases']), 'previousSemanticsUnchanged': True, 'responsesAddingOnlyGeometry': additions}
rasters = {}
references = {}
for name in ['path', 'scene', 'page']:
    p = Path('.codex-work') / ('page-render' if name == 'page' else name + '-raster') / 'parity.json'
    r = read(p)
    assert r['cases'] == prior[name + ('RenderReport' if name == 'page' else 'RasterReport')]['cases']
    hashes(r, True)
    files(r)
    assert r['exactPixelAndMetadataEquality']
    rasters[name] = r
    regressions[name + 'Raster'] = {'report': entry(p), 'batches': len(r['cases']), 'allPreviousCasesUnchanged': True}
    reference = prior['reusedGeometryReferences'][name]['report']
    assert entry(reference['path']) == reference
    lookup = {c['name']: c for c in r['cases']}
    for c in read(reference['path'])['cases']: assert c['frameSha256'] == lookup[c['name']]['frameSha256']
    references[name] = {'report': reference, 'reusedByFrameDigest': True}

geometry = read(ROOT / 'parity.json')
hashes(geometry)
files(geometry)
assert len(geometry['cases']) == 823 and geometry['exactIndexAndCandidateBytes']
assert sum(c.get('geometryDeclarationsPreserved', False) for c in geometry['cases']) == 400
manifest = read(ROOT / 'manifest.json')
assert len(manifest['cases']) == 423
assert entry(manifest['base']['path'])['sha256'] == manifest['base']['sha256']
for c in manifest['cases']: assert entry(c['path'])['sha256'] == c['sha256']
definition = manifest['standardDefinitions']
assert entry(definition['path'])['sha256'] == definition['sha256']
assert [definition[k] for k in ['records', 'uniqueNames', 'duplicateName', 'missingName']] == [187, 186, 'upDownArrow', 'upArrow']
regressions['sourceGeometry'] = {'report': entry(ROOT / 'parity.json'), 'batches': 823}
independent = read(ROOT / 'independent.json')
lookup = {c['name']: c for c in geometry['cases']}
assert [len(independent['cases']), independent['geometryDeclarations'], independent['validXsdParts'], independent['negativeXsdCases']] == [400, 5601, 2399, 23]
for c in independent['cases']:
    assert c['sourceSha256'] == lookup['inspect-' + c['name']]['sourceSha256']
    assert c['responseSha256'] == lookup['inspect-' + c['name']]['responseSha256']
    assert c['editSha256'] == lookup['edit-' + c['name']]['pptxSha256']
for c in independent['xsdInputs']: assert entry(c['path'])['sha256'] == c['sha256']
assert [len(independent['coverage'][k]) for k in ['presets', 'commands', 'handles', 'fill']] == [187, 6, 2, 6]
assert independent['intentionalXsdExclusions'] == ['retained-attribute']
contracts = read(ROOT / 'contracts.json')
assert [contracts[k] for k in ['schemas', 'sourceGeometryResponses', 'sourceGeometryEditRequests', 'sourceLineResponses', 'sourceLineEditRequests', 'lineStyleResponses', 'lineColorResponses', 'pathRasterResponses', 'sceneRasterResponses', 'pageRenderResponses']] == [53, 823, 400, 136, 59, 144, 247, 267, 316, 132]
for prefix, extension in [('contracts/generated/', '.schema.json'), ('packages/contracts/src/generated/', '.ts')]:
    changed = [s['path'] for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(extension) and entry(s['path']) != s]
    assert changed == [prefix + 'pptx-source-response' + extension], changed
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'rust-tests.log').read_text(), re.M)
assert len(tests) == 285
logs = ['rust-tests.log', 'clippy.log', 'schema-check.log', 'types-check.log', 'fmt-check.log', 'independent.log', 'regressions.log', 'contracts.log']
for name in logs:
    raw = (ROOT / name).read_text()
    assert 'error:' not in raw and 'FAILED' not in raw and 'Traceback' not in raw, name
for p in ['Cargo.lock', 'pnpm-lock.yaml']: assert entry(p) == next(s for s in prior['sourceFiles'] if s['path'] == p)
author = read('.codex-work/source-lines/author/parity.json')
hashes(author)
files(author)
author_additions = compare_index_cases(author['cases'], prior['auxiliaryAuthorReport']['cases'])
# Component and external numeric/reference evidence remains attached to its
# original sources/artifacts. It is inherited, never reported as rerun here.
legacy = read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
component = legacy['componentReport']
assert read('.codex-work/skia/verification/parity.json') == component
for k, p in [('nativeSha256', '.codex-work/skia/mo-skia-probe'), ('asanSha256', '.codex-work/skia/mo-skia-probe-asan'), ('wasmSha256', artifacts['rasterWasm']['path']), ('glueSha256', artifacts['rasterWasmGlue']['path']), ('adapterSha256', artifacts['rasterAdapter']['path'])]: assert component[k] == entry(p)['sha256']
for build in legacy['componentBuilds'].values():
    assert build['lock'] == read('components/skia/lock.json')
    for c in build['componentSources'] + build['artifacts']: assert entry(c['path']) == c
assert read('.codex-work/line-colors/independent.json') == prior['independentReport']
paths = {s['path'] for s in prior['sourceFiles']}
assert all(Path(p).exists() for p in paths)
for base in ['components', 'crates', 'tools', 'contracts', 'packages', 'fixtures', 'docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs', '.toml', '.json', '.ts', '.mjs', '.py', '.md', '.h', '.cpp', '.bin', '.ttf', '.otf', '.ttc', '.patch', '.txt'] and '__pycache__' not in p.parts: paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']: assert len(Path(p).read_text().splitlines()) <= 2000, p
checks = {'rustTests': len(tests), 'newRustTests': 8, 'nativeWasmLogicalBatches': sum(r['batches'] for r in regressions.values()),
          'previousLogicalBatchesRerun': 3216, 'newGeometryBatches': 823, 'newPptxInputs': 423, 'nativeTextEditCandidates': 400,
          'independentGeometryDeclarations': 5601, 'presetNames': 187, 'pathCommandKinds': 6, 'handleKinds': 2, 'pathFillModes': 6,
          'validXsdParts': 2399, 'negativeXsdCases': 23, 'intentionalXsdExclusions': 1,
          'runtimeSchemas': 53, 'schemasChanged': 1, 'strictClippy': True, 'rustfmt': True, 'typescript': True}
assert checks['nativeWasmLogicalBatches'] == 4039
result = {'format': 'musteroffice.source-geometry-verification/1',
          'scope': 'Native geometry declarations and physical bindings in actual PPTX. No formula execution, preset expansion, shape rendering or target application fidelity acceptance.',
          'previousEvidence': entry(previous), 'checks': checks, 'artifacts': artifacts,
          'artifactByteDeltas': {k: v['byteLength'] - prior['artifacts'][k]['byteLength'] for k, v in artifacts.items()},
          'artifactSizeScope': 'Uncompressed incomplete development artifacts. Not complete kernel, installer, memory or performance acceptance.',
          'environment': {'platform': platform.system() + ' ' + platform.release(), 'architecture': platform.machine(), 'rust': '1.92.0', 'node': '23.5.0', 'python': platform.python_version()},
          'geometryReport': geometry, 'geometryManifest': manifest, 'independentReport': independent, 'contractReport': contracts,
          'regressionReports': regressions, 'pathRasterReport': rasters['path'], 'sceneRasterReport': rasters['scene'], 'pageRenderReport': rasters['page'],
          'reusedGeometryReferences': references, 'auxiliaryAuthorReport': author, 'auxiliaryResponsesAddingOnlyGeometry': author_additions,
          'reusedColorOracle': {'evidence': entry(previous), 'allQueryCasesAndEditCandidatesUnchanged': True},
          'reusedComponentEvidence': {'evidence': entry('docs/reviews/evidence/2026-09-24-miter-clip-verification.json'), 'batches': len(component['cases']), 'referencePixels': component['oraclePixels'], 'artifactDigestsVerified': True, 'rerun': False},
          'rustTestNames': tests, 'validationLogs': [entry(ROOT / p) for p in logs], 'sourceFiles': [entry(p) for p in sorted(paths)],
          'dependencyChanges': {'externalRuntimeVersions': [], 'lockFilesUnchanged': True, 'newDevelopmentDependencies': []},
          'limitations': [
              '187 preset names are recognized, not implemented geometry. Formulas, path coordinates and handles are declarations only.',
              'Official definition verification input contains 187 records but 186 unique names: duplicate upDownArrow and missing upArrow. No correction inferred and no templates shipped in runtime.',
              'Unknown retained properties block future complete evaluation. One intentional unknown-attribute fixture is excluded from positive XSD counts.',
              'Existing source-index responses add only geometry records; all prior fields and native edit candidates remain unchanged.',
              'No new Office/WPS/LibreOffice application run, editing roundtrip, imported page pixel comparison or complete deck rendering.',
              'No browser-worker, resource lifecycle, advanced object/playback, Agent/Musterwork E0-E3, installer or end-to-end performance acceptance.'
          ]}
raw = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
summary = {'checks': checks, 'sourceFiles': len(paths), 'artifactByteDeltas': result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f: f.write(raw)
    summary['evidence'] = entry(OUTPUT)
print(json.dumps(summary, indent=2))
