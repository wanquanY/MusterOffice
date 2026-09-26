"""Seal verified author placement, real raster/export probes and external gaps."""
import hashlib
import json
import platform
import re
from pathlib import Path

root=Path('.codex-work/page-placement')
output=Path('docs/reviews/evidence/2026-09-24-page-placement-verification.json')
assert not output.exists(),'immutable evidence already exists'
def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
previous='docs/reviews/evidence/2026-09-24-scene-raster-reachability-verification.json';prior=read(previous)
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
unchanged=['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']
for k in unchanged:assert artifacts[k]==prior['artifacts'][k]
def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm',
             'componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in r:assert r[k]==artifacts[a]['sha256'],k
def cases(r):
    for c in r.get('cases',[]):
        for k in ['request','response','font','bundle','frame','pixels','scene','pptx']:
            if k+'Path' in c:assert entry(c[k+'Path'])['sha256']==c[k+'Sha256']
regressions={}
for name in ['document','opc','export','source','color','font']:
    old_entry=prior['regressionReports'][name]['report'];assert old_entry==entry(old_entry['path'])
    old=read(old_entry['path']);p=root/(name+'-regression.json');r=read(p);hashes(r)
    for a,b in zip(r['cases'],old['cases'],strict=True):
        for k in ['name','response','sha256','sourceSha256','byteLength','error','code','resultSha256']:assert a.get(k)==b.get(k),(name,k)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
