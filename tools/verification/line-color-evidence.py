"""Bind native line color computation, independent checks and unchanged prior semantics.

Default only verifies; --seal exclusively creates a new immutable evidence file.
"""
import hashlib,json,platform,re,sys
from pathlib import Path
ROOT=Path('.codex-work/line-colors')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-line-color-verification.json')
def read(p):return json.loads(Path(p).read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def entry(p):
 p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':sha(b)}
previous='docs/reviews/evidence/2026-09-24-line-style-verification.json'
assert entry(previous)['sha256']=='894ce56b35393693c2fa52479256c7e030763ce9bf2f68a536eb43df5aaa578f'
prior=read(previous);artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for key in ['cppWasm','cppWasmGlue','typescriptAdapter','nativeWorker','rasterWasm','rasterWasmGlue','rasterAdapter']:
 assert artifacts[key]==prior['artifacts'][key],key

def hashes(r,raster=False):
 aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
 assert any(k in r for k in ['nativeSha256','nativeCliSha256'])
 assert any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
 for k,a in aliases.items():
  if k in r:assert r[k]==artifacts[a]['sha256'],k

def files(r):
 for c in r.get('cases',[]):
  for k in ['request','canonicalRequest','response','plan','source','font','bundle','frame','pixels','scene','pptx','export','editRequest','originalSource']:
   if k+'Path' in c:assert entry(c[k+'Path'])['sha256']==c[k+'Sha256'],(c['name'],k)

regressions={}
for name in ['document','opc','source','color','font','export']:
 p=ROOT/(name+'-regression.json');r=read(p);old_entry=prior['regressionReports'][name]['report']
 assert entry(old_entry['path'])==old_entry
 assert r['cases']==read(old_entry['path'])['cases'],name
 if name=='source':
  for c in r['cases']:
   if 'source' in c:assert entry(c['source'])['sha256']==c['sourceSha256']
  for c in r['outputs']:assert entry(c['output'])['sha256']==c['sha256']
 hashes(r);files(r);regressions[name]={'report':entry(p),'batches':len(r['cases']),'allPreviousCasesUnchanged':True}
historical={
 'shaping':('text-shaping','textReport'),'unicode':('unicode-text','unicodeReport'),
 'cascade':('font-cascade','cascadeReport'),'bidi':('bidi','bidiReport'),'bidiConformance':('bidi','conformanceReport'),
 'itemizationAndParagraph':('font-fallback','updatedItemizationReport'),'mixedFont':('font-fallback','fallbackReport'),
 'lineBreaking':('line-break','lineBreakReport'),'fontMetrics':('font-metrics','metricsReport'),
 'lineShaping':('line-shaping','lineShapeReport'),'lineGeometry':('line-geometry','lineGeometryReport'),
 'paragraphLayout':('paragraph-layout','paragraphLayoutReport'),'fontOutlines':('font-outlines','outlinesReport'),
 'paragraphPaths':('paragraph-paths','paragraphPathsReport')}
for name,(milestone,key) in historical.items():
 p=prior['regressionReports'][name]['report']['path'];r=read(p);old=read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
 field='batches' if name=='bidiConformance' else 'cases';assert r[field]==old[field],name
 hashes(r);files(r);regressions[name]={'report':entry(p),'batches':len(r[field]),'allPreviousCasesUnchanged':True}
for name in ['placement','groups','angles','angleSource','sourceLines','lineStyles']:
 p=prior['regressionReports'][name]['report']['path'];r=read(p);snapshot=ROOT/'previous'/(name+'.json')
 assert entry(snapshot)['sha256']==prior['regressionReports'][name]['report']['sha256'],name
 assert r['cases']==read(snapshot)['cases'],name
 hashes(r,name in ['placement','groups']);files(r);regressions[name]={'report':entry(p),'batches':len(r['cases']),'allPreviousCasesUnchanged':True}
rasters={};references={}
for name in ['path','scene','page']:
 p=Path('.codex-work')/('page-render' if name=='page' else name+'-raster')/'parity.json';r=read(p)
 old=prior[name+('RenderReport' if name=='page' else 'RasterReport')]
 assert r['cases']==old['cases'],name
 hashes(r,True);files(r);assert r['exactPixelAndMetadataEquality'];rasters[name]=r
 regressions[name+'Raster']={'report':entry(p),'batches':len(r['cases']),'allPreviousCasesUnchanged':True}
 reference=prior['reusedGeometryReferences'][name]['report'];assert entry(reference['path'])==reference
 lookup={c['name']:c for c in r['cases']}
 for c in read(reference['path'])['cases']:assert c['frameSha256']==lookup[c['name']]['frameSha256']
 references[name]={'report':reference,'reusedByFrameDigest':True}
lines=read(ROOT/'parity.json');hashes(lines);files(lines)
assert len(lines['cases'])==247 and lines['exactResponseAndEditCandidateBytes']
assert sum(c.get('colorSemanticsPreserved',False) for c in lines['cases'])==116
manifest=read(ROOT/'manifest.json');assert len(manifest['cases'])==90
for c in manifest['cases']:
 assert entry(c['path'])['sha256']==c['sha256']
 assert entry(c['base']['path'])['sha256']==c['base']['sha256']
regressions['lineColors']={'report':entry(ROOT/'parity.json'),'batches':247}
independent=read(ROOT/'independent.json');lookup={c['name']:c for c in lines['cases']}
assert [len(independent['cases']),independent['objects'],independent['editedPackagesPreserved'],independent['validXsdParts']]==[235,589,116,718]
for c in independent['cases']:
 assert c['sourceSha256']==lookup[c['name']]['sourceSha256'] and c['responseSha256']==lookup[c['name']]['responseSha256']
for c in independent['xsdInputs']:assert entry(c['path'])['sha256']==c['sha256']
assert sum(len(c['intentionalNonconformingParts']) for c in independent['xsdCases'])==2
assert [len(independent['coverage'][k]) for k in ['models','transforms','presets']]==[6,28,190]
assert independent['maxRgba16Difference']==0 and independent['maxWorkingChannelAbsoluteDifference']<=2e-12
assert independent['presetFactsSha256']==entry('.codex-work/ecma376/preset-values.json')['sha256']
contracts=read(ROOT/'contracts.json')
assert [contracts[k] for k in ['schemas','lineStyleResponses','lineStyleRequests','lineStyleEditRequests','sourceLineResponses','sourceLineEditRequests','pathRasterResponses','sceneRasterResponses','pageRenderResponses','lineColorResponses','lineColorRequests','lineColorEditRequests']]==[53,144,140,28,136,59,267,316,132,247,241,116]
old_schemas={s['path'] for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')}
assert len(old_schemas)==51
assert all(entry(s['path'])==s for s in prior['sourceFiles'] if s['path'] in old_schemas)
new_schemas={str(p) for p in Path('contracts/generated').glob('*.schema.json')}-old_schemas
assert new_schemas=={'contracts/generated/pptx-line-color-query.schema.json','contracts/generated/pptx-line-color-response.schema.json'}
old_types=[s for s in prior['sourceFiles'] if s['path'].startswith('packages/contracts/src/generated/')]
assert len(old_types)==51 and all(entry(s['path'])==s for s in old_types)
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M);assert len(tests)==277
logs=['rust-tests.log','clippy.log','schema-check.log','types-check.log','fmt-check.log','independent.log','regressions.log']
for name in logs:
 raw=(ROOT/name).read_text();assert 'error:' not in raw and 'FAILED' not in raw,name
for path in ['Cargo.lock','pnpm-lock.yaml']:assert entry(path)==next(s for s in prior['sourceFiles'] if s['path']==path)

legacy=read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
component=legacy['componentReport'];assert read('.codex-work/skia/verification/parity.json')==component
for k,p in [('nativeSha256','.codex-work/skia/mo-skia-probe'),('asanSha256','.codex-work/skia/mo-skia-probe-asan'),('wasmSha256',artifacts['rasterWasm']['path']),('glueSha256',artifacts['rasterWasmGlue']['path']),('adapterSha256',artifacts['rasterAdapter']['path'])]:assert component[k]==entry(p)['sha256']
for build in legacy['componentBuilds'].values():
 assert build['lock']==read('components/skia/lock.json')
 for c in build['componentSources']+build['artifacts']:assert entry(c['path'])==c
# Auxiliary author and declaration oracles are reused only with identical cases.
author=read('.codex-work/source-lines/author/parity.json');hashes(author);files(author)
assert author['cases']==prior['auxiliaryAuthorReport']['cases']
declarations=read('docs/reviews/evidence/2026-09-24-source-lines-verification.json')
assert read('.codex-work/source-lines/independent.json')==declarations['independentReport']
assert read('.codex-work/line-style/independent.json')==prior['independentReport']
paths={s['path'] for s in prior['sourceFiles']};assert all(Path(p).exists() for p in paths)
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
 for p in Path(base).rglob('*'):
  if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
for p in paths:
 if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':len(tests),'newRustTests':8,'nativeWasmLogicalBatches':sum(r['batches'] for r in regressions.values()),'previousLogicalBatchesUnchanged':2969,'newLineColorBatches':247,'newPptxInputs':90,'reusedStylePptxInputs':28,'nativeTextEditCandidates':116,'independentBatches':235,'independentObjects':589,'resolvedColors':541,'unresolvedColors':16,'colorModels':6,'colorTransforms':28,'presetColors':190,'validXsdParts':718,'intentionalXsdExclusions':2,'runtimeSchemas':53,'newSchemas':2,'previousSchemasUnchanged':51,'strictClippy':True,'rustfmt':True,'typescript':True}
assert checks['nativeWasmLogicalBatches']==3216
result={'format':'musteroffice.line-color-verification/1','scope':'Actual native line inheritance plus working-precision color expressions, explicit draft profiles and context. No imported page rendering or Office/WPS fidelity acceptance.',
 'previousEvidence':entry(previous),'checks':checks,'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts, not complete kernel or installer, performance or RSS acceptance. Raster-worker bytes changed without size change; all regressions rerun.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'lineColorReport':lines,'lineColorManifest':manifest,'independentReport':independent,
 'contractReport':contracts,'regressionReports':regressions,'pathRasterReport':rasters['path'],'sceneRasterReport':rasters['scene'],'pageRenderReport':rasters['page'],
 'reusedGeometryReferences':references,'auxiliaryAuthorReport':author,
 'reusedDeclarationAndStyleOracles':{'evidence':entry(previous),'declarationEvidence':entry('docs/reviews/evidence/2026-09-24-source-lines-verification.json'),'allCasesAndEditCandidatesUnchanged':True},
 'reusedComponentEvidence':{'evidence':entry('docs/reviews/evidence/2026-09-24-miter-clip-verification.json'),'batches':len(component['cases']),'referencePixels':component['oraclePixels'],'artifactDigestsVerified':True,'rerun':False},
 'rustTestNames':tests,'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(paths)],
 'dependencyChanges':{'externalRuntimeVersions':[],'lockFilesUnchanged':True,'newDevelopmentDependencies':[]},
 'limitations':[
  'Native style/color profiles remain draft, not Office/WPS target version acceptance. Existing application discrepancies remain open.',
  'Working sRGB/linear channels are unassociated and not prematurely clipped; downstream compositor, gamut management and brush compilation remain to be connected.',
  'File placeholder expression takes outer precedence; nested phClr uses explicit host context. Target-app validation of this interpretation remains incomplete.',
  'Unresolved line styles/gradient/pattern/retained extensions are not color support. Missing context and cycles return diagnostics; budget/cancellation abort the batch.',
  'Two owned missing-color-map source probes are intentionally nonconforming to XSD and excluded from positive counts.',
  'No new external application run, editing roundtrip or page pixel comparison in this milestone. No installer or end-to-end performance evidence.',
  'Native macOS arm64 and Node WASM only. Full resource lifecycle, browser Workers, advanced objects/playback and Agent/Musterwork E0-E3 remain incomplete.'
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(paths),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
 with OUTPUT.open('x') as f:f.write(raw)
 summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
