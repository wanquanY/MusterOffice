"""Seal actual gradient painting and the ABI/typed-brush migration audit."""
import hashlib,json,platform,re,subprocess,sys
from pathlib import Path
ROOT=Path('.codex-work/gradient-raster')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-gradient-raster-verification.json')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-fill-color-verification.json')
read=lambda p:json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
assert entry(PREVIOUS)['sha256']=='e2e1759a80e39315b0063de94121fc79277cb4fd801661e98b7bbe0cd9375813'
old=read(PREVIOUS);artifacts={k:entry(v['path']) for k,v in old['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rustWasmGlue']:assert artifacts[k]==old['artifacts'][k]
def files(value):
    if isinstance(value,list):
        for v in value:files(v)
    elif isinstance(value,dict):
        if {'path','sha256'}<=value.keys():
            a=entry(value['path']);assert a['sha256']==value['sha256'],value['path']
            if 'byteLength' in value:assert a['byteLength']==value['byteLength']
        for k,v in value.items():
            if k.endswith('Path') and k[:-4]+'Sha256' in value:assert entry(v)['sha256']==value[k[:-4]+'Sha256'],v
            if isinstance(v,(dict,list)):files(v)
def artifact_hashes(report,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    for key,name in aliases.items():
        if key in report:assert report[key]==artifacts[name]['sha256'],(key,name)
    files(report.get('cases',report.get('batches')))
regressions={}
audit=read(ROOT/'regression-audit.json')
assert audit['previousEvidenceSha256']==entry(PREVIOUS)['sha256']
assert audit['counts']=={'unchangedBatches':6965,'migratedBatches':715,'identicalSolidPixels':620,'exactOldFramesReconstructed':620,'plansChecked':132}
for name,record in old['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==record['report']['sha256']
    path=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(record['report']['path'])
    report=read(path);artifact_hashes(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster'])
    field='batches' if name=='bidiConformance' else 'cases'
    if audit['reports'][name].get('casesUnchanged'):assert report[field]==read(snapshot)[field],name
    regressions[name]={'report':entry(path),'batches':len(report[field]),'migrationAudit':audit['reports'][name]}
assert len(regressions)==40 and sum(x['batches'] for x in regressions.values())==7680
parity=read(ROOT/'parity.json');artifact_hashes(parity,True)
assert len(parity['cases'])==58 and sum(c['status']=='rendered' for c in parity['cases'])==42
assert parity['exactNativeWasmPixelsAndMetadata'] and parity['sanitizedPixelsIdentical'] and len(parity['forgedComponentRejections'])==14
assert entry('.codex-work/skia/mo-skia-probe')['sha256']==parity['nativeProbeSha256']
assert entry('.codex-work/skia/mo-skia-probe-asan')['sha256']==parity['sanitizedProbeSha256']
reference=read(ROOT/'reference.json');assert reference['counts']=={'cases':39,'pixels':159744,'channels':638976,'wireGradientCoordinates':156,'wireStopValues':770}
assert reference['toleranceRgba8']==reference['maximumChannelDifference']==1
lookup={x['name']:x for x in parity['cases']}
for c in reference['cases']:
    assert c['pixelsSha256']==lookup[c['name']]['pixelsSha256'] and c['requestSha256']==lookup[c['name']]['requestSha256']
contracts=read(ROOT/'contracts.json');assert [contracts[k] for k in ['schemas','gradientRasterResponses','gradientRasterRequests','pathRasterResponses','sceneRasterResponses','pageRenderResponses']]==[61,58,54,267,316,132]
component=read('.codex-work/skia/verification/parity.json')
assert len(component['cases'])==197
files(component['cases'])
builds={name:read('.codex-work/skia/'+name+'-build.json') for name in ['native','wasm','native-asan']}
for name,b in builds.items():
    assert b['lock']==read('components/skia/lock.json')
    files(b['componentSources']);files(b['artifacts'])
    assert '-DSK_DISABLE_LOWP_RASTER_PIPELINE' in b['profile']['extra_cflags']
    assert '-DMO_SKIA_DETERMINISTIC_RASTER' in b['profile']['extra_cflags']
    assert '-ffp-contract=off' in b['profile']['extra_cflags']
closure=read(ROOT/'compiled-closure.json');assert len(closure['native']['translationUnits'])==497 and len(closure['wasm']['translationUnits'])==496
benchmark=read(ROOT/'benchmark.json');assert len(benchmark['results'])==6
for p,sha in benchmark['artifacts'].items():assert entry(p)['sha256']==sha
for case in benchmark['results']:
    for timing in case['timing'].values():assert len(timing['samplesMs'])==25 and timing['p95Ms']>=timing['medianMs']>0
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==360
logs=['workspace-tests.log','clippy.log','fmt.log','schema-check.log','types-check.log','parity.log','reference.log','regressions.log','regression-audit.log','native-build.log','wasm-build.log','bindgen.log','skia-native-build.log','skia-wasm-build.log','skia-asan-build.log','skia-parity.log','closure.log','path-reference.log','scene-reference.log','page-reference.log','benchmark.log']
for log in logs:
    s=(ROOT/log).read_text();assert not any(t in s for t in ['error:','FAILED','Traceback','AssertionError']),log
for lock in ['Cargo.lock','pnpm-lock.yaml']:assert entry(lock)==next(x for x in old['sourceFiles'] if x['path']==lock)
sources={x['path'] for x in old['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
changed_schemas=[x['path'] for x in old['sourceFiles'] if x['path'].startswith('contracts/generated/') and entry(x['path'])!=x]
assert set(changed_schemas)=={'contracts/generated/'+s+'.schema.json' for s in ['path-raster-request','path-raster-response','scene-raster-request','scene-raster-response','page-compile-response','page-raster-response']}
regressions['gradientRaster']={'report':entry(ROOT/'parity.json'),'batches':58}
preview=read(ROOT/'preview.json');assert entry(preview['path'])['sha256']==preview['sha256']
checks={'rustTests':360,'newRustTests':7,'nativeWasmLogicalBatches':7738,'previousLogicalBatchesRerun':7680,'newGradientBatches':58,'newRenderedFrames':42,'forgedComponentRejections':14,'componentRegressionBatches':197,'independentGradientPixels':159744,'oldRenderedFramesUnchanged':620,'runtimeSchemas':61,'changedSchemas':6,'strictClippy':True,'rustfmt':True,'typescript':True}
result={'format':'musteroffice.gradient-raster-verification/1','previousEvidence':entry(PREVIOUS),'checks':checks,
 'scope':'Evaluated world-space linear/circular radial gradient painting, not complete native PPTX spatial gradient semantics or imported-page rendering.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-old['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'environment':benchmark['environment'],'artifactSizeScope':'Uncompressed incomplete development artifacts, not a full kernel or Musterwork installer.',
 'regressionReports':regressions,'migrationAudit':audit,'gradientReport':parity,'independentReference':reference,'contracts':contracts,
 'componentReport':component,'componentBuilds':builds,'compiledClosure':closure,'benchmark':benchmark,'preview':preview,
 'geometryReferences':{name:entry(path) for name,path in [('paths','.codex-work/path-raster/reference.json'),('scenes','.codex-work/scene-raster/reference.json'),('pages','.codex-work/page-render/reference.json')]},
 'rustTestNames':tests,'sourceFiles':[entry(p) for p in sorted(sources)],'validationLogs':[entry(ROOT/p) for p in logs],
 'changedSchemas':changed_schemas,'dependencyChanges':{'externalRuntimeVersions':[],'cargoAndPnpmLocksUnchanged':True},
 'limitations':['The draft PathDraw/PathInstance contract now requires typed brush; C++ component ABI is 4. Legacy color fields/ABI are rejected explicitly.',
 'Gradient inputs are evaluated world-space lines/circles. Elliptical/rectangular/shape-path gradients and native PPTX gradient geometry/inheritance mapping remain pending.',
 'Native/WASM pixels are identical for the tested frames. Independent analytic reference permits one code value; this is not target-application visual acceptance or a global shader error proof.',
 'Only macOS arm64 Native and Node-hosted WASM are validated. SIMD float arithmetic is retained on Native. Windows/Linux/browser/GPU acceptance remains pending.',
 'ASan/UBSan passed; LeakSanitizer is unsupported on this macOS host and was disabled. This is not complete memory leak or hostile-input isolation certification.',
 'Local benchmark includes each selected host boundary and warm full-request work; no old-profile timing, RSS, full PPTX/page/product latency or installer measurement.',
 'Pattern/image/effect painting, complete native page text/geometry, advanced objects/playback, Office/WPS roundtrip, Agent services and Musterwork E0-E3 remain required.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
