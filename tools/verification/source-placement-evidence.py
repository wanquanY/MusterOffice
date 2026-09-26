"""Seal digest-bound source placement, shared arithmetic regressions and observations."""
import hashlib,json,re,sys
from pathlib import Path
ROOT=Path('.codex-work/source-placement');PREVIOUS=Path('docs/reviews/evidence/2026-09-25-preset-expansion-verification.json');OUTPUT=Path('docs/reviews/evidence/2026-09-25-source-placement-verification.json')
read=lambda p:json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
assert entry(PREVIOUS)['sha256']=='69748c15d357b6a7ad4beee1ebc27728d2d05f75b35e33dcd27c501e36f82937'
old=read(PREVIOUS);artifacts={k:entry(v['path']) for k,v in old['artifacts'].items()}
for key in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[key]==old['artifacts'][key]
for name in ['Cargo.lock','pnpm-lock.yaml']:assert entry(name)==next(x for x in old['sourceFiles'] if x['path']==name)
def bound_files(v):
    if isinstance(v,list):
        for x in v:bound_files(x)
    elif isinstance(v,dict):
        for k,x in v.items():
            if k.endswith('Path') and k[:-4]+'Sha256' in v:assert entry(x)['sha256']==v[k[:-4]+'Sha256'],x
            if isinstance(x,(list,dict)):bound_files(x)
def current_artifacts(report,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','nativeRasterWorkerSha256':'nativeRasterWorker','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    for k,a in aliases.items():
        if k in report:assert report[k]==artifacts[a]['sha256'],(k,a)
audit=read(ROOT/'regression-audit.json');assert audit['previousEvidenceSha256']==entry(PREVIOUS)['sha256'];assert (audit['unchangedBatches'],audit['migratedBatches'])==(9066,1)
regressions={}
for name,info in audit['reports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==old['regressionReports'][name]['report']['sha256']
    report=read(info['reportPath']);current_artifacts(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster','gradientRaster','presetExpansion']);field='batches' if name=='bidiConformance' else 'cases';bound_files(report[field])
    regressions[name]={'report':entry(info['reportPath']),'batches':len(report[field]),'migrationAudit':info}
assert len(regressions)==42 and sum(r['batches'] for r in regressions.values())==9067
parity=read(ROOT/'parity.json');current_artifacts(parity,True);bound_files(parity);assert parity['exactNativeWasmResponsesAndPixels']
assert {k:sum(c['kind']==k for c in parity['cases']) for k in ['source','placement','paths','raster']}=={'source':146,'placement':156,'paths':16,'raster':16}
reference=read(ROOT/'reference.json');source=read(ROOT/'source-check.json');contracts=read(ROOT/'contracts-combined.log');obs=read(ROOT/'observations.json');preview=read(ROOT/'preview.json');bound_files(obs)
assert reference['counts']=={'objects':363,'coefficients':2178,'controlPoints':1815}
assert (source['transforms'],source['validXsdSlides'],len(source['invalidXsdSlides']),source['retainedLocations'])==(1637,137,9,3)
assert (contracts['schemas'],contracts['sourcePlacementResponses'],contracts['sourcePlacementRequests'])==(63,334,184)
assert (contracts['sourceResponses'],contracts['sourceRequests'],contracts['pagePlacementResponses'])==(73,95,64)
by_key={(c['name'],c['kind']):c for c in parity['cases']}
for kind,records in [('source',source['cases']),('placement',reference['cases'])]:
    for c in records:assert c['responseSha256']==by_key[(c['name'],kind)]['responseSha256']
assert len(obs['cases'])==8 and len(obs['differences'])==3
assert entry(preview['path'])['sha256']==preview['sha256'] and len(preview['frames'])==16
for c in preview['frames']:assert c['pixelsSha256']==by_key[(c['name'],'raster')]['pixelsSha256']
author=read('.codex-work/page-placement/reference.json');assert author['counts']=={'documents':52,'objects':8807,'coefficients':52842,'sourceControlPoints':44042}
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==371
logs=['workspace-tests.log','clippy.log','fmt.log','native-build.log','wasm-build.log','wasm-bindgen.log','schema-check.log','types-check.log','parity.log','reference.log','source-check.log','regressions.log','regression-audit.log','author-reference.log','contracts-combined.log','observations.log','preview.log']
for name in logs:
    t=(ROOT/name).read_text();assert not any(x in t for x in ['error:','FAILED','Traceback','AssertionError']),name
changed=[x['path'] for x in old['sourceFiles'] if x['path'].startswith('contracts/generated/') and entry(x['path'])!=x];assert changed==['contracts/generated/pptx-source-response.schema.json']
sources={x['path'] for x in old['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.xml','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
regressions['sourcePlacement']={'report':entry(ROOT/'parity.json'),'batches':334}
result={'format':'musteroffice.source-placement-verification/1','previousEvidence':entry(PREVIOUS),
 'checks':{'rustTests':371,'newRustTests':8,'nativeWasmLogicalBatches':9401,'previousLogicalBatchesRerun':9067,'newSourcePlacementBatches':334,'newRenderedFrames':16,'nativeInputPackages':146,'runtimeSchemas':63,'changedExistingSchemas':1,'newSchemas':2,'strictClippy':True,'rustfmt':True,'typescript':True},
 'scope':'Imported object/group placements with resolved transform provenance and shared authored arithmetic. Diagnostic source-path painting only; source page compilation and target-application equivalence remain incomplete.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-old['artifacts'][k]['byteLength'] for k,v in artifacts.items()},'artifactSizeScope':'Uncompressed incomplete development artifacts, not a complete kernel or Musterwork installer. No new product latency/RSS measurements.',
 'environment':old['environment'],'regressionReports':regressions,'migrationAudit':audit,'placementReport':parity,'independentReference':reference,'sourceReference':source,'authorReference':author,'contracts':contracts,'targetObservations':obs,'preview':preview,
 'rustTestNames':tests,'sourceFiles':[entry(p) for p in sorted(sources)],'validationLogs':[entry(ROOT/p) for p in logs],'changedSchemas':changed,'newSchemas':['contracts/generated/pptx-placement-query.schema.json','contracts/generated/pptx-placement-response.schema.json'],
 'dependencyChanges':{'externalRuntimeVersions':[],'cargoAndPnpmLocksUnchanged':True},
 'limitations':['The draft is not a certified Office/WPS transform profile. WPS differs on tested zero-child-extent groups and partial placeholder transforms; differences remain open.',
 'Target observations only check owned rectangle visibility/bounds in WPS and LibreOffice PDF. Microsoft Office and save/edit/reopen were not observed.',
 'The source query does not compose master/layout/slide visibility, source brushes/effects, text, pictures or advanced content into a complete page.',
 'Diagnostic raster requests subtract source anchors and use explicit red paint. They do not certify native styling or full error propagation across source guide math and page lowering.',
 'Queries use macOS arm64 Native and Node-hosted WASM; browser workers, product cancellation/latency/memory gates and other platforms remain unaccepted.',
 'Advanced objects/playback, complete author/edit/export, Agent operation services and Musterwork E0-E3 remain incomplete.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':result['checks'],'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
