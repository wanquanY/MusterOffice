"""Bind draft line-style implementation, independent checks and prior regressions.

Default only verifies; --seal exclusively creates a new immutable evidence file.
"""
import hashlib,json,platform,re,sys
from pathlib import Path
ROOT=Path('.codex-work/line-style')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-line-style-verification.json')
def read(p):return json.loads(Path(p).read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def entry(p):
 p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':sha(b)}
previous='docs/reviews/evidence/2026-09-24-source-lines-verification.json'
assert entry(previous)['sha256']=='6557f54e335f80ef5228029f457166e1dbe07e7b50befdbce4281192217003e0'
prior=read(previous);artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for key in ['cppWasm','cppWasmGlue','typescriptAdapter','nativeRasterWorker','rasterWasm','rasterWasmGlue','rasterAdapter']:
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
for name in ['placement','groups','angles','angleSource','sourceLines']:
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
assert len(lines['cases'])==144 and lines['exactResponseAndEditCandidateBytes']
assert sum(c.get('lineSemanticsPreserved',False) for c in lines['cases'])==28
manifest=read(ROOT/'manifest.json');assert len(manifest['cases'])==28
assert entry(manifest['base']['path'])['sha256']==manifest['base']['sha256']
for c in manifest['cases']:assert entry(c['path'])['sha256']==c['sha256']
regressions['lineStyles']={'report':entry(ROOT/'parity.json'),'batches':144}
independent=read(ROOT/'independent.json');lookup={c['name']:c for c in lines['cases']}
assert [len(independent['cases']),independent['objects'],independent['editedPackagesPreserved'],independent['validXsdParts']]==[118,124,28,224]
for c in independent['cases']:
 assert c['sourceSha256']==lookup[c['name']]['sourceSha256'] and c['responseSha256']==lookup[c['name']]['responseSha256']
for c in independent['xsdInputs']:assert entry(c['path'])['sha256']==c['sha256']
assert sum(len(c['intentionalNonconformingParts']) for c in independent['xsdCases'])==1
observations=read(ROOT/'libreoffice.json');assert len(observations['cases'])==24
fixtures={c['name']:c for c in manifest['cases']}
for c in observations['cases']:
 assert c['sourceSha256']==fixtures[c['name']]['sha256'];assert entry(c['pdfPath'])['sha256']==c['pdfSha256']
# These are recorded differences to investigate, not successes to count as fidelity.
obs={c['name']:c for c in observations['cases']}
assert not obs['parent-direct-before-parent-style']['strokes']
assert not obs['theme-override-inherited-reference']['strokes']
assert obs['empty-dash-block']['strokes'][0]['dash']==obs['theme']['strokes'][0]['dash']
assert obs['theme-no-color']['strokes'][0]['width']==0

contracts=read(ROOT/'contracts.json')
assert [contracts[k] for k in ['schemas','lineStyleResponses','lineStyleRequests','lineStyleEditRequests','sourceLineResponses','sourceLineEditRequests','pathRasterResponses','sceneRasterResponses','pageRenderResponses']]==[51,144,140,28,136,59,267,316,132]
old_schemas={s['path'] for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')}
assert len(old_schemas)==49
assert all(entry(s['path'])==s for s in prior['sourceFiles'] if s['path'] in old_schemas)
new_schemas={str(p) for p in Path('contracts/generated').glob('*.schema.json')}-old_schemas
assert new_schemas=={'contracts/generated/pptx-line-query.schema.json','contracts/generated/pptx-line-response.schema.json'}
old_types=[s for s in prior['sourceFiles'] if s['path'].startswith('packages/contracts/src/generated/')]
assert len(old_types)==49 and all(entry(s['path'])==s for s in old_types)
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M);assert len(tests)==269
logs=['rust-tests.log','clippy.log','schema-check.log','types-check.log','fmt-check.log','independent.log','more-regressions.log','regressions.log','observations.log']
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
assert author['cases']==prior['authorReport']['cases']
assert read('.codex-work/source-lines/independent.json')==prior['independentReport']
paths={s['path'] for s in prior['sourceFiles']};assert all(Path(p).exists() for p in paths)
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
 for p in Path(base).rglob('*'):
  if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
for p in paths:
 if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':len(tests),'newRustTests':8,'nativeWasmLogicalBatches':sum(r['batches'] for r in regressions.values()),'previousLogicalBatchesUnchanged':2825,'newLineStyleBatches':144,'newPptxInputs':28,'nativeTextEditCandidates':28,'independentBatches':118,'independentObjects':124,'validXsdParts':224,'intentionalXsdExclusions':1,'libreOfficeObservationFiles':24,'runtimeSchemas':51,'newSchemas':2,'previousSchemasUnchanged':49,'strictClippy':True,'rustfmt':True,'typescript':True}
assert checks['nativeWasmLogicalBatches']==2969
result={'format':'musteroffice.line-style-verification/1','scope':'Explicit draft native line style query and provenance. No RGB evaluation, imported page rendering or Office/WPS fidelity acceptance.',
 'previousEvidence':entry(previous),'checks':checks,'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts, not complete kernel or installer, performance or RSS acceptance. Native text-worker bytes changed without size change; all regressions rerun.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'lineStyleReport':lines,'lineStyleManifest':manifest,'independentReport':independent,'applicationObservations':observations,
 'contractReport':contracts,'regressionReports':regressions,'pathRasterReport':rasters['path'],'sceneRasterReport':rasters['scene'],'pageRenderReport':rasters['page'],
 'reusedGeometryReferences':references,'auxiliaryAuthorReport':author,
 'reusedDeclarationOracle':{'evidence':entry(previous),'allCasesAndEditCandidatesUnchanged':True},
 'reusedComponentEvidence':{'evidence':entry('docs/reviews/evidence/2026-09-24-miter-clip-verification.json'),'batches':len(component['cases']),'referencePixels':component['oraclePixels'],'artifactDigestsVerified':True,'rerun':False},
 'rustTestNames':tests,'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(paths)],
 'dependencyChanges':{'externalRuntimeVersions':[],'lockFilesUnchanged':True,'newDevelopmentDependencies':[]},
 'limitations':[
  'Resolved means native style declarations/defaults with provenance; native color expressions retain their transforms and are not final RGBA or brush values.',
  'Named Microsoft-rule interpretation remains draft. Zero reference, parent style and placeholder color precedence need target Office/WPS verification.',
  'LibreOffice observations differ on empty custom dash lists, inherited style references, omitted width/color and placeholder color context; no application fidelity passed.',
  'Unknown line/reference retained content and selected gradient/pattern fills are unresolved, not silently discarded. Full style families remain incomplete.',
  'No page compiler wiring, line mutation, WPS/PowerPoint run or application editing roundtrip in this milestone.',
  'Native macOS arm64 and Node WASM only. Full resource lifecycle, browser Workers, advanced objects/playback and Agent/Musterwork E0-E3 remain incomplete.'
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(paths),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
 with OUTPUT.open('x') as f:f.write(raw)
 summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
