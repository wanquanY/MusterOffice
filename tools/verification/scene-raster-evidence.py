"""Seal affine scene implementation evidence after the current artifact checks."""
import hashlib
import json
import platform
import re
from pathlib import Path

root=Path('.codex-work/scene-raster')
output=Path('docs/reviews/evidence/2026-09-24-scene-raster-reachability-verification.json')
assert not output.exists(), 'immutable evidence already exists'
def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes()
    return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
previous='docs/reviews/evidence/2026-09-24-path-raster-verification.json';prior=read(previous)
initial_scene='docs/reviews/evidence/2026-09-24-scene-raster-verification.json';initial=read(initial_scene)
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
unchanged_components=['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']
for key in unchanged_components:assert artifacts[key]==prior['artifacts'][key]
def cases(report):
    for c in report.get('cases',[]):
        for k in ['request','response','font','bundle','frame','pixels']:
            if k+'Path' in c:assert entry(c[k+'Path'])['sha256']==c[k+'Sha256']
def hashes(report,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm',
             'componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in report for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in report for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in report:assert report[k]==artifacts[a]['sha256'],k
regressions={}
for name in ['document','opc','export','source','color','font']:
    path=root/(name+'-regression.json');r=read(path);old=read('.codex-work/path-raster/'+name+'-regression.json');hashes(r)
    for a,b in zip(r['cases'],old['cases'],strict=True):
        for k in ['name','response','sha256','sourceSha256','byteLength','error','code','resultSha256']:assert a.get(k)==b.get(k),(name,k)
    regressions[name]={'report':entry(path),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
historical={
 'shaping':('text-shaping','textReport'), 'unicode':('unicode-text','unicodeReport'),
 'cascade':('font-cascade','cascadeReport'), 'bidi':('bidi','bidiReport'),
 'bidiConformance':('bidi','conformanceReport'), 'itemizationAndParagraph':('font-fallback','updatedItemizationReport'),
 'mixedFont':('font-fallback','fallbackReport'), 'lineBreaking':('line-break','lineBreakReport'),
 'fontMetrics':('font-metrics','metricsReport'), 'lineShaping':('line-shaping','lineShapeReport'),
 'lineGeometry':('line-geometry','lineGeometryReport'), 'paragraphLayout':('paragraph-layout','paragraphLayoutReport'),
 'fontOutlines':('font-outlines','outlinesReport'), 'paragraphPaths':('paragraph-paths','paragraphPathsReport'),
}
for name,(milestone,key) in historical.items():
    path=prior['regressionReports'][name]['report']['path'];r=read(path)
    old=read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
    field='batches' if name=='bidiConformance' else 'cases'
    assert r[field]==old[field],name;hashes(r);cases(r)
    regressions[name]={'report':entry(path),'batches':len(r[field]),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
r=read('.codex-work/path-raster/parity.json');assert r['cases']==prior['pathRasterReport']['cases'];hashes(r,True);cases(r)
regressions['pathRaster']={'report':entry('.codex-work/path-raster/parity.json'),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==1999
report=read(root/'parity.json');hashes(report,True);cases(report)
assert len(report['cases'])==181 and sum(c['status']=='rendered' for c in report['cases'])==165
current={c['name']:c for c in report['cases']}
old_scene={c['name']:c for c in initial['sceneRasterReport']['cases']}
assert len(old_scene)==179 and all(current[k]==v for k,v in old_scene.items())
reachability_cases=['unused-transform-range','empty-instance-transform-range']
assert set(current)-set(old_scene)==set(reachability_cases)
for name in reachability_cases:
    assert current[name]['status']=='rendered' and current[name]['work']['loweringAttempts']==1
assert sum('paragraphSource' in c for c in report['cases'])==29
fallbacks=[c['name'] for c in report['cases'] if c.get('work',{}).get('loweringAttempts')==2]
assert fallbacks==['minimum-viewport-origin','matrix-range-fallback','matrix-precision-fallback']
reference=read(root/'reference.json')
assert reference['counts']=={'scenes':165,'instances':67050,'controlCoordinates':2249746,'exactTransformApplications':5259471}
for c in reference['cases']:assert c['frameSha256']==entry(root/(c['name']+'.frame.bin'))['sha256']
contracts=read(root/'contracts.json');assert contracts['schemas']==44 and contracts['sceneRasterResponses']==181
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')];assert len(schemas)==42
for s in schemas:assert s==entry(s['path'])
tests=(root/'rust-tests.log').read_text();names=re.findall(r'^test (.+) \.\.\. ok$',tests,re.M)
assert len(names)==225 and 'FAILED' not in tests
clippy=(root/'clippy.log').read_text();assert 'Finished' in clippy and 'error:' not in clippy
assert 'error' not in (root/'types-check.log').read_text().lower()
previews=read(root/'previews.json');assert len(previews)==7
for p in previews:
    assert {k:p[k] for k in ['path','byteLength','sha256']}==entry(p['path'])
    assert p['pixelsSha256']==entry(Path(p['path']).with_suffix('.rgba'))['sha256']
benchmark=read(root/'benchmark.json')
assert benchmark['rustWasmSha256']==artifacts['rustWasm']['sha256'] and benchmark['componentSha256']==artifacts['rasterWasm']['sha256']
assert benchmark['sourceSha256']==entry(benchmark['sourcePath'])['sha256']
assert benchmark['instances']==1000 and benchmark['outputs']['shared']['sha256']==benchmark['outputs']['duplicated']['sha256']
for kind,count in [('shared',1),('duplicated',1000)]:
    assert len(benchmark['samplesMilliseconds'][kind])==12
    s=sorted(benchmark['samplesMilliseconds'][kind]);assert benchmark['medianMilliseconds'][kind]==(s[5]+s[6])/2
    assert benchmark['outputs'][kind]['metadata']['info']['work']['compiledPaths']==count
    assert benchmark['outputs'][kind]['sha256']==benchmark['outputs'][kind]['metadata']['info']['raster']['sha256']
metadata=read(root/'cargo-metadata.json');oldmeta=read('.codex-work/path-raster/cargo-metadata.json')
oldpackages={(p['name'],p['version']) for p in oldmeta['packages']}
added={('mo-render','0.1.0'),('num-bigint','0.4.8'),('num-integer','0.1.47')}
assert {(p['name'],p['version']) for p in metadata['packages']}==oldpackages|added
oldfeatures={p['id']:p['features'] for p in oldmeta['resolve']['nodes']}
features=[{'id':p['id'],'before':oldfeatures[p['id']],'after':p['features']} for p in metadata['resolve']['nodes'] if p['id'] in oldfeatures and p['features']!=oldfeatures[p['id']]]
assert len(features)==1 and features[0]['id'].endswith('num-traits@0.2.19') and features[0]['before']==['std'] and features[0]['after']==['i128','std']
numeric=read('components/rust-numeric/component.json')
for p in numeric['packages']:
    for f in p['licenseFiles']:assert f==entry(f['path'])
    assert any(m['name']==p['name'] and m['version']==p['version'] and m['license']==p['declaredLicense'] for m in metadata['packages'])
component=read('.codex-work/skia/verification/parity.json')
assert component==prior['componentRegressionReport']
assert component['nativeSha256']==entry('.codex-work/skia/mo-skia-probe')['sha256']
assert component['asanSha256']==entry('.codex-work/skia/mo-skia-probe-asan')['sha256']
paths={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
sources=[entry(p) for p in sorted(paths)]
result={
 'format':'musteroffice.scene-raster-verification/1',
 'scope':'Evaluated shared paths, affine transform forest, bounded exact arithmetic and certified CPU lowering. No complete presentation-to-Draw-IR compiler, target-app visual acceptance or product replacement.',
 'previousEvidence':entry(previous),'initialSceneEvidence':entry(initial_scene),'artifacts':artifacts,
 'reachabilityCorrection':{'onlyAncestorsOfNonemptyInstancesAreComposed':True,'allReferencesAndDepthsRemainValidated':True,
                           'previousSceneBatchesUnchanged':179,'newCases':reachability_cases},
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts. No full kernel, font/media distribution or Musterwork installer estimate.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'checks':{'rustTests':225,'newRustTests':10,'nativeWasmBatches':2180,'newSceneBatches':181,'oldSemanticBatchesUnchanged':1999,
           'runtimeSchemas':44,'existing42SchemasUnchanged':True,'exactPixelAndMetadataEquality':True,
           'independentUnroundedAffineReference':True,'coordinateCount':2249746,'actualAllocationFailureAndReuseRejection':True,
           'originalChainRangeAndPrecisionFallback':True,'rootViewportCancellationBeforeNarrowing':True,
           'rustfmt':True,'strictClippy':True,'typescript':True},
 'sceneRasterReport':report,'independentCoordinateReport':reference,'contractReport':contracts,
 'localSharingBenchmark':benchmark,'regressionReports':regressions,'rustTestNames':names,
 'diagnosticPreviews':previews,
 'visualInspection':{'renderer':'actual compiled scene pixels, PNG encoding only','inputs':[entry(root/('rotate-paragraph-'+n+'.png')) for n in ['cjk-punctuation','cubic']],
                     'observation':'Whole-paragraph quarter-turn and owned cubic contours visible within the explicit viewport; no target application comparison.'},
 'componentEvidenceReuse':{'reexecutedIndependentSuites':False,'unchangedArtifacts':{k:artifacts[k] for k in unchanged_components},
                           'priorVerifiedComponentEvidence':prior['independentComponentEvidence'],'componentReportSha256':entry('.codex-work/skia/verification/parity.json')['sha256']},
 'dependencyChanges':{'newInternalCrates':['mo-render'],'newExternalVersions':[['num-bigint','0.4.8'],['num-integer','0.1.47']],
                      'previousDevelopmentDependencyNowInRuntime':['num-traits','0.2.19'],'workspaceFeatureChanges':features,
                      'runtimeFeatureTree':entry(root/'runtime-features.txt'),'numericComponent':numeric},
 'sourceFiles':sources,
 'limitations':[
  'Evaluated path/affine subset only: no author object compiler, full Draw IR, strokes, brushes, images, clip, opacity groups, effects or GPU.',
  'Coordinates are fixed i128 Q32; intermediate non-root spaces still have this numeric range. Root viewport cancellation occurs before narrowing. Fixed source matrix quantization is authoritative input, not an Office transform policy.',
  'Shared matrix lowering is verified against the original chain. Numeric/precision failure retries the original chain before any backend call, still under the same tolerance and output budgets. Two attempts are the maximum.',
  'Error bounds certify control coordinates, not complete raster coverage, geometric ink, target application visuals or total font approximation.',
  'Limits include 8192 transform nodes, depth 64 and 4194304 planned point-node operations per attempt. Input/output/path budgets are inherited from the bounded raster profile, not the final whole-presentation limits.',
  'The local benchmark includes JSON/CPU raster/copy with precomputed outlines, warm instances and no forced GC. It excludes layout, fonts, startup, file publication and peak RSS. No full product performance conclusion.',
  'Native macOS arm64 and Node WASM only; no Windows/Linux/browser evidence. Worker pools, GPU/cache eviction, asynchronous cancellation and complete RSS/process-group policy remain unfinished.',
  'Native editable advanced objects, complete playback, Agent OperationService/MCP/Skill/Plugin, Office/WPS interoperability and Musterwork E0-E3 replacement remain incomplete.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
output.write_text(raw);print(json.dumps({'evidence':entry(output),'sourceFiles':len(sources),'artifacts':{k:v['byteLength'] for k,v in artifacts.items()}},indent=2))
