"""Seal the author transform correction and real target application observations."""
import hashlib
import json
import platform
import re
from pathlib import Path

ROOT=Path('.codex-work/group-compat')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-group-placement-verification.json')
assert not OUTPUT.exists(), 'immutable evidence already exists'

def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);data=p.read_bytes()
    return {'path':str(p),'byteLength':len(data),'sha256':hashlib.sha256(data).hexdigest()}

previous='docs/reviews/evidence/2026-09-24-page-placement-verification.json'
prior=read(previous)
assert entry(previous)['sha256']=='e49040a0c05374c2b9a12e01114caf57bc7b8f94395d20a2bca78e11578f27fa'
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
unchanged=[k for k in artifacts if k not in ['nativeCli','rustWasm']]
for k in unchanged:assert artifacts[k]==prior['artifacts'][k],k

def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli',
             'nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm',
             'componentSha256':'rasterWasm' if raster else 'cppWasm',
             'componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
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
    old_entry=prior['regressionReports'][name]['report']
    assert old_entry==entry(old_entry['path'])
    old=read(old_entry['path']);p=ROOT/(name+'-regression.json');r=read(p);hashes(r)
    for a,b in zip(r['cases'],old['cases'],strict=True):
        for k in ['name','response','sha256','sourceSha256','byteLength','error','code','resultSha256']:
            assert a.get(k)==b.get(k),(name,k)
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
    hashes(r);cases(r)
    regressions[name]={'report':entry(p),'batches':len(r[field]),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
for name,p,old in [('pathRaster','.codex-work/path-raster/parity.json',read('docs/reviews/evidence/2026-09-24-path-raster-verification.json')['pathRasterReport']),
                   ('sceneRaster','.codex-work/scene-raster/parity.json',read('docs/reviews/evidence/2026-09-24-scene-raster-reachability-verification.json')['sceneRasterReport'])]:
    r=read(p);assert r['cases']==old['cases'];hashes(r,True);cases(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==2180
placement=read('.codex-work/page-placement/parity.json');groups=read(ROOT/'parity.json')
for r in [placement,groups]:hashes(r,True);cases(r)
assert len(placement['cases'])==64 and len(groups['cases'])==3
assert sum(c['status']=='evaluated' for r in [placement,groups] for c in r['cases'])==55
# Author requests are identical; only the declared draft semantics were changed.
for a,b in zip(placement['cases'],prior['pagePlacementReport']['cases'],strict=True):
    assert a['requestSha256']==b['requestSha256'] and a['name']==b['name'] and a['status']==b['status']
    if a['status']!='evaluated':assert a['responseSha256']==b['responseSha256']
references=[read('.codex-work/page-placement/reference.json'),read(ROOT/'reference.json')]
counts={k:sum(r['counts'][k] for r in references) for k in references[0]['counts']}
assert counts=={'documents':55,'objects':8931,'coefficients':53586,'sourceControlPoints':44854}
for directory,reference in zip([Path('.codex-work/page-placement'),ROOT],references,strict=True):
    for c in reference['cases']:assert entry(directory/(c['name']+'.response.json'))['sha256']==c['responseSha256']
renders=[read('.codex-work/page-placement/render.json'),read(ROOT/'render.json')]
for r in renders:
    hashes(r,True);cases(r);assert r['nativeWasmPixelsAndPptxEqual']
    for c in r['cases']:assert int(c['authorCoordinateErrorBound'])+int(c['sceneCoordinateErrorBound'])<=2**24
assert sum(len(r['cases']) for r in renders)==7
external=read('.codex-work/page-placement/external.json')
assert len(external['cases'])==4 and external['allWithinObservationTolerance']
for a,b in zip(external['cases'],prior['externalObservation']['cases'],strict=True):
    for k in ['source','pdf']:
        assert a[k]==b[k] and a[k]==entry(a[k]['path'])
    assert a['maximumVertexToBoundaryDistancePt']<.1 and b['maximumVertexToBoundaryDistancePt']>28
observations=read(ROOT/'observations.json');observed=[c for p in observations['cases'] for c in p['cases']]
assert len(observed)==48 and all(c['wpsWithinCoarseObservationTolerance'] for c in observed)
assert sum(c['libreOfficeWithinObservationTolerance'] for c in observed)==33
for p in observations['cases']:
    for k in ['source','placementResponse','privateCapture','pdf']:assert p[k]==entry(p[k]['path'])
    assert p['source']['sha256']==entry(ROOT/'observed-inputs'/(p['name']+'.pptx'))['sha256']
contracts=read(ROOT/'contracts.json');assert contracts['schemas']==46 and contracts['pagePlacementResponses']==67
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')]
assert len(schemas)==46
for s in schemas:assert s==entry(s['path'])
for path in ['Cargo.toml','Cargo.lock','crates/mo-presentation-compile/Cargo.toml']:
    old=next(s for s in prior['sourceFiles'] if s['path']==path);assert old==entry(path)
names=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M)
assert len(names)==235 and 'FAILED' not in (ROOT/'rust-tests.log').read_text()
assert 'Finished' in (ROOT/'clippy.log').read_text() and 'error:' not in (ROOT/'clippy.log').read_text()
assert 'error' not in (ROOT/'types-check.log').read_text().lower()
paths={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
paths.update(['README.md','docs/README.md','docs/design/implementation/algorithms-and-components.md'])
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
lo_source=entry('.codex-work/group-compat/lo-shape.cxx')
assert lo_source['sha256']=='4fe33f65097a140d8065536be7947bf2e99bcade570fde86a4bc01a40309a187'
result={
 'format':'musteroffice.group-placement-verification/1',
 'scope':'Correction of author group scaling/reflection/center semantics, supported by 48 actual WPS geometric observations. Complete painting, pixel fidelity, Office validation and editing round-trip remain incomplete.',
 'profile':'drawingml-sector-scale-reflection-q96-enclosure-v2-draft',
 'previousEvidence':entry(previous),'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development binaries; not the complete kernel or Musterwork installer.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'references':[
  {'url':'https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip','sections':['L.4.7.3','L.4.7.4','L.4.7.5','L.4.7.6'],'role':'Informative annex used for the superseded first profile; not an empirical interoperability gate.'},
  {'url':'https://learn.microsoft.com/lb-lu/openspecs/office_standards/ms-oi29500/98567b7e-097a-4450-a82a-f83e25539aaf','role':'Microsoft implementation notes for 20.1.7.5, units and group child box. Does not establish the entire observed angular-sector rule.'},
  {'url':'https://github.com/LibreOffice/core/blob/afbbd0df0edb6d40b450b0337ac646b0913a760c/oox/source/drawingml/shape.cxx','source':lo_source,'role':'Read-only research lead for scale-axis switching and reflection. No source copied into runtime or linked; WPS observations independently inform the implemented rule.'},
 ],
 'checks':{'rustTests':235,'newRustTests':3,'nativeWasmBatches':2247,'placementBatches':67,'oldUnrelatedSemanticBatchesUnchanged':2180,'existingPlacementInputsUnchanged':64,'runtimeSchemas':46,'allSchemasUnchanged':True,'independentHighPrecisionReference':True,'actualRasterAndEditablePptxPages':7,'wpsGeometricProbes':48,'wpsWithinCoarseObservationTolerance':48,'libreOfficeOriginalRectanglesWithinTolerance':4,'libreOfficeNewProbesWithinTolerance':33,'libreOfficeNewProbesDifferent':15,'strictClippy':True,'rustfmt':True,'typescript':True},
 'pagePlacementReport':placement,'groupPlacementReport':groups,'independentReferences':references,'independentReferenceCounts':counts,
 'diagnosticRenderReports':renders,'originalRectangleObservation':external,'targetObservations':observations,
 'contractReport':contracts,'regressionReports':regressions,'rustTestNames':names,
 'sourceFiles':[entry(p) for p in sorted(paths)],
 'visualInspection':{'kernelPreview':entry(ROOT/'nested-kernel.png'),'observation':'Actual native/WASM raster of nested asymmetric paths visually compared with the WPS native window capture. Full WPS screenshots remain private because unrelated application tabs are visible.'},
 'dependencyChanges':{'newExternalRuntimeVersions':[],'lockfileUnchanged':True,'developmentOnly':['PyMuPDF','Pillow','NumPy']},
 'limitations':[
  'WPS 12.1.22553 screen capture measurements are coarse geometric samples with resampling/compression and registration uncertainty, not pixel-perfect, editing round-trip or Office acceptance.',
  '15 raw signed/multi-turn-angle samples differ in LibreOffice 26.2.0.3. Inputs and disagreements remain recorded; no flattening or source rewriting conceals them.',
  'The rule is a draft target profile supported by the recorded finite cases. Other Office/WPS versions, deeper nested application cases and degenerate groups require further observations.',
  'Current author model lacks arbitrary child offset and zero child extents. Whole-model validation and canonical digest cancellation still use pre/post checks.',
  'Diagnostic page bridges assert only owned solid paths/rectangles. Full geometry, text, style, media, playback and production page painting remain unfinished.',
  'Other operating systems, browser workers, production resources/performance, public Agent surfaces, Musterwork integration and E0-E3 remain incomplete.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
OUTPUT.write_text(raw)
print(json.dumps({'evidence':entry(OUTPUT),'sourceFiles':len(paths),'checks':result['checks']},indent=2))
