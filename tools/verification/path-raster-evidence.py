"""Seal the completed path raster milestone; never overwrite historical evidence."""
import hashlib
import json
import platform
import re
from pathlib import Path

root = Path('.codex-work/path-raster')
output = Path('docs/reviews/evidence/2026-09-24-path-raster-verification.json')
assert not output.exists(), 'evidence is immutable'
def read(p):
    return json.loads(Path(p).read_text())
def entry(p):
    p = Path(p)
    b = p.read_bytes()
    return {'path': str(p), 'byteLength': len(b), 'sha256': hashlib.sha256(b).hexdigest()}
previous = 'docs/reviews/evidence/2026-09-24-paragraph-paths-verification.json'
prior = read(previous)
artifacts = {k: entry(v['path']) for k, v in prior['artifacts'].items()}
for key, path in {
    'nativeRasterWorker': 'target/release/mo-raster-worker',
    'rasterWasm': '.codex-work/skia/mo-skia.wasm',
    'rasterWasmGlue': '.codex-work/skia/mo-skia.mjs',
    'rasterAdapter': '.codex-work/raster-component/index.js',
}.items():
    artifacts[key] = entry(path)
for key in ['cppWasm', 'cppWasmGlue', 'typescriptAdapter']:
    assert artifacts[key] == prior['artifacts'][key]

def verify_cases(report):
    for case in report.get('cases', []):
        for key in ['request', 'response', 'bundle', 'font', 'frame', 'pixels']:
            if key+'Path' in case:
                assert entry(case[key+'Path'])['sha256'] == case[key+'Sha256']

def main_hashes(report):
    aliases = {'nativeSha256': 'nativeCli', 'nativeCliSha256': 'nativeCli',
               'nativeWorkerSha256': 'nativeWorker', 'rustWasmSha256': 'rustWasm',
               'wasmSha256': 'rustWasm', 'wasmKernelSha256': 'rustWasm',
               'componentSha256': 'cppWasm', 'componentWasmSha256': 'cppWasm'}
    assert any(k in report for k in ['nativeSha256', 'nativeCliSha256'])
    assert any(k in report for k in ['rustWasmSha256', 'wasmSha256', 'wasmKernelSha256'])
    for key, artifact in aliases.items():
        if key in report:
            assert report[key] == artifacts[artifact]['sha256'], key

regressions = {}
for name in ['document', 'opc', 'export', 'source', 'color', 'font']:
    path = root/(name+'-regression.json')
    current = read(path)
    old = read('.codex-work/paragraph-paths/'+name+'-regression.json')
    main_hashes(current)
    for a, b in zip(current['cases'], old['cases'], strict=True):
        for key in ['name', 'response', 'sha256', 'sourceSha256', 'byteLength', 'error', 'code', 'resultSha256']:
            assert a.get(key) == b.get(key), (name, a.get('name'), key)
    regressions[name] = {'report': entry(path), 'batches': len(current['cases']),
                         'artifactHashesVerified': True, 'semanticResultsUnchanged': True}

historical = {
    'shaping': ('text-shaping', 'textReport'),
    'unicode': ('unicode-text', 'unicodeReport'),
    'cascade': ('font-cascade', 'cascadeReport'),
    'bidi': ('bidi', 'bidiReport'),
    'bidiConformance': ('bidi', 'conformanceReport'),
    'itemizationAndParagraph': ('font-fallback', 'updatedItemizationReport'),
    'mixedFont': ('font-fallback', 'fallbackReport'),
    'lineBreaking': ('line-break', 'lineBreakReport'),
    'fontMetrics': ('font-metrics', 'metricsReport'),
    'lineShaping': ('line-shaping', 'lineShapeReport'),
    'lineGeometry': ('line-geometry', 'lineGeometryReport'),
    'paragraphLayout': ('paragraph-layout', 'paragraphLayoutReport'),
    'fontOutlines': ('font-outlines', 'outlinesReport'),
}
for name, (milestone, key) in historical.items():
    path = prior['regressionReports'][name]['report']['path']
    current = read(path)
    old = read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
    field = 'batches' if name == 'bidiConformance' else 'cases'
    assert current[field] == old[field], name
    main_hashes(current)
    verify_cases(current)
    regressions[name] = {'report': entry(path), 'batches': len(current[field]),
                         'artifactHashesVerified': True, 'semanticResultsUnchanged': True}
