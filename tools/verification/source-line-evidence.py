"""Verify source line declarations, author absence and unchanged prior semantics.

Default validates current artifacts. --seal exclusively creates immutable evidence.
"""
import copy
import hashlib
import json
import platform
import re
import sys
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/source-lines')
OUTPUT = Path('docs/reviews/evidence/2026-09-24-source-lines-verification.json')
def read(p): return json.loads(Path(p).read_text())
def sha(b): return hashlib.sha256(b).hexdigest()
def entry(p):
    p = Path(p); b = p.read_bytes()
    return {'path': str(p), 'byteLength': len(b), 'sha256': sha(b)}
def canonical(v): return json.dumps(v, ensure_ascii=False, separators=(',', ':')).encode()

previous = 'docs/reviews/evidence/2026-09-24-miter-clip-verification.json'
assert entry(previous)['sha256'] == '5080e0b25bd0fd26510c2e32eda9efe6b88204246427c02c20c8c2cb43f1188f'
prior = read(previous)
artifacts = {k: entry(v['path']) for k, v in prior['artifacts'].items()}
for key in ['nativeWorker', 'cppWasm', 'cppWasmGlue', 'typescriptAdapter',
            'rustWasmGlue', 'rasterWasm', 'rasterWasmGlue', 'rasterAdapter']:
    assert artifacts[key] == prior['artifacts'][key], key

def hashes(r, raster=False):
    aliases = {'nativeSha256': 'nativeCli', 'nativeCliSha256': 'nativeCli',
               'nativeWorkerSha256': 'nativeRasterWorker' if raster else 'nativeWorker',
               'wasmSha256': 'rustWasm', 'rustWasmSha256': 'rustWasm', 'wasmKernelSha256': 'rustWasm',
               'componentSha256': 'rasterWasm' if raster else 'cppWasm',
               'componentWasmSha256': 'cppWasm', 'adapterSha256': 'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256', 'nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256', 'rustWasmSha256', 'wasmKernelSha256'])
    for k, a in aliases.items():
        if k in r: assert r[k] == artifacts[a]['sha256'], k

def case_files(r):
    for c in r.get('cases', []):
        for k in ['request', 'canonicalRequest', 'response', 'plan', 'source', 'font',
                  'bundle', 'frame', 'pixels', 'scene', 'pptx', 'export']:
            if k + 'Path' in c:
                assert entry(c[k + 'Path'])['sha256'] == c[k + 'Sha256'], (c['name'], k)

def old_source(response):
    """Only the added declaration fields and exact updated theme notice may differ."""
    v = copy.deepcopy(response)
    index = v if 'surfaces' in v and 'themes' in v else v.get('index', {})
    for s in index.get('surfaces', {}).values():
        for o in s['objects']:
            o.pop('line', None); o.pop('lineReference', None)
    for t in index.get('themes', {}).values():
        if t.get('formatScheme'):
            for item in t['formatScheme']['lines']: item.pop('line', None)
        t['notices'] = [
            'format style entries source-bound; fill/line/effect semantics unresolved'
            if n == 'line declarations parsed; style inheritance and other format families unresolved' else n
            for n in t['notices']]
    return v

regressions = {}
for name in ['document', 'opc', 'source', 'color', 'font', 'export']:
    p = ROOT / (name + '-regression.json'); r = read(p)
    old_entry = prior['regressionReports'][name]['report']; assert entry(old_entry['path']) == old_entry
    old = read(old_entry['path']); cases = copy.deepcopy(r['cases'])
    if name == 'source':
        for c in cases:
            if 'response' in c: c['response'] = old_source(c['response'])
        for c in r['cases']:
            if 'source' in c: assert entry(c['source'])['sha256'] == c['sourceSha256']
        for c in r['outputs']: assert entry(c['output'])['sha256'] == c['sha256']
    assert cases == old['cases'], name
    hashes(r); case_files(r)
    regressions[name] = {'report': entry(p), 'batches': len(cases),
                         'previousSemanticsPreserved': True, 'declarationsAdded': name == 'source'}

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
    p = prior['regressionReports'][name]['report']['path']; r = read(p)
    old = read('docs/reviews/evidence/2026-09-24-' + milestone + '-verification.json')[key]
    field = 'batches' if name == 'bidiConformance' else 'cases'
    assert r[field] == old[field], name
    hashes(r); case_files(r)
    regressions[name] = {'report': entry(p), 'batches': len(r[field]), 'semanticResultsUnchanged': True}
angles = read('docs/reviews/evidence/2026-09-24-static-rotation-export-verification.json')
for name, key in [('placement', 'pagePlacementReport'), ('groups', 'groupPlacementReport'),
                  ('angles', 'angleExportReport'), ('angleSource', 'angleSourceReport')]:
    p = prior['regressionReports'][name]['report']['path']; r = read(p)
    cases = copy.deepcopy(r['cases'])
    if name in ['angles', 'angleSource']:
        for c in cases: c['responseSha256'] = sha(canonical(old_source(read(c['responsePath']))))
    assert cases == angles[key]['cases'], name
    hashes(r, name in ['placement', 'groups']); case_files(r)
    regressions[name] = {'report': entry(p), 'batches': len(cases), 'previousSemanticsPreserved': True,
                         'declarationsAdded': name in ['angles', 'angleSource']}

