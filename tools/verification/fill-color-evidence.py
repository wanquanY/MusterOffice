"""Seal fill working colors against the immutable source-effect baseline."""
import hashlib,json,platform,re,subprocess,sys
from pathlib import Path
ROOT=Path('.codex-work/fill-colors');OUTPUT=Path('docs/reviews/evidence/2026-09-25-fill-color-verification.json')
read=lambda p:json.loads(Path(p).read_text())
def entry(p):
 p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
previous=Path('docs/reviews/evidence/2026-09-25-source-effect-verification.json')
assert entry(previous)['sha256']=='68328c078bb3e3d2dedba89b9b52fe7a9b6ac484e6f88793930f6be84bb22a9f'
prior=read(previous);artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['nativeWorker','nativeRasterWorker','cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[k]==prior['artifacts'][k],k

def files(value):
 if isinstance(value,list):
  for c in value:files(c)
 elif isinstance(value,dict):
  if {'path','sha256'}<=value.keys():
   actual=entry(value['path']);assert actual['sha256']==value['sha256'],value['path']
   if 'byteLength' in value:assert actual['byteLength']==value['byteLength']
  for key,path in value.items():
   if key.endswith('Path') and key[:-4]+'Sha256' in value:assert entry(path)['sha256']==value[key[:-4]+'Sha256'],path
   if isinstance(path,(dict,list)):files(path)
def artifact_hashes(report,raster=False):
 aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
 assert any(k in report for k in ['nativeSha256','nativeCliSha256']) and any(k in report for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
 for key,artifact in aliases.items():
  if key in report:assert report[key]==artifacts[artifact]['sha256'],key
 files(report.get('cases',report.get('batches')))
regressions={}
for name,record in prior['regressionReports'].items():
 snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==record['report']['sha256']
 path=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(record['report']['path'])
 report=read(path);old=read(snapshot);field='batches' if name=='bidiConformance' else 'cases'
 assert report[field]==old[field],name
 artifact_hashes(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster'])
 if name=='source':
  for c in report['cases']:
   if 'source' in c:assert entry(c['source'])['sha256']==c['sourceSha256']
  for c in report['outputs']:assert entry(c['output'])['sha256']==c['sha256']
 regressions[name]={'report':entry(path),'batches':len(report[field]),'previousResponsesRequestsCandidatesAndFramesUnchanged':True}
assert len(regressions)==39 and sum(r['batches'] for r in regressions.values())==7108
parity=read(ROOT/'parity.json');independent=read(ROOT/'independent.json');manifest=read(ROOT/'manifest.json')
artifact_hashes(parity);assert parity['exactResponseAndEditCandidateBytes']
assert len(parity['cases'])==572 and len(manifest['cases'])==129
assert sum(c.get('colorSemanticsPreserved',False) for c in parity['cases'])==263
assert [independent[k] for k in ['targets','colorSlots','resolvedColors','unresolvedColors','nativeContexts','editedPackagesPreserved','validXsdParts','intentionalNonconformingParts']]==[1267,1083,1033,50,168,263,1026,6]
assert [len(independent['coverage'][k]) for k in ['models','transforms','presets']]==[6,28,190]
assert independent['maxRgba16Difference']==0 and independent['maxWorkingChannelAbsoluteDifference']<=2e-12
lookup={c['name']:c for c in parity['cases']}
for c in independent['cases']:
 assert c['sourceSha256']==lookup[c['name']]['sourceSha256'] and c['responseSha256']==lookup[c['name']]['responseSha256']
for c in manifest['cases']:assert entry(c['path'])['sha256']==c['sha256'];files(c['base'])
files(independent['xsdInputs']);assert entry('.codex-work/ecma376/preset-values.json')['sha256']==independent['presetFactsSha256']
line_reference=read('.codex-work/line-colors/independent.json');assert [line_reference[k] for k in ['objects','resolvedColors','unresolvedColors','editedPackagesPreserved','validXsdParts']]==[589,541,16,116,718];files(line_reference['xsdInputs'])
contracts=read(ROOT/'contracts.json');assert [contracts[k] for k in ['schemas','fillColorResponses','fillColorRequests','fillColorEditRequests','lineColorResponses','lineColorRequests','lineColorEditRequests','sourceResponses','sourceRequests','fillStyleResponses']]==[61,572,566,263,247,241,116,73,95,230]
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==353
logs=['workspace-tests.log','clippy.log','fmt.log','schema-check.log','types-check.log','parity.log','independent.log','line-independent.log','regressions.log','native-build.log','wasm-build.log','bindgen.log']
for file in logs:
 raw=(ROOT/file).read_text();assert not any(s in raw for s in ['error:','FAILED','Traceback']),file
changed=[]
for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')]:
 old=[s for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(suffix)];assert len(old)==59
 for item in old:
  if entry(item['path'])!=item:changed.append(item['path'])
assert set(changed)=={prefix+name+suffix for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')] for name in ['pptx-color-response','pptx-line-color-response']}
for lock in ['Cargo.lock','pnpm-lock.yaml']:assert entry(lock)==next(s for s in prior['sourceFiles'] if s['path']==lock)
for reference in prior['reusedGeometryReferences'].values():files(reference)
component=read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
for build in component['componentBuilds'].values():
 assert build['lock']==read('components/skia/lock.json')
 for record in build['componentSources']+build['artifacts']:assert entry(record['path'])==record
assert read('.codex-work/skia/verification/parity.json')==component['componentReport']
sources={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
 for p in Path(base).rglob('*'):
  if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
 if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':353,'newRustTests':12,'nativeWasmLogicalBatches':7680,'previousLogicalBatchesRerun':7108,'newFillColorBatches':572,'ownedPptxInputs':129,'editedPptxCandidates':263,'independentColorSlots':1083,'resolvedColors':1033,'unresolvedColors':50,'nativePlaceholderContexts':168,'validXsdParts':1026,'intentionalNonconformingParts':6,'runtimeSchemas':61,'unchangedPreviousSchemas':57,'strictClippy':True,'rustfmt':True,'typescript':True}
regressions['fillColors']={'report':entry(ROOT/'parity.json'),'batches':572}
result={'format':'musteroffice.fill-color-verification/1','previousEvidence':entry(previous),'checks':checks,
 'scope':'Working colors for resolved solid, gradient-stop and pattern expressions, with lazy owner-native placeholder contexts. Not spatial brushes, image/effect evaluation or imported-page painting.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'python':platform.python_version(),'rust':subprocess.check_output(['rustc','--version'],text=True).strip(),'node':subprocess.check_output(['node','--version'],text=True).strip()},
 'artifactSizeScope':'Uncompressed incomplete development binaries; not a full kernel, installer, latency or memory acceptance.',
 'regressionReports':regressions,'fillColorReport':parity,'independentReport':independent,'legacyLineColorReference':entry('.codex-work/line-colors/independent.json'),'manifest':manifest,'contractReport':contracts,'rustTestNames':tests,
 'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(sources)],'reusedGeometryReferences':prior['reusedGeometryReferences'],'reusedComponentEvidence':{**prior['reusedComponentEvidence'],'rerun':False},'dependencyChanges':{'externalRuntimeVersions':[],'cargoAndPnpmLocksUnchanged':True},
 'limitations':['Native context lookup uses the fill expression owner and its local fillRef/lnRef/bgRef when phClr is reached; absent/native nested phClr uses explicit host context. This draft still requires target-application observations.',
 'Each gradient stop is evaluated in native order; no sorting, deduplication, spatial interpolation or pattern/image brush rasterization is claimed.',
 'Independent color values and contexts come from ZIP/XML and Decimal math. Fill selection is cross-checked with the existing fill query and prior independent fill evidence, not a second complete inheritance implementation.',
 'Two existing missing-map probes and four explicit unknown-attribute probes are intentionally XSD-nonconforming; counted separately from 1026 valid parts.',
 'The old line DOM oracle was updated for the previously introduced typed gradient/pattern declarations. Full-line queries still explicitly report unsupportedFill; production line responses remain byte-identical.',
 'No new Office/WPS/LibreOffice visual or full edit roundtrip. Complete brush/effect/image rendering, source page compilation, advanced objects/playback, Agent/Musterwork E0-E3 and resource/installer acceptance remain pending.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
 with OUTPUT.open('x') as f:f.write(raw)
 summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
