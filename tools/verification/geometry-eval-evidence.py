"""Seal actual native guide evaluation and preserve the prior behavior baseline."""
import hashlib
import json
import platform
import re
import sys
from pathlib import Path

ROOT=Path('.codex-work/geometry-eval')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-geometry-evaluation-verification.json')
def read(p):return json.loads(Path(p).read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':sha(b)}
previous='docs/reviews/evidence/2026-09-24-source-geometry-verification.json'
assert entry(previous)['sha256']=='d72a87306ebffaba4a085a3543cc69750cdbd073ce85e4a6b0c485b2b96535a3'
prior=read(previous);artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[k]==prior['artifacts'][k],k
def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in r:assert r[k]==artifacts[a]['sha256'],k
def files(r):
    for c in r.get('cases',[]):
        for k in ['request','canonicalRequest','response','plan','source','font','bundle','frame','pixels','scene','pptx','export','editRequest','originalSource']:
            if k+'Path' in c:assert entry(c[k+'Path'])['sha256']==c[k+'Sha256'],(c['name'],k)
regressions={}
for name,previous_report in prior['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==previous_report['report']['sha256']
    old=read(snapshot)
    p=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(previous_report['report']['path'])
    r=read(p);field='batches' if name=='bidiConformance' else 'cases'
    assert r[field]==old[field],name
    hashes(r,name in ['placement','groups','pathRaster','sceneRaster','pageRaster']);files(r)
    if name=='source':
        for c in r['cases']:
            if 'source' in c:assert entry(c['source'])['sha256']==c['sourceSha256']
        for c in r['outputs']:assert entry(c['output'])['sha256']==c['sha256']
    regressions[name]={'report':entry(p),'batches':len(r[field]),'allPreviousCasesUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==4039
geometry=read(ROOT/'parity.json');hashes(geometry);files(geometry)
assert geometry['exactResponseAndEditCandidateBytes'] and len(geometry['cases'])==1168
assert sum(c.get('geometryValuesPreserved',False) for c in geometry['cases'])==560
manifest=read(ROOT/'manifest.json');assert len(manifest['cases'])==602 and manifest['standardDefinitionVariants']==561
for c in manifest['cases']:
    assert entry(c['path'])['sha256']==c['sha256']
    assert entry(c['base']['path'])['sha256']==c['base']['sha256']
regressions['geometryEvaluation']={'report':entry(ROOT/'parity.json'),'batches':1168}
application=read(ROOT/'application-probes/observations.json')
application_queries=read(ROOT/'application-probes/parity.json');hashes(application_queries);files(application_queries)
assert len(application['cases'])==len(application_queries['cases'])==9 and application_queries['exactResponseBytes']
for c,q in zip(application['cases'],application_queries['cases']):
    assert (c['name'],c['sourceSha256'])==(q['name'],q['sourceSha256'])
    assert entry(c['sourcePath'])['sha256']==c['sourceSha256'] and entry(c['pdfPath'])['sha256']==c['pdfSha256']
assert sum(c['outcome']=='resolved' for c in application_queries['cases'])==3
application_contracts=read(ROOT/'application-contracts.json');assert [application_contracts[k] for k in ['geometryEvaluationResponses','geometryEvaluationRequests','schemas']]==[9,9,55]
application_xsd=read(ROOT/'application-xsd.json');assert len(application_xsd['cases'])==9 and application_xsd['xsdParts']==54
for c,q,o in zip(application_xsd['cases'],application_queries['cases'],application['cases']):
    assert c['sourceSha256']==q['sourceSha256'] and c['responseSha256']==q['responseSha256'] and c['pdfSha256']==o['pdfSha256']
for c in application_xsd['xsdInputs']:assert entry(c['path'])['sha256']==c['sha256']
regressions['geometryApplicationProbes']={'report':entry(ROOT/'application-probes/parity.json'),'batches':9}
independent=read(ROOT/'independent.json');lookup={c['name']:c for c in geometry['cases']}
assert [independent[k] for k in ['resolved','unresolved','numericValues','validXsdParts','editedPackagesPreserved']]==[560,42,38698,3611,560]
assert independent['precisionDigits']==80 and float(independent['maxToleranceRatio'])<=1
assert set(['*/','+-','+/','?:','abs','at2','cat2','cos','max','min','mod','pin','sat2','sin','sqrt','tan','val'])<=set(independent['formulaTokens'])
for c in independent['cases']:
    assert c['sourceSha256']==lookup[c['name']]['sourceSha256'] and c['responseSha256']==lookup[c['name']]['responseSha256']
for c in independent['xsdInputs']:assert entry(c['path'])['sha256']==c['sha256']
contracts=read(ROOT/'contracts.json')
assert [contracts[k] for k in ['schemas','geometryEvaluationResponses','geometryEvaluationRequests','geometryEvaluationEdits','sourceGeometryResponses','sourceGeometryEditRequests','lineColorResponses']]==[55,1168,1166,560,823,400,247]
for prefix,extension in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')]:
    old={s['path'] for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(extension)}
    assert len(old)==53
    assert all(entry(s['path'])==s for s in prior['sourceFiles'] if s['path'] in old)
    current={str(p) for p in Path(prefix).glob('*'+extension)}
    assert current-old=={prefix+'pptx-geometry-query'+extension,prefix+'pptx-geometry-response'+extension}
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M);assert len(tests)==292
logs=['rust-tests.log','clippy.log','schema-check.log','types-check.log','fmt-check.log','independent.log','regressions.log','contracts.log','application-parity.log']
for name in logs:
    raw=(ROOT/name).read_text();assert not any(s in raw for s in ['error:','FAILED','Traceback']),name
old_lock=next(s for s in prior['sourceFiles'] if s['path']=='Cargo.lock')
lock=Path('Cargo.lock').read_text();a=lock.index('name = "mo-pptx"');b=lock.index('[[package]]',a)
section=lock[a:b];assert section.count(' "libm",\n')==1
assert sha((lock[:a]+section.replace(' "libm",\n','')+lock[b:]).encode())==old_lock['sha256']
assert entry('pnpm-lock.yaml')==next(s for s in prior['sourceFiles'] if s['path']=='pnpm-lock.yaml')
author=read('.codex-work/source-lines/author/parity.json');hashes(author);files(author)
assert author['cases']==prior['auxiliaryAuthorReport']['cases']
assert read('.codex-work/source-geometry/independent.json')==prior['independentReport']
color_seal=read('docs/reviews/evidence/2026-09-24-line-color-verification.json')
assert read('.codex-work/line-colors/independent.json')==color_seal['independentReport']
references={}
for name,reference in prior['reusedGeometryReferences'].items():
    report=reference['report'];assert entry(report['path'])==report
    current=read(regressions[name+'Raster']['report']['path']);frames={c['name']:c for c in current['cases']}
    for c in read(report['path'])['cases']:assert c['frameSha256']==frames[c['name']]['frameSha256']
    references[name]=reference
legacy=read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json');component=legacy['componentReport']
assert read('.codex-work/skia/verification/parity.json')==component
for k,p in [('nativeSha256','.codex-work/skia/mo-skia-probe'),('asanSha256','.codex-work/skia/mo-skia-probe-asan'),('wasmSha256',artifacts['rasterWasm']['path']),('glueSha256',artifacts['rasterWasmGlue']['path']),('adapterSha256',artifacts['rasterAdapter']['path'])]:assert component[k]==entry(p)['sha256']
for build in legacy['componentBuilds'].values():
    assert build['lock']==read('components/skia/lock.json')
    for c in build['componentSources']+build['artifacts']:assert entry(c['path'])==c
paths={s['path'] for s in prior['sourceFiles']};assert all(Path(p).exists() for p in paths)
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':292,'newRustTests':7,'nativeWasmLogicalBatches':sum(r['batches'] for r in regressions.values()),'previousLogicalBatchesRerun':4039,
        'newGeometryBatches':1177,'newPptxInputs':611,'primaryPptxInputs':602,'applicationProbes':9,'standardDefinitionAspectVariants':561,'nativeTextEditCandidates':560,
        'independentNumericValues':38698,'primaryResolvedInputs':560,'primaryUnresolvedInputs':42,'applicationResolvedInputs':3,'applicationUnresolvedInputs':6,
        'formulaOperations':17,'validXsdParts':3611,'applicationXsdParts':54,'intentionalXsdExclusions':1,
        'runtimeSchemas':55,'newSchemas':2,'previousSchemasUnchanged':53,'strictClippy':True,'rustfmt':True,'typescript':True}
assert checks['nativeWasmLogicalBatches']==5216
result={'format':'musteroffice.geometry-evaluation-verification/1','scope':'Actual native custom geometry guide and coordinate evaluation with explicit draft binary64 rules. No preset expansion, shape-space path compilation, rendering or target application acceptance.',
        'previousEvidence':entry(previous),'checks':checks,'artifacts':artifacts,
        'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
        'artifactSizeScope':'Uncompressed incomplete development artifacts. Worker digests changed and their regressions reran. Not complete kernel, installer, RSS or performance acceptance.',
        'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
        'geometryEvaluationReport':geometry,'manifest':manifest,'independentReport':independent,'contractReport':contracts,'regressionReports':regressions,
        'applicationObservations':application,'applicationQueryReport':application_queries,'applicationContractReport':application_contracts,'applicationXsdReport':application_xsd,
        'reusedGeometryReferences':references,'auxiliaryAuthorReport':author,
        'reusedSourceGeometryOracle':{'evidence':entry(previous),'allCasesAndCandidateBytesUnchanged':True},
        'reusedComponentEvidence':{'evidence':entry('docs/reviews/evidence/2026-09-24-miter-clip-verification.json'),'batches':len(component['cases']),'referencePixels':component['oraclePixels'],'artifactDigestsVerified':True,'rerun':False},
        'rustTestNames':tests,'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(paths)],
        'dependencyChanges':{'externalRuntimeVersions':[],'newInternalDependencyEdge':'mo-pptx -> existing libm 0.2.16 force-soft-floats','pnpmLockUnchanged':True},
        'limitations':[
            'Draft numeric profile, not Office/WPS compatibility. Decimal tolerance checks mathematical computation and does not certify a rendered pixel error bound.',
            '42 inputs unresolved. Standard addenda includes over-arity formulas and names outside the documented builtin vocabulary; no silent token removal or invented values.',
            'Preset declarations remain explicitly unexpanded. Geometry inheritance, path-space scaling, arcs, fill defaults and page compilation remain separate unfinished work.',
            'Formula guide values use binary64 with pinned software libm; original XML and author lexical values remain unmodified. Guide results are not Q32 wire coordinates.',
            'Nine LibreOffice PDF observations reveal differences and are not Office/WPS acceptance, kernel frame comparison or editing roundtrip. Browser worker lifecycle, advanced objects/playback, Agent/Musterwork E0-E3 and installer performance remain incomplete.'
        ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(paths),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
