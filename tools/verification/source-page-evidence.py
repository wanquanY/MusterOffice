"""Seal native static page computation, strict historical regression and scope."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
ROOT=Path('.codex-work/source-page');PREVIOUS=Path('docs/reviews/evidence/2026-09-25-source-placement-verification.json');OUTPUT=Path('docs/reviews/evidence/2026-09-25-source-page-verification.json')
read=lambda p:json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
assert entry(PREVIOUS)['sha256']=='8d74af2791387928f8997407563c2c4088065364a69080ed0d198f40e029bc3f'
old=read(PREVIOUS);artifacts={k:entry(v['path']) for k,v in old['artifacts'].items()}
for key in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[key]==old['artifacts'][key]
assert entry('pnpm-lock.yaml')==next(x for x in old['sourceFiles'] if x['path']=='pnpm-lock.yaml')
lock=Path('Cargo.lock').read_text();start=lock.index('name = "mo-raster-worker"');end=lock.index('\n[[package]]',start)
block=lock[start:end];assert block.count(' "mo-raster",\n')==1
prior_lock=lock[:start]+block.replace(' "mo-raster",\n','')+lock[end:]
assert hashlib.sha256(prior_lock.encode()).hexdigest()==next(x['sha256'] for x in old['sourceFiles'] if x['path']=='Cargo.lock')
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
audit=read(ROOT/'regression-audit.json');assert audit['previousEvidenceSha256']==entry(PREVIOUS)['sha256'];assert (audit['unchangedBatches'],audit['migratedBatches'])==(7782,1619)
assert audit['additions']=={'hidden':0,'showMasterShapes':0,'textBodyOrdinal':10916,'visualIssues':7}
regressions={}
for name,info in audit['reports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==old['regressionReports'][name]['report']['sha256']
    report=read(info['reportPath']);current_artifacts(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster','gradientRaster','presetExpansion','sourcePlacement'])
    field='batches' if name=='bidiConformance' else 'cases';bound_files(report[field])
    regressions[name]={'report':entry(info['reportPath']),'batches':len(report[field]),'migrationAudit':info}
assert len(regressions)==43 and sum(r['batches'] for r in regressions.values())==9401
parity=read(ROOT/'parity.json');current_artifacts(parity,True);bound_files(parity);assert parity['exactNativeWasmResponsesAndPixels']
assert {k:sum(c['kind']==k for c in parity['cases']) for k in ['source','compile','raster']}=={'source':28,'compile':36,'raster':36}
assert sum('pixelsPath' in c for c in parity['cases'])==21 and len(parity['malformed'])==2
reference=read(ROOT/'reference.json');source=read(ROOT/'source-check.json');contracts=read(ROOT/'contracts-combined.log');cli=read(ROOT/'cli.json')
bound_files(reference);bound_files(source);current_artifacts(cli,True);assert cli['parityReportSha256']==entry(ROOT/'parity.json')['sha256'];assert len(cli['cases'])==5
assert len(reference['cases'])==10 and reference['pixelsChecked']==3461792
assert len(reference['libreOffice']['cases'])==10 and sum(c['matchesOracleAtTolerance1'] for c in reference['libreOffice']['cases'])==9
assert source['counts']=={'surfaces':168,'objects':238,'explicitVisibility':7,'textBodies':85,'visualLocations':2,'validChangedParts':82}
assert len(source['invalidChangedParts'])==2
assert (contracts['schemas'],contracts['sourcePageResponses'],contracts['sourcePageRequests'])==(66,100,66)
assert (contracts['sourceResponses'],contracts['sourceRequests'],contracts['pagePlacementResponses'])==(73,95,64)
assert (contracts['sourcePlacementResponses'],contracts['sourcePlacementRequests'],contracts['fillColorResponses'])==(334,184,572)
by_key={(c['name'],c['kind']):c for c in parity['cases']}
for c in source['cases']:assert c['responseSha256']==by_key[(c['name'],'source')]['responseSha256']
for c in reference['cases']:assert c['pixelsSha256']==by_key[(c['name'],'raster')]['pixelsSha256']
author=read('.codex-work/page-placement/reference.json');assert author['counts']=={'documents':52,'objects':8807,'coefficients':52842,'sourceControlPoints':44042}
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==384
logs=['workspace-tests.log','clippy.log','fmt.log','build-native.log','build-wasm.log','wasm-bindgen.log','schema-check.log','types-check.log','parity.log','reference.log','source-check.log','cli.log','regressions.log','regression-audit.log','author-reference.log','contracts-combined.log','libreoffice-convert.log']
for name in logs:
    t=(ROOT/name).read_text();assert not any(x in t for x in ['error:','FAILED','Traceback','AssertionError']),name
changed=[x['path'] for x in old['sourceFiles'] if x['path'].startswith('contracts/generated/') and entry(x['path'])!=x]
assert changed==['contracts/generated/pptx-fill-color-response.schema.json','contracts/generated/pptx-source-response.schema.json']
sources={x['path'] for x in old['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.xml','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
environment=json.loads(subprocess.check_output(['node','--input-type=module','-e','import os from "node:os";console.log(JSON.stringify({platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version}));'],text=True));assert environment==old['environment']
regressions['sourcePage']={'report':entry(ROOT/'parity.json'),'batches':100}
result={'format':'musteroffice.source-page-verification/1','previousEvidence':entry(PREVIOUS),
 'checks':{'rustTests':384,'newRustTests':13,'nativeWasmLogicalBatches':9501,'previousLogicalBatchesRerun':9401,'newSourcePageBatches':100,'newRenderedFrames':21,'nativeInputPackages':28,'runtimeSchemas':66,'changedExistingSchemas':2,'newSchemas':3,'strictClippy':True,'rustfmt':True,'typescript':True,'cliPublicationChecks':5,'workerFrameRejections':2},
 'scope':'Native immutable PPTX to static solid page compilation and CPU raster. Physical drawing contexts, visibility and source bindings preserved. Unsupported visible content fails atomically; full source pages and Musterwork integration remain incomplete.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-old['artifacts'][k]['byteLength'] for k,v in artifacts.items()},'artifactSizeScope':'Uncompressed incomplete development artifacts, not a complete kernel or Musterwork installer. No new product latency/RSS measurements.',
 'environment':environment,'regressionReports':regressions,'migrationAudit':audit,'pageReport':parity,'independentReference':reference,'sourceReference':source,'authorReference':author,'contracts':contracts,'cliPublication':cli,
 'rustTestNames':tests,'sourceFiles':[entry(p) for p in sorted(sources)],'validationLogs':[entry(ROOT/p) for p in logs],'changedSchemas':changed,'newSchemas':['contracts/generated/pptx-page-request.schema.json','contracts/generated/pptx-page-compile-response.schema.json','contracts/generated/pptx-page-raster-response.schema.json'],
 'dependencyChanges':{'externalRuntimeVersions':[],'pnpmLockUnchanged':True,'cargoLockChange':'Only the mo-raster-worker -> existing mo-raster workspace dependency edge; removing this edge reproduces the previous lock digest.'},
 'limitations':['The static solid page profile is draft, not complete PPTX rendering or an Office/WPS compatibility certification.',
 'Text bodies, pictures, graphic frames, visible effects/references, spatial native brushes, miter/dashed/compound/arrow lines and advanced content are not connected to this page compiler. Unsupported visible content produces an error.',
 'A static color view does not implement black/white output, timing, transitions or interaction. Those phase-one commitments remain pending.',
 'Coordinate bounds start at binary64 guide results, not exact upstream formula semantics; they do not prove pixel coverage, color fidelity or target application appearance.',
 'The independent pixel oracle is limited to opaque axis-aligned rectangle interiors, excluding edges. Ten LibreOffice PDF observations include one unresolved useBgFill discrepancy.',
 'Microsoft Office and WPS were not validated in this stage; GUI tooling lacked Accessibility/Screen Recording grants and was stopped. Prior WPS source-transform differences remain open.',
 'There is no editing/save/reopen application acceptance. Raw XML bindings and 82 valid changed PML parts do not certify every native package feature.',
 'Queries have per-layer work budgets (at most three layers plus background); page object/path/device budgets are separate. Persistent caches and production cancellation latency are pending.',
 'Native tests are macOS arm64, WASM tests use Node. Browser/other-platform workers, production memory/latency/installer gates, full editable export/playback, Agent services and Musterwork E0-E3 remain incomplete.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':result['checks'],'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