historical={
 'shaping':('text-shaping','textReport'),'unicode':('unicode-text','unicodeReport'),
 'cascade':('font-cascade','cascadeReport'),'bidi':('bidi','bidiReport'),
 'bidiConformance':('bidi','conformanceReport'),'itemizationAndParagraph':('font-fallback','updatedItemizationReport'),
 'mixedFont':('font-fallback','fallbackReport'),'lineBreaking':('line-break','lineBreakReport'),
 'fontMetrics':('font-metrics','metricsReport'),'lineShaping':('line-shaping','lineShapeReport'),
 'lineGeometry':('line-geometry','lineGeometryReport'),'paragraphLayout':('paragraph-layout','paragraphLayoutReport'),
 'fontOutlines':('font-outlines','outlinesReport'),'paragraphPaths':('paragraph-paths','paragraphPathsReport'),
}
for name,(milestone,key) in historical.items():
    p=prior['regressionReports'][name]['report']['path'];r=read(p)
    old=read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
    field='batches' if name=='bidiConformance' else 'cases';assert r[field]==old[field],name
    hashes(r);cases(r);regressions[name]={'report':entry(p),'batches':len(r[field]),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
for name,p,old in [('pathRaster','.codex-work/path-raster/parity.json',read('docs/reviews/evidence/2026-09-24-path-raster-verification.json')['pathRasterReport']),('sceneRaster','.codex-work/scene-raster/parity.json',prior['sceneRasterReport'])]:
    r=read(p);assert r['cases']==old['cases'];hashes(r,True);cases(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==2180
report=read(root/'parity.json');hashes(report);cases(report)
assert len(report['cases'])==64 and sum(c['status']=='evaluated' for c in report['cases'])==52
reference=read(root/'reference.json');assert reference['counts']=={'documents':52,'objects':8807,'coefficients':52842,'sourceControlPoints':44042}
for c in reference['cases']:assert entry(root/(c['name']+'.response.json'))['sha256']==c['responseSha256']
render=read(root/'render.json');hashes(render,True);cases(render);assert len(render['cases'])==4 and render['nativeWasmPixelsAndPptxEqual']
for c in render['cases']:
    assert entry(root/(c['name']+'.response.json'))['sha256']==c['placementResponseSha256']
    assert int(c['authorCoordinateErrorBound'])+int(c['sceneCoordinateErrorBound'])<=2**24
external=read(root/'external.json');assert len(external['cases'])==4 and not external['allWithinObservationTolerance']
for c in external['cases']:
    for k in ['source','pdf','kernelPreview','externalPreview']:assert c[k]==entry(c[k]['path'])
    assert not c['withinObservationTolerance'] and c['maximumVertexToBoundaryDistancePt']>28
contracts=read(root/'contracts.json');assert contracts['schemas']==46 and contracts['pagePlacementResponses']==64
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')];assert len(schemas)==44
for s in schemas:assert s==entry(s['path'])
names=re.findall(r'^test (.+) \.\.\. ok$',(root/'rust-tests.log').read_text(),re.M)
assert len(names)==232 and 'FAILED' not in (root/'rust-tests.log').read_text()
assert 'Finished' in (root/'clippy.log').read_text() and 'error:' not in (root/'clippy.log').read_text()
assert 'error' not in (root/'types-check.log').read_text().lower()
metadata=read(root/'cargo-metadata.json');oldmeta=read('.codex-work/scene-raster/cargo-metadata.json')
oldpackages={(p['name'],p['version']) for p in oldmeta['packages']}
assert {(p['name'],p['version']) for p in metadata['packages']}==oldpackages|{('mo-presentation-compile','0.1.0')}
oldfeatures={p['id']:p['features'] for p in oldmeta['resolve']['nodes']}
assert all(p['features']==oldfeatures[p['id']] for p in metadata['resolve']['nodes'] if p['id'] in oldfeatures)
standard=entry('.codex-work/ecma376/part1.pdf')
assert standard['sha256']==read('docs/reviews/evidence/2026-09-24-pptx-colors-verification.json')['standard']['pdfSha256']
paths={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
paths.update(['README.md','docs/README.md','docs/design/implementation/algorithms-and-components.md'])
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
result={
 'format':'musteroffice.page-placement-verification/1',
 'scope':'Author-model page coordinate stage and four actual rectangle raster/PPTX probes. Complete page painting and Office/WPS compatibility are not implemented or accepted.',
 'previousEvidence':entry(previous),'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development binaries; no complete kernel or Musterwork installer estimate.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'standardProfile':{'status':'draft, target-application behavior unresolved','sourceUrl':'https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip','pdf':standard,'sections':['L.4.7.3','L.4.7.4','L.4.7.5','L.4.7.6'],'sourceRole':'informative annex, not a substitute for actual Office/WPS interoperability'},
 'checks':{'rustTests':232,'newRustTests':7,'nativeWasmBatches':2244,'newPlacementBatches':64,'oldSemanticBatchesUnchanged':2180,'runtimeSchemas':46,'existing44SchemasUnchanged':True,'independentHighPrecisionReference':True,'actualRasterAndEditablePptxProbes':4,'externalObservations':4,'externalWithinObservationTolerance':0,'strictClippy':True,'rustfmt':True,'typescript':True},
 'pagePlacementReport':report,'independentReference':reference,'diagnosticRenderReport':render,'externalObservation':external,'contractReport':contracts,
 'regressionReports':regressions,'rustTestNames':names,
 'visualInspection':{'inputs':[entry(root/(name+'.png')) for name in ['diagnostic-0-none','diagnostic-1800000-h']]+[entry(root/'libreoffice'/(name+'.png')) for name in ['diagnostic-0-none','diagnostic-1800000-h']],
                     'observation':'Actual kernel and external PDF pixels show differing rectangle aspect/orientation; differences retained as unresolved interoperability evidence.'},
 'componentEvidenceReuse':{'reexecutedIndependentSuites':False,'unchangedArtifacts':{k:artifacts[k] for k in unchanged},'previousVerifiedEvidence':entry(previous)},
 'dependencyChanges':{'newInternalCrates':['mo-presentation-compile'],'externalVersionChanges':[],'existingFeatureChanges':[],'developmentOnlyPdfTools':'Bundled PyMuPDF and Pillow, not linked to runtime'},
 'sourceFiles':[entry(p) for p in sorted(paths)],
 'limitations':[
  'Current author model only: zero-origin positive path/group viewport, no arbitrary child offset or zero child extents from imported OOXML.',
  'Placement records cover existing object kinds, not shape geometry evaluation, text layout, image decoding, styles, placeholder resolution, clipping or complete page painting.',
  'Q96 interval estimates are narrowed to Q32 plus explicit coefficient/center uncertainty. A production page painter must propagate this author error in addition to scene/raster error.',
  'The adopted informative-annex transform profile differs visibly from all four LibreOffice probes. Office/WPS target behavior must be measured before finalizing compatibility; no interoperability gate is closed.',
  'Cancellation covers placement loops and angle evaluation; existing whole-model validation and canonical digest have pre/post checks, not fine-grained interruption.',
  'Native macOS arm64 and Node WASM only; browser Worker, other OS targets, production resource accounting, incremental caches and complete performance/packaging evidence remain unfinished.',
  'Full native advanced objects, media/playback, public OperationService/Agent surfaces, Musterwork adapter and E0-E3 replacement remain incomplete.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
output.write_text(raw);print(json.dumps({'evidence':entry(output),'sourceFiles':len(paths),'checks':result['checks']},indent=2))