rasters = {}; references = {}
for name in ['path', 'scene', 'page']:
    p = Path('.codex-work') / ('page-render' if name == 'page' else name + '-raster') / 'parity.json'
    r = read(p); old = prior[name + ('RenderReport' if name == 'page' else 'RasterReport')]
    hashes(r, True); case_files(r); assert r['exactPixelAndMetadataEquality']
    lookup = {c['name']: c for c in r['cases']}
    assert all(lookup[c['name']] == c for c in old['cases']), name
    extra = set(lookup) - {c['name'] for c in old['cases']}
    assert extra == ({'stroke-author-unresolved-miter-limit'} if name == 'page' else set()), extra
    if name == 'page': assert lookup[next(iter(extra))]['status'] == 'MAPPING_NOT_IMPLEMENTED'
    rasters[name] = r
    regressions[name + 'Raster'] = {'report': entry(p), 'batches': len(r['cases']),
                                    'previousCasesUnchanged': len(old['cases'])}
    reference = read(p.with_name('reference.json'))
    assert reference == prior['parameterAndGeometryReferences'][name]
    for c in reference['cases']: assert c['frameSha256'] == lookup[c['name']]['frameSha256']
    references[name] = {'report': entry(p.with_name('reference.json')), 'reusedByFrameDigest': True}

lines = read(ROOT / 'parity.json'); hashes(lines); case_files(lines)
assert lines['exactIndexAndCandidateBytes'] and len(lines['cases']) == 136 and len(lines['outputs']) == 59
manifest = read(ROOT / 'manifest.json')
assert len(manifest['cases']) == 77
assert entry(manifest['base']['path'])['sha256'] == manifest['base']['sha256']
for c in manifest['cases']: assert entry(c['path'])['sha256'] == c['sha256']
regressions['sourceLines'] = {'report': entry(ROOT / 'parity.json'), 'batches': len(lines['cases'])}
authors = read(ROOT / 'author/parity.json'); hashes(authors); case_files(authors)
assert authors['bundleSha256'] == entry('fixtures/presentations/native-export/resources.bin')['sha256']
assert [c['limit'] for c in authors['cases']] == [None, '0', '400000']
for c in authors['cases']:
    for field, schema in [('request', 'pptx-export-request'), ('response', 'pptx-source-response')]:
        Draft202012Validator(read('contracts/generated/' + schema + '.schema.json')).validate(read(c[field + 'Path']))
independent = read(ROOT / 'independent.json')
assert [independent[k] for k in ['lineDeclarations', 'lineReferences', 'validXsdParts', 'negativeXsdCases']] == [1175, 10, 471, 18]
assert len(independent['cases']) == 59 and all(c['lineXmlAndOtherPartsPreserved'] for c in independent['cases'])
for c in independent['xsdInputs']: assert entry(c['path'])['sha256'] == c['sha256']
for a, c in zip(independent['authorFiles'], authors['cases'], strict=True):
    assert a['pptxSha256'] == c['pptxSha256'] and a['verification']['requestSha256'] == c['requestSha256']
    assert a['verification']['result'] == 'passed' and not a['verification']['differences']
themes = read(ROOT / 'themes-independent.json'); assert themes['passed'] == 42
source = read(ROOT / 'source-regression.json'); source_lookup = {c['name']: c for c in source['cases']}
for c in themes['cases']: assert c['sourceSha256'] == source_lookup[c['name']]['sourceSha256']

contracts = read(ROOT / 'contracts.json')
assert [contracts[k] for k in ['schemas', 'sourceLineResponses', 'sourceLineEditRequests',
                              'pathRasterResponses', 'sceneRasterResponses', 'pageRenderResponses', 'pageCompileResponses']] == [49, 136, 59, 267, 316, 132, 132]
schemas = [s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')]
changed = [s['path'] for s in schemas if entry(s['path']) != s]
assert set(changed) == {'contracts/generated/' + s + '.schema.json' for s in [
    'document', 'kernel-request', 'kernel-response', 'page-placement-request', 'page-render-request',
    'pptx-export-request', 'pptx-source-response', 'transaction']}
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'rust-tests.log').read_text(), re.M)
assert len(tests) == 261
logs = ['rust-tests.log', 'clippy.log', 'schema-check.log', 'types-check.log', 'fmt-check.log']
for name in logs:
    raw = (ROOT / name).read_text(); assert 'error:' not in raw and 'FAILED' not in raw, name
for p in ['Cargo.lock', 'pnpm-lock.yaml']:
    assert entry(p) == next(s for s in prior['sourceFiles'] if s['path'] == p)

component = prior['componentReport']
assert read('.codex-work/skia/verification/parity.json') == component
for k, p in [('nativeSha256', '.codex-work/skia/mo-skia-probe'), ('asanSha256', '.codex-work/skia/mo-skia-probe-asan'),
             ('wasmSha256', artifacts['rasterWasm']['path']), ('glueSha256', artifacts['rasterWasmGlue']['path']),
             ('adapterSha256', artifacts['rasterAdapter']['path'])]: assert component[k] == entry(p)['sha256']