scene = read('.codex-work/paragraph-paths/parity.json')
assert scene['cases'] == prior['paragraphPathsReport']['cases']
main_hashes(scene)
verify_cases(scene)
regressions['paragraphPaths'] = {'report': entry('.codex-work/paragraph-paths/parity.json'),
                                'batches': len(scene['cases']), 'artifactHashesVerified': True,
                                'semanticResultsUnchanged': True}
assert sum(r['batches'] for r in regressions.values()) == 1883

report = read(root/'parity.json')
for key, artifact in [('nativeCliSha256', 'nativeCli'), ('nativeWorkerSha256', 'nativeRasterWorker'),
                      ('rustWasmSha256', 'rustWasm'), ('componentSha256', 'rasterWasm'), ('adapterSha256', 'rasterAdapter')]:
    assert report[key] == artifacts[artifact]['sha256']
verify_cases(report)
assert len(report['cases']) == 116
assert sum(c['status'] == 'rendered' for c in report['cases']) == 89
assert sum('paragraphSource' in c for c in report['cases']) == 29
assert len(report['bridgeFaults']) == 9
for c in report['cases']:
    if 'paragraphSource' in c:
        assert c['paragraphSource'] in {p['responsePath'] for p in scene['cases']}
reference = read(root/'reference.json')
assert reference['counts'] == {'cases': 89, 'localCoordinates': 535264,
                               'drawCoordinates': 133546, 'transformedCoordinates': 2108978}
for c in reference['cases']:
    assert c['frameSha256'] == entry(root/(c['name']+'.frame.bin'))['sha256']
component = read('.codex-work/skia/verification/parity.json')
assert len(component['cases']) == 148 and component['oraclePixels'] == 61444
assert component['maxChannelDifference'] == 0 and component['differentBytes'] == 0
for key, artifact in [('wasmSha256', 'rasterWasm'), ('glueSha256', 'rasterWasmGlue'), ('adapterSha256', 'rasterAdapter')]:
    assert component[key] == artifacts[artifact]['sha256']
assert component['nativeSha256'] == entry('.codex-work/skia/mo-skia-probe')['sha256']
assert component['asanSha256'] == entry('.codex-work/skia/mo-skia-probe-asan')['sha256']
for c in component['cases']:
    assert c['requestSha256'] == entry(c['requestPath'])['sha256']
previews = read(root/'previews.json')
assert len(previews) == 7
for p in previews:
    assert {k: p[k] for k in ['path', 'byteLength', 'sha256']} == entry(p['path'])
    assert p['pixelsSha256'] == entry(Path(p['path']).with_suffix('.rgba'))['sha256']
contracts = read(root/'contracts.json')
assert contracts['schemas'] == 42 and contracts['pathRasterResponses'] == 116
tests = (root/'rust-tests.log').read_text()
names = re.findall(r'^test (.+) \.\.\. ok$', tests, re.M)
assert len(names) == 215 and 'FAILED' not in tests
clippy = (root/'clippy.log').read_text()
assert 'Finished' in clippy and 'error:' not in clippy
assert 'error' not in (root/'types-check.log').read_text().lower()
oldmeta = read('.codex-work/paragraph-paths/cargo-metadata.json')
metadata = read(root/'cargo-metadata.json')
oldpackages = {(p['name'], p['version']) for p in oldmeta['packages']}
new_internal = ['mo-raster', 'mo-skia-sys', 'mo-raster-worker']
assert {(p['name'], p['version']) for p in metadata['packages']} == oldpackages | {(p, '0.1.0') for p in new_internal}
oldfeatures = {p['id']: p['features'] for p in oldmeta['resolve']['nodes']}
for p in metadata['resolve']['nodes']:
    if p['id'] in oldfeatures:
        assert p['features'] == oldfeatures[p['id']]
