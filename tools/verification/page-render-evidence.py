"""Seal author page implementation and exact-build regression/interop evidence."""
import hashlib,json,platform,re
from pathlib import Path
ROOT=Path('.codex-work/page-render')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-page-render-verification.json')
assert not OUTPUT.exists(),'immutable evidence already exists'
def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
previous='docs/reviews/evidence/2026-09-24-static-rotation-export-verification.json';prior=read(previous)
assert entry(previous)['sha256']=='cb31ade7d5960371c733489cedfac3e1fbfd19d11ec10b42a8b2f13e8c51dc03'
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[k]==prior['artifacts'][k],k

def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256','nativeCliSha256']) and any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in r:assert r[k]==artifacts[a]['sha256'],k

def cases(r):
    for c in r.get('cases',[]):
        for k in ['request','canonicalRequest','response','plan','source','font','bundle','frame','pixels','scene','pptx']:
            if k+'Path' in c:assert entry(c[k+'Path'])['sha256']==c[k+'Sha256'],(c['name'],k)

regressions={}
for name in ['document','opc','source','color','font']:
    old_entry=prior['regressionReports'][name]['report'];assert old_entry==entry(old_entry['path'])
    old=read(old_entry['path']);p=ROOT/(name+'-regression.json');r=read(p);hashes(r)
    for a,b in zip(r['cases'],old['cases'],strict=True):
        for k in ['name','response','sha256','sourceSha256','byteLength','error','code','resultSha256']:assert a.get(k)==b.get(k),(name,k)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