for build in prior['componentBuilds'].values():
    assert build['lock'] == read('components/skia/lock.json')
    for c in build['componentSources'] + build['artifacts']: assert entry(c['path']) == c

moves = {'crates/mo-pptx/src/source/theme/values.rs': 'crates/mo-pptx/src/source/drawingml/color.rs',
         'crates/mo-pptx/src/source/theme/names.rs': 'crates/mo-pptx/src/source/drawingml/names.rs'}
paths = {s['path'] for s in prior['sourceFiles']}
assert {p for p in paths if not Path(p).exists()} == set(moves)
paths -= set(moves)
for base in ['components', 'crates', 'tools', 'contracts', 'packages', 'fixtures', 'docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs', '.toml', '.json', '.ts', '.mjs', '.py', '.md', '.h', '.cpp', '.bin', '.ttf', '.otf', '.ttc', '.patch', '.txt'] and '__pycache__' not in p.parts:
            paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
checks = {'rustTests': len(tests), 'newRustTests': 8, 'nativeWasmLogicalBatches': sum(r['batches'] for r in regressions.values()),
          'previousLogicalBatchesPreserved': 2688, 'newSourceLineBatches': 136, 'newPageRejections': 1,
          'sourceFiles': 77, 'successfulTextEdits': 59, 'independentLineDeclarations': 1175,
          'independentLineReferences': 10, 'validXsdParts': 471, 'negativeXsdCases': 18,
          'auxiliaryAuthorFiles': 3, 'auxiliaryAuthorXsdParts': sum(len(c['verification']['schemasChecked']) for c in independent['authorFiles']),
          'themeRegressionInputs': 42, 'runtimeSchemas': 49, 'changedSchemas': 8,
          'strictClippy': True, 'rustfmt': True, 'typescript': True}
assert checks['nativeWasmLogicalBatches'] == 2825
result = {'format': 'musteroffice.source-lines-verification/1',
    'scope': 'Native line declarations and physical source bindings, text-edit preservation, absent author miter limits. No effective line inheritance, imported rendering or target-app fidelity acceptance.',
    'previousEvidence': entry(previous), 'checks': checks, 'artifacts': artifacts,
    'artifactByteDeltas': {k: v['byteLength'] - prior['artifacts'][k]['byteLength'] for k, v in artifacts.items()},
    'artifactSizeScope': 'Uncompressed incomplete development artifacts; not installer size, complete kernel, performance or RSS acceptance. Raster worker delta is a linked artifact observation, not an optimization claim.',
    'environment': {'platform': platform.system() + ' ' + platform.release(), 'architecture': platform.machine(), 'rust': '1.92.0', 'node': '23.5.0', 'python': platform.python_version()},
    'sourceLineReport': lines, 'sourceLineManifest': manifest, 'authorReport': authors,
    'independentReport': independent, 'themeIndependentReport': themes,
    'pathRasterReport': rasters['path'], 'sceneRasterReport': rasters['scene'], 'pageRenderReport': rasters['page'],
    'reusedGeometryReferences': references,
    'reusedComponentEvidence': {'evidence': entry(previous), 'batches': len(component['cases']),
                              'referencePixels': component['oraclePixels'], 'artifactDigestsVerified': True, 'rerun': False},
    'sourceResponseTransition': {'addedFields': ['SourceObject.line', 'SourceObject.lineReference', 'SourceStyleEntry.line'],
                                'themeNoticeUpdated': True, 'previousFieldsAndOutputsPreserved': True},
    'contractReport': contracts, 'changedSchemas': changed, 'regressionReports': regressions,
    'rustTestNames': tests, 'validationLogs': [entry(ROOT / p) for p in logs],
    'sourceModuleMoves': moves, 'sourceFiles': [entry(p) for p in sorted(paths)],
    'dependencyChanges': {'externalRuntimeVersions': [], 'lockFilesUnchanged': True, 'newDevelopmentDependencies': []},
    'limitations': [
        'Line references and properties are declarations, not effective inherited styles. No default cap/join/limit/width is guessed.',
        'Gradient/pattern fills and extensions retain source bindings without claiming modeled semantics. One intentional unknown attribute part is excluded from positive XSD counts.',
        'Missing author miter limit exports as a:miter without lim and returns an unresolved rendering diagnostic. Explicit values retain the prior mapping.',
        'No Office/WPS/LibreOffice run or edit roundtrip performed against this milestone. Prior compatibility discrepancies remain open.',
        'Native macOS arm64 and Node WASM only; production isolation/pools/complete budgets and browser Workers remain incomplete.',
        'Full styles, page text/images, advanced editable objects, playback, Agent integration and Musterwork E0-E3 remain incomplete.',
    ]}
raw = json.dumps(result, ensure_ascii=False, indent=2) + '\n'; assert '/Users/' not in raw
summary = {'checks': checks, 'sourceFiles': len(paths), 'artifactByteDeltas': result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f: f.write(raw)
    summary['evidence'] = entry(OUTPUT)
print(json.dumps(summary, indent=2))