schemas = [s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')]
assert len(schemas) == 40
for s in schemas:
    assert s == entry(s['path'])
native_build = read('.codex-work/skia/native-build.json')
for a in native_build['componentSources']+native_build['artifacts']:
    assert a == entry(a['path'])
assert native_build['target'] == 'native' and native_build['sanitizers'] is False
assert native_build['lock'] == read('components/skia/lock.json')
paths = {s['path'] for s in prior['sourceFiles']}
for base in ['components', 'crates', 'tools', 'contracts', 'packages', 'fixtures', 'docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs', '.toml', '.json', '.ts', '.mjs', '.py', '.md', '.h', '.cpp', '.bin', '.ttf', '.otf', '.ttc', '.patch', '.txt'] and '__pycache__' not in p.parts:
            paths.add(str(p))
sources = [entry(p) for p in sorted(paths)]
for p in paths:
    if Path(p).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
result = {
    'format': 'musteroffice.path-raster-verification/1',
    'scope': 'Exact rational Q32 path compiler, isolated native raster worker and bounded WASM pixel bridge. Actual paragraph and geometric pixels, no complete page Draw IR or target application acceptance.',
    'previousEvidence': entry(previous),
    'independentComponentEvidence': entry('docs/reviews/evidence/2026-09-24-skia-component-verification.json'),
    'environment': {'platform': platform.system()+' '+platform.release(), 'architecture': platform.machine(),
                    'rust': '1.92.0', 'node': '23.5.0', 'python': platform.python_version()},
    'artifacts': artifacts,
    'existingArtifactByteDeltas': {k: v['byteLength']-prior['artifacts'][k]['byteLength'] for k, v in artifacts.items() if k in prior['artifacts']},
    'artifactSizeScope': 'Uncompressed incomplete development artifacts, no full kernel/installer or performance/RSS estimate. Native raster worker already contains its linked Skia code; static archives are build inputs, not additional runtime files.',
    'checks': {'rustTests': 215, 'newRustTests': 10, 'nativeWasmBatches': 1999, 'newPathRasterBatches': 116,
               'oldSemanticBatchesUnchanged': 1883, 'runtimeSchemas': 42, 'existing40SchemasUnchanged': True,
               'exactPixelAndMetadataEquality': True, 'independentCoordinateOracle': True,
               'independentComponentBatches': 148, 'independentComponentOraclePixels': 61444,
               'actualAllocationFailureAndReuseRejection': True, 'fullProcessExitDeadline': True,
               'rustfmt': True, 'strictClippy': True, 'typescript': True},
    'pathRasterReport': report, 'independentCoordinateReport': reference,
    'componentRegressionReport': component, 'nativeComponentBuild': native_build,
    'contractReport': contracts, 'regressionReports': regressions, 'rustTestNames': names,
    'diagnosticPreviews': previews,
    'visualInspection': {'renderer': 'actual Rust-compiled Skia pixels, PNG encoding only',
                         'inputs': [entry(root/('paragraph-'+n+'.png')) for n in ['bidi-brackets', 'cjk-punctuation', 'cubic']],
                         'observation': 'Visible wrapped text and owned cubic contours within the explicit viewport; no Office/WPS comparison or quality gate claimed.'},
    'dependencyChanges': {'newInternalCrates': new_internal, 'newExternalVersions': [], 'featureChanges': []},
    'sourceFiles': sources,
    'limitations': [
        'Translation and rational scale only; no full Draw IR, presentation compiler, affine/group/clip/stroke/brush/image/effect/color glyph or GPU support.',
        'Coordinate bound covers quantized control points only, not pixel coverage, font source approximation, complete ink bounds or target application visuals.',
        'This profile explicitly rejects out-of-range precision or work. Production tiling/culling/cache and the complete presentation limits are not implemented.',
        'WASM component has a 256 MiB heap, not a complete host memory cap. Native RSS/process groups/pools, asynchronous cancellation and browser Worker remain incomplete.',
        'Native is verified only on macOS arm64, with libc++/libSystem. No Windows/Linux/browser execution or package-size acceptance.',
        'Actual WASM component allocation failure and independent Skia abort regression are verified; exhaustive native allocator fault injection is not claimed.',
        'CLI files are development artifacts, not Musterwork Artifact/CAS commits. Full editable advanced objects, playback, Agent interfaces, Office/WPS interoperability and E0-E3 remain incomplete.',
    ],
}
raw = json.dumps(result, ensure_ascii=False, indent=2)+'\n'
assert '/Users/' not in raw
output.write_text(raw)
print(json.dumps({'evidence': entry(output), 'sourceFiles': len(sources), 'artifacts': {k: v['byteLength'] for k, v in artifacts.items()}}, indent=2))