historical={
 'shaping':('text-shaping','textReport'),'unicode':('unicode-text','unicodeReport'),
 'cascade':('font-cascade','cascadeReport'),'bidi':('bidi','bidiReport'),'bidiConformance':('bidi','conformanceReport'),
 'itemizationAndParagraph':('font-fallback','updatedItemizationReport'),'mixedFont':('font-fallback','fallbackReport'),
 'lineBreaking':('line-break','lineBreakReport'),'fontMetrics':('font-metrics','metricsReport'),
 'lineShaping':('line-shaping','lineShapeReport'),'lineGeometry':('line-geometry','lineGeometryReport'),
 'paragraphLayout':('paragraph-layout','paragraphLayoutReport'),'fontOutlines':('font-outlines','outlinesReport'),
 'paragraphPaths':('paragraph-paths','paragraphPathsReport'),
}
for name,(milestone,key) in historical.items():
    p=prior['regressionReports'][name]['report']['path'];r=read(p)
    old=read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
    field='batches' if name=='bidiConformance' else 'cases';assert r[field]==old[field],name
    hashes(r);cases(r);regressions[name]={'report':entry(p),'batches':len(r[field]),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
for name,p,old in [('pathRaster','.codex-work/path-raster/parity.json',read('docs/reviews/evidence/2026-09-24-path-raster-verification.json')['pathRasterReport']),('sceneRaster','.codex-work/scene-raster/parity.json',read('docs/reviews/evidence/2026-09-24-scene-raster-reachability-verification.json')['sceneRasterReport'])]:
    r=read(p);assert r['cases']==old['cases'];hashes(r,True);cases(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==2163
for name,p,key,raster in [
 ('export',ROOT/'export-regression.json','exportRegressionReport',False),
 ('placement','.codex-work/page-placement/parity.json','pagePlacementReport',True),
 ('groups','.codex-work/angle-export/parity.json','groupPlacementReport',True),
 ('angles','.codex-work/angle-export/angle-parity.json','angleExportReport',False),
 ('angleSource','.codex-work/angle-export/source-parity.json','angleSourceReport',False),
]:
    r=read(p);old=prior[key];hashes(r,raster);cases(r)
    assert r['cases']==old['cases'],name
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==2271
pages=read(ROOT/'parity.json');hashes(pages,True);cases(pages)
assert len(pages['cases'])==74 and sum(c['status']=='rendered' for c in pages['cases'])==53
assert pages['exactPixelAndMetadataEquality'] and pages['exactPlanEquality'] and pages['priorGroupPixelsUnchanged']==3 and pages['cliExclusivePublication'] and pages['failedPageHasNoArtifact']
reference=read(ROOT/'reference.json');assert reference['counts']=={'pages':53,'objects':8305,'controlCoordinates':71806,'trueArcSamples':6300,'analyticRemainders':26}
for c in reference['cases']:
    original=next(p for p in pages['cases'] if p['name']==c['name'])
    for k in ['planSha256','frameSha256']:assert c[k]==original[k]
contracts=read(ROOT/'contracts.json');assert contracts['schemas']==49 and contracts['pageRenderResponses']==74 and contracts['pageCompileResponses']==74
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')];assert len(schemas)==46
for s in schemas:assert s==entry(s['path'])
inputs=read(ROOT/'external/inputs.json');hashes(inputs);cases(inputs)
observations=read(ROOT/'external/observations.json');audits=read(ROOT/'external/independent-files.json');assert len(inputs['cases'])==len(observations['cases'])==len(audits)==8
assert observations['allWithinObservationTolerance'] and inputs['nativeWasmPptxEqual']
for a,b,c in zip(inputs['cases'],observations['cases'],audits,strict=True):
    assert a['name']==b['name']==c['name']
    for key in ['pptx','plan','pixels']:assert b[key]==entry(a[key+'Path'])
    assert b['pdf']==entry(b['pdf']['path']) and b['withinObservationTolerance']
    assert c['validation']['fileSha256']==a['pptxSha256'] and c['validation']['result']=='passed'
assert sum(len(c['validation']['schemasChecked']) for c in audits)==48
names=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M)
assert len(names)==242 and 'FAILED' not in (ROOT/'rust-tests.log').read_text()
assert 'Finished' in (ROOT/'clippy.log').read_text() and 'error:' not in (ROOT/'clippy.log').read_text()
assert 'error' not in (ROOT/'types-check.log').read_text().lower()
# External Cargo versions/checksums did not change; only internal edges did.
import tomllib
lock=tomllib.loads(Path('Cargo.lock').read_text())
external=[p for p in lock['package'] if 'source' in p]
assert entry('pnpm-lock.yaml')==next(s for s in prior['sourceFiles'] if s['path']=='pnpm-lock.yaml')
paths={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
paths.update(['README.md','docs/README.md','Cargo.lock','Cargo.toml'])
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
result={
 'format':'musteroffice.page-render-verification/1',
 'scope':'Actual author page -> certified solid geometry / Draw IR -> CPU raster. Explicit subset with atomic failure, not complete PPT rendering, Office/WPS fidelity, or Musterwork replacement acceptance.',
 'previousEvidence':entry(previous),'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development binaries, not complete engine or desktop installation sizes.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'checks':{'rustTests':242,'newRustTests':5,'nativeWasmLogicalBatches':2345,'previousLogicalBatchesUnchanged':2271,'newPageLogicalCases':74,'newPageApiComparisons':148,'pageRendered':53,'pageRejected':21,'runtimeSchemas':49,'previousSchemasUnchanged':46,'independentFileAudits':8,'ecmaSchemaPartInstances':48,'libreOfficeGeometryObservations':8,'strictClippy':True,'rustfmt':True,'typescript':True},
 'pageRenderReport':pages,'independentGeometryReference':reference,'externalInputs':inputs,'externalObservations':observations,'independentFileAudits':audits,'contractReport':contracts,'regressionReports':regressions,'rustTestNames':names,
 'dependencyChanges':{'externalRuntimeVersions':[],'internalEdgesAdded':['mo-presentation-compile -> mo-render','mo-presentation-compile -> mo-raster'],'currentExternalCargoPackages':external,'developmentOnlyTools':['python-pptx','lxml','PyMuPDF','Pillow','NumPy']},
 'sourceFiles':[entry(p) for p in sorted(paths)],
 'limitations':[
  'Current page profile rejects text, pictures, connectors, strokes and unresolved inherited appearance. It does not silently omit them; independent text APIs remain available.',
  'Only existing author geometry, master/layout/slide model and explicit background/theme context are evaluated. Full placeholders, effects, preset geometry and source import are unfinished.',
  'Certified error bounds cover coordinates and curve approximation, not antialias coverage, hinting, color fidelity or Office pixel equality.',
  'Exact uniform full-page viewport only. Arbitrary viewports, page tiles and general clip require additional semantics.',
  'LibreOffice observations compare sampled evaluated-path/PDF boundaries of eight owned files. New curve cases have no WPS/PowerPoint or editing-roundtrip acceptance.',
  'Production workers/pools, full resource/RSS budgets, fine-grained validation/hash/serialization cancellation and cross-platform runtime evidence remain incomplete.',
  'Playback, advanced editable objects, Agent integrations, packaging/performance gates and Musterwork E0-E3 are unfinished.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
OUTPUT.write_text(raw);print(json.dumps({'evidence':entry(OUTPUT),'sourceFiles':len(paths),'checks':result['checks'],'artifactByteDeltas':result['artifactByteDeltas']},indent=2))
