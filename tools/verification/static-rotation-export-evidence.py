"""Seal static-angle canonical export, source preservation and real app evidence."""
import hashlib,json,platform,re
from pathlib import Path

ROOT=Path('.codex-work/angle-export')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-static-rotation-export-verification.json')
assert not OUTPUT.exists(),'immutable evidence already exists'
def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
previous='docs/reviews/evidence/2026-09-24-group-placement-verification.json';prior=read(previous)
assert entry(previous)['sha256']=='969fff2c77e161226fb143f0e8e93c59c9e285dcbb76ffa14234341025e6cff6'
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in artifacts:
    if k not in ['nativeCli','rustWasm']:assert artifacts[k]==prior['artifacts'][k],k

def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm',
             'componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256','nativeCliSha256']) and any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in r:assert r[k]==artifacts[a]['sha256'],k

def cases(r):
    for c in r.get('cases',[]):
        for k in ['request','canonicalRequest','response','source','font','bundle','frame','pixels','scene','pptx']:
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
for name,p,old in [('pathRaster','.codex-work/path-raster/parity.json',read('docs/reviews/evidence/2026-09-24-path-raster-verification.json')['pathRasterReport']),
                   ('sceneRaster','.codex-work/scene-raster/parity.json',read('docs/reviews/evidence/2026-09-24-scene-raster-reachability-verification.json')['sceneRasterReport'])]:
    r=read(p);assert r['cases']==old['cases'];hashes(r,True);cases(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'artifactHashesVerified':True,'semanticResultsUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==2163
placement=read('.codex-work/page-placement/parity.json');groups=read(ROOT/'parity.json')
for r,old in [(placement,prior['pagePlacementReport']),(groups,prior['groupPlacementReport'])]:
    hashes(r,True);cases(r)
    for a,b in zip(r['cases'],old['cases'],strict=True):
        for k in ['name','status','requestSha256','responseSha256']:assert a[k]==b[k],(a['name'],k)
assert len(placement['cases'])+len(groups['cases'])==67
references=[read('.codex-work/page-placement/reference.json'),read(ROOT/'reference.json')]
for directory,r in zip([Path('.codex-work/page-placement'),ROOT],references,strict=True):
    for c in r['cases']:assert c['responseSha256']==entry(directory/(c['name']+'.response.json'))['sha256']
counts={k:sum(r['counts'][k] for r in references) for k in references[0]['counts']};assert counts==prior['independentReferenceCounts']
render=read(ROOT/'render.json');hashes(render,True);cases(render);assert render['nativeWasmPixelsAndPptxEqual']
for a,b in zip(render['cases'],prior['diagnosticRenderReports'][1]['cases'],strict=True):
    for k in ['name','sceneSha256','pixelsSha256','placementResponseSha256']:assert a[k]==b[k],k
export=read(ROOT/'export-regression.json');hashes(export);assert len(export['cases'])==17
angles=read(ROOT/'angle-parity.json');source=read(ROOT/'source-parity.json')
for r in [angles,source]:hashes(r);cases(r)
assert len(angles['cases'])==20 and len(source['cases'])==4
independent=read(ROOT/'independent.json')
assert independent['totals']=={'comparedPackages':10,'auditedPackages':30,'schemaParts':288,'nativeObjects':529,'canonicalRotationValues':529,'changedRotationTokens':23}
for c in independent['packageComparisons']:
    for k in ['before','after']:assert c[k]==entry(c[k]['path'])
    assert c['allOtherUncompressedBytesIdentical'] and c['canonicalizedRotationTokens']>0
for r in independent['fileAudits']:assert r['result']=='passed' and not r['differences']
# Bind independent requests/results to exactly the current runtime outputs.
expected=[(c['pptxSha256'],c['requestSha256']) for c in angles['cases']]
expected += [(entry(c['pptx'])['sha256'],entry(ROOT/(c['name']+'.export.json'))['sha256']) for c in read(ROOT/'probes.json')]
for c in export['cases']:
    if c['status']=='exported':expected.append((c['sha256'],entry(Path(export['artifactDirectory'])/(c['name']+'.json'))['sha256']))
assert sorted(expected)==sorted((r['fileSha256'],r['requestSha256']) for r in independent['fileAudits'])
compat=read(ROOT/'compatibility.json');obs=compat['external']
assert obs==read(ROOT/'observations.json')
observed=[c for p in obs['cases'] for c in p['cases']]
assert len(observed)==48 and all(c['wpsWithinCoarseObservationTolerance'] and c['libreOfficeWithinObservationTolerance'] for c in observed)
for p in obs['cases']:
    for k in ['source','placementResponse','privateCapture','pdf']:assert p[k]==entry(p[k]['path'])
    assert p['source']['sha256']==entry(ROOT/'observed-inputs'/(p['name']+'.pptx'))['sha256']
assert sum(c['canvasPixelsCompared'] for c in compat['wpsCanvasComparisons'])==1730538
for c in compat['wpsCanvasComparisons']:
    assert c['pixelsDifferent']==0 and c['afterLibreOfficeMaxPt']<.2
    for k in ['rawInput','canonicalInput','rawPrivateCapture','canonicalPrivateCapture']:assert c[k]==entry(c[k]['path'])
assert len(compat['sourcePreservation'])==4 and sum(c['unchangedCompressedEntriesVerified'] for c in compat['sourcePreservation'])==80
for a,b in zip(compat['sourcePreservation'],source['cases'],strict=True):
    assert a['sourceSha256']==b['sourceSha256'] and a['outputSha256']==b['pptxSha256'] and a['result']=='passed'
contracts=read(ROOT/'contracts.json');assert contracts['schemas']==46 and contracts['pagePlacementResponses']==67
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')];assert len(schemas)==46
for s in schemas:assert s==entry(s['path'])
for s in prior['sourceFiles']:
    if s['path'].endswith('Cargo.toml') or s['path'] in ['Cargo.lock','pnpm-lock.yaml']:assert s==entry(s['path'])
names=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M)
assert len(names)==237 and 'FAILED' not in (ROOT/'rust-tests.log').read_text()
assert 'Finished' in (ROOT/'clippy.log').read_text() and 'error:' not in (ROOT/'clippy.log').read_text()
assert 'error' not in (ROOT/'types-check.log').read_text().lower()
paths={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
paths.update(['README.md','docs/README.md','docs/design/implementation/algorithms-and-components.md'])
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
result={
 'format':'musteroffice.static-rotation-export-verification/1',
 'scope':'Static a:xfrm rotations normalize on new authored export. Author raw values and source-preserving XML retain their original representation. Animation travel is outside this rule. Full Office/WPS interoperability, playback and replacement are not accepted.',
 'previousEvidence':entry(previous),'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts, not a complete kernel or Musterwork installer.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'standard':{'url':'https://ecma-international.org/wp-content/uploads/ECMA-376-1_5th_edition_december_2016.zip','section':'20.1.10.3 ST_Angle','pdf':entry('.codex-work/ecma376/part1.pdf'),'interpretation':'Signed 60000ths of a degree are legal. Nonnegative one-turn static serialization is an empirically validated export choice, not rejection of signed input.'},
 'checks':{'rustTests':237,'newRustTests':2,'nativeWasmBatches':2271,'existingNonExportBatchesUnchanged':2230,'exportRegressionBatches':17,'exportPackagesChangedOnlyAtRotationTokens':7,'exportRejectionsUnchanged':10,'newAngleExportBatches':20,'newSourcePreservationBatches':4,'existingPlacementBatchesUnchanged':67,'runtimeSchemas':46,'allSchemasUnchanged':True,'independentFileAudits':30,'ecmaSchemaPartInstances':288,'wpsGeometryProbes':48,'libreOfficeGeometryProbes':48,'wpsCanvasPixelsCompared':1730538,'wpsCanvasPixelsDifferent':0,'strictClippy':True,'rustfmt':True,'typescript':True},
 'angleExportReport':angles,'angleSourceReport':source,'exportRegressionReport':export,
 'independentFileValidation':independent,'compatibilityObservations':compat,
 'pagePlacementReport':placement,'groupPlacementReport':groups,'independentReferences':references,'diagnosticRenderReport':render,
 'contractReport':contracts,'regressionReports':regressions,'rustTestNames':names,
 'dependencyChanges':{'externalRuntimeVersions':[],'manifestsAndLockfilesUnchanged':True,'developmentOnlyTools':['python-pptx','lxml','PyMuPDF','Pillow','NumPy']},
 'sourceFiles':[entry(p) for p in sorted(paths)],
 'limitations':[
  'WPS raw-vs-canonical equality covers the inspected canvas region of three actual captures. It is not equality between kernel raster and WPS, nor proof of a complete presentation.',
  'LibreOffice comparison covers the 48 owned static paths at the recorded version and a vertex-to-boundary observation tolerance, not complete visual fidelity or editing round-trip.',
  'Source-preserving text edits intentionally retain noncanonical rotations; source geometry editing and target-profile migration require separate semantics.',
  'Static transform normalization must not be applied to animation deltas, direction or cumulative revolutions.',
  'Existing free connector endpoint export accepts an equivalent zero orientation only. Other transform/endpoint mappings remain explicitly unsupported.',
  'Complete page painting, advanced editable objects, media/playback, PowerPoint and other OS/browser evidence, public Agent surfaces, performance/packaging and Musterwork E0-E3 remain unfinished.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
OUTPUT.write_text(raw);print(json.dumps({'evidence':entry(OUTPUT),'sourceFiles':len(paths),'checks':result['checks']},indent=2))
