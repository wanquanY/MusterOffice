"""Seal preset implementation, exact contract migration and real cross-end renders."""
import hashlib,json,re,sys
from pathlib import Path
ROOT=Path('.codex-work/preset-expansion')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-gradient-raster-verification.json')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-preset-expansion-verification.json')
read=lambda p:json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
assert entry(PREVIOUS)['sha256']=='9ce82669b289c405bdf636f9180dff1e744e6748fa9065af25006b9bf2836702'
old=read(PREVIOUS);artifacts={k:entry(v['path']) for k,v in old['artifacts'].items()}
for key in ['rustWasmGlue','cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[key]==old['artifacts'][key]
for name in ['Cargo.lock','pnpm-lock.yaml']:assert entry(name)==next(x for x in old['sourceFiles'] if x['path']==name)
def bound_files(value):
    if isinstance(value,list):
        for v in value:bound_files(v)
    elif isinstance(value,dict):
        for k,v in value.items():
            if k.endswith('Path') and k[:-4]+'Sha256' in value:assert entry(v)['sha256']==value[k[:-4]+'Sha256'],v
            if isinstance(v,(list,dict)):bound_files(v)
def current_artifacts(report,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','nativeRasterWorkerSha256':'nativeRasterWorker','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    for k,a in aliases.items():
        if k in report:assert report[k]==artifacts[a]['sha256'],(k,a)
audit=read(ROOT/'regression-audit.json');assert audit['previousEvidenceSha256']==entry(PREVIOUS)['sha256']
assert (audit['unchangedBatches'],audit['migratedBatches'])==(5541,2197)
regressions={}
for name,info in audit['reports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==old['regressionReports'][name]['report']['sha256']
    report=read(info['reportPath']);current_artifacts(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster','gradientRaster']);bound_files(report.get('cases',report.get('batches')))
    field='batches' if name=='bidiConformance' else 'cases'
    regressions[name]={'report':entry(info['reportPath']),'batches':len(report[field]),'migrationAudit':info}
assert len(regressions)==41 and sum(r['batches'] for r in regressions.values())==7738
parity=read(ROOT/'parity.json');current_artifacts(parity,True);bound_files(parity['cases'])
assert len(parity['cases'])==1329 and parity['exactNativeWasmResponsesAndPixels']
assert {kind:sum(c['kind']==kind for c in parity['cases']) for kind in ['geometry','paths','raster']}=={'geometry':571,'paths':571,'raster':187}
independent=read(ROOT/'independent.json');paths=read(ROOT/'paths-independent.json')
assert (independent['numericValues'],independent['originsChecked'],independent['validXsdSlides'])==(43832,52091,571)
assert (paths['compiled'],paths['unresolved'],paths['paths'],paths['controlPoints'],paths['arcSegments'],paths['ellipseSamples'])==(568,3,967,75481,22852,68556)
by_key={(c['name'],c['kind']):c for c in parity['cases']}
for c in independent['cases']:assert c['responseSha256']==by_key[(c['name'],'geometry')]['responseSha256']
for c in paths['cases']:assert c['responseSha256']==by_key[(c['name'],'paths')]['responseSha256']
contracts=read(ROOT/'contracts.json');assert (contracts['schemas'],contracts['presetExpansionRequests'],contracts['presetExpansionResponses'])==(61,1329,1329)
assert (contracts['nativePathResponses'],contracts['geometryEvaluationResponses'])==(814,1383)
catalog=read('components/drawingml-presets/catalog.json');assert len(catalog['definitions'])==187
assert entry('components/drawingml-presets/catalog.xml')['sha256']==catalog['catalogSha256']
assert entry('components/drawingml-presets/catalog-index.bin')['sha256']==catalog['indexSha256']
preview=read(ROOT/'preview.json');assert entry(preview['path'])['sha256']==preview['sha256'] and len(preview['sourcePixelSha256'])==187
assert preview['sourcePixelSha256']==[c['pixelsSha256'] for c in parity['cases'] if c['kind']=='raster']
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==363
logs=['workspace-tests.log','clippy.log','fmt.log','native-build.log','wasm-build.log','bindgen.log','schema-check.log','types-check.log','catalog-check.log','parity.log','independent.log','paths-independent.log','regressions.log','regression-audit.log','geometry-regression-independent.log','path-regression-independent.log']
for name in logs:
    text=(ROOT/name).read_text();assert not any(x in text for x in ['error:','FAILED','Traceback','AssertionError']),name
changed_schemas=[x['path'] for x in old['sourceFiles'] if x['path'].startswith('contracts/generated/') and entry(x['path'])!=x]
assert set(changed_schemas)=={'contracts/generated/'+s+'.schema.json' for s in ['pptx-geometry-query','pptx-geometry-response','pptx-paths-query','pptx-paths-response']}
sources={x['path'] for x in old['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.xml','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
regressions['presetExpansion']={'report':entry(ROOT/'parity.json'),'batches':1329}
result={'format':'musteroffice.preset-expansion-verification/1','previousEvidence':entry(PREVIOUS),
 'checks':{'rustTests':363,'newRustTests':3,'nativeWasmLogicalBatches':9067,'previousLogicalBatchesRerun':7738,'newPresetBatches':1329,'newRenderedFrames':187,'presetNames':187,'nativeInputPackages':571,'runtimeSchemas':61,'changedSchemas':4,'strictClippy':True,'rustfmt':True,'typescript':True},
 'scope':'Source prstGeom expansion, typed document/catalog provenance and native paths; diagnostic paint only, not complete source-page rendering or target-application acceptance.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-old['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts, not full kernel or Musterwork installer. No new RSS/startup/latency benchmark.',
 'environment':old['environment'],'regressionReports':regressions,'migrationAudit':audit,'presetReport':parity,'independentReference':independent,'pathReference':paths,'contracts':contracts,
 'catalog':catalog,'catalogSource':read('components/drawingml-presets/source.json'),'catalogRuntimeBytes':350386,'preview':preview,
 'oldIndependentReferences':{name:entry(path) for name,path in [('geometry','.codex-work/geometry-eval/independent.json'),('paths','.codex-work/native-paths/independent.json')]},
 'rustTestNames':tests,'sourceFiles':[entry(p) for p in sorted(sources)],'validationLogs':[entry(ROOT/p) for p in logs],'changedSchemas':changed_schemas,
 'dependencyChanges':{'newDataComponent':'components/drawingml-presets','externalRuntimeVersions':[],'cargoAndPnpmLocksUnchanged':True},
 'limitations':['Only the declared preset defaults/aspect ratios and seven adjustment probes are tested; this is not exhaustive adjustment or degeneracy acceptance.',
 'Preset appendix repairs and reflected upArrow are explicit draft rules; target Office/WPS numerical, handle interaction and visual comparisons remain pending.',
 'Derived geometry/path provenance and geometry profile change together; old v1 requests are rejected, raw source declarations stay unchanged.',
 'New pixels use explicit diagnostic fill/stroke, not source styles. Complete page transforms, text/image/effect painting and native spatial gradients remain required.',
 'Only macOS arm64 Native and Node-hosted WASM are verified. C++ components are reused; no new sanitizer or leak certification is claimed.',
 'Complete preset authoring/edit operations, advanced objects/playback, Agent services, editable application roundtrip and Musterwork E0-E3 remain incomplete.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':result['checks'],'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
