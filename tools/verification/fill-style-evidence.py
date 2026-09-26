"""Seal fill inheritance evidence only after exact old behavior is rechecked."""
import copy
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path
ROOT=Path('.codex-work/fill-resolution')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-fill-style-verification.json')
def read(p): return json.loads(Path(p).read_text())
def sha(b): return hashlib.sha256(b).hexdigest()
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':sha(b)}
previous=Path('docs/reviews/evidence/2026-09-24-source-fill-verification.json')
assert entry(previous)['sha256']=='482c5a161471031647014be9b8b45b2972e5fbd4d5af103eb08ec464c7726a29'
prior=read(previous);artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']: assert artifacts[k]==prior['artifacts'][k],k
def files(value):
    if isinstance(value,list):
        for c in value: files(c)
    elif isinstance(value,dict):
        if {'path','sha256'}<=value.keys():
            actual=entry(value['path']);assert actual['sha256']==value['sha256']
            if 'byteLength' in value: assert actual['byteLength']==value['byteLength']
        for key,path in value.items():
            if key.endswith('Path') and key[:-4]+'Sha256' in value: assert entry(path)['sha256']==value[key[:-4]+'Sha256'],path
            if isinstance(path,(dict,list)): files(path)
def artifact_hashes(report,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker','wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in report for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in report for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for key,artifact in aliases.items():
        if key in report: assert report[key]==artifacts[artifact]['sha256'],key
    files(report.get('cases',report.get('batches')))
def previous_index(response):
    r=copy.deepcopy(response);index=r.get('index',r)
    for surface in index.get('surfaces',{}).values():
        for obj in surface['objects']: obj.pop('useBackgroundFill',None)
    return r
def compare(current,old):
    assert len(current)==len(old);changed=0
    for a,b in zip(current,old):
        if a==b: continue
        assert a.keys()==b.keys(),a.get('name');normalized=copy.deepcopy(a)
        if 'response' in normalized: normalized['response']=previous_index(normalized['response'])
        if 'responseSha256' in normalized and normalized['responseSha256']!=b['responseSha256']:
            value=previous_index(read(a['responsePath']));raw=json.dumps(value,ensure_ascii=False,separators=(',',':')).encode()
            assert sha(raw)==b['responseSha256'],(a['name'],sha(raw),b['responseSha256'])
            normalized['responseSha256']=b['responseSha256']
        assert normalized==b,a.get('name');changed+=1
    return changed
regressions={}
for name,record in prior['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==record['report']['sha256']
    path=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(record['report']['path'])
    report=read(path);old=read(snapshot);field='batches' if name=='bidiConformance' else 'cases'
    changes=compare(report[field],old[field]);artifact_hashes(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster'])
    if name=='source':
        for c in report['cases']:
            if 'source' in c: assert entry(c['source'])['sha256']==c['sourceSha256']
        for c in report['outputs']: assert entry(c['output'])['sha256']==c['sha256']
    regressions[name]={'report':entry(path),'batches':len(report[field]),'previousSemanticsAndArtifactsUnchanged':True,'sourceBackgroundFlagAdditionsOnly':changes}
assert len(regressions)==36 and sum(r['batches'] for r in regressions.values())==6523
parity=read(ROOT/'parity.json');independent=read(ROOT/'independent.json');manifest=read(ROOT/'manifest.json')
artifact_hashes(parity);assert parity['exactResponseAndCandidateBytes']
assert len(parity['cases'])==230 and len(manifest['cases'])==28 and len(independent['cases'])==28
assert sum(c.get('fillStylesPreserved',False) for c in parity['cases'])==28
assert [independent[k] for k in ['origins','colorContexts','authoredExpectations','validXsdParts']]==[740,13,56,223]
lookup={c['name']:c for c in parity['cases']}
for c in independent['cases']: assert c['sourceSha256']==lookup['owned-'+c['name']]['sourceSha256']
for c in manifest['cases']: assert entry(c['path'])['sha256']==c['sha256']
files(independent['xsdInputs'])
contracts=read(ROOT/'contracts.json');assert [contracts[k] for k in ['schemas','fillStyleResponses','fillStyleRequests','fillStyleEditRequests','fillStyleInspections']]==[59,230,225,28,28]
source_reference=read('.codex-work/source-fills/independent.json');assert [source_reference[k] for k in ['fillDeclarations','validXsdParts','negativeXsdCases','semanticRangeRejections']]==[3639,999,36,1];files(source_reference['xsdInputs'])
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==328
logs=['workspace-tests.log','clippy.log','fmt.log','schema-check.log','types-check.log','parity.log','independent.log','source-independent.log','regressions.log','native-build.log','wasm-build.log','bindgen.log']
for file in logs:
    raw=(ROOT/file).read_text();assert not any(s in raw for s in ['error:','FAILED','Traceback']),file
changed=[]
for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')]:
    old=[s for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(suffix)];assert len(old)==57
    for item in old:
        if entry(item['path'])!=item: changed.append(item['path'])
assert set(changed)=={'contracts/generated/pptx-source-response.schema.json','packages/contracts/src/generated/pptx-source-response.ts'}
for lock in ['Cargo.lock','pnpm-lock.yaml']: assert entry(lock)==next(s for s in prior['sourceFiles'] if s['path']==lock)
for reference in prior['reusedGeometryReferences'].values(): files(reference)
component=read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
for build in component['componentBuilds'].values():
    assert build['lock']==read('components/skia/lock.json')
    for record in build['componentSources']+build['artifacts']: assert entry(record['path'])==record
assert read('.codex-work/skia/verification/parity.json')==component['componentReport']
sources={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']: assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':328,'newRustTests':15,'nativeWasmLogicalBatches':6753,'previousLogicalBatchesRerun':6523,'newFillStyleBatches':230,'ownedPptxInputs':28,'editedPptxCandidates':28,'provenanceOrigins':740,'colorContexts':13,'authoredExpectations':56,'validXsdParts':223,'runtimeSchemas':59,'unchangedPreviousSchemas':56,'strictClippy':True,'rustfmt':True,'typescript':True}
regressions['fillStyles']={'report':entry(ROOT/'parity.json'),'batches':230}
result={'format':'musteroffice.fill-style-verification/1','previousEvidence':entry(previous),'checks':checks,
 'scope':'Draft native fill inheritance and provenance for objects, pictures, lines, root groups and backgrounds. No resolved color/image/effect brushes or imported-page rendering.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'python':platform.python_version(),'rust':subprocess.check_output(['rustc','--version'],text=True).strip(),'node':subprocess.check_output(['node','--version'],text=True).strip()},
 'artifactSizeScope':'Uncompressed incomplete development binaries; not a full kernel, installer, latency or memory acceptance.',
 'regressionReports':regressions,'fillReport':parity,'independentReport':independent,'sourceDeclarationReference':entry('.codex-work/source-fills/independent.json'),'manifest':manifest,'contractReport':contracts,'rustTestNames':tests,
 'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(sources)],'reusedGeometryReferences':prior['reusedGeometryReferences'],'reusedComponentEvidence':{**prior['reusedComponentEvidence'],'rerun':False},'dependencyChanges':{'externalRuntimeVersions':[],'cargoAndPnpmLocksUnchanged':True},
 'limitations':['The named Microsoft-2024 fill interpretation remains draft; no new Office/WPS/LibreOffice observation or application edit roundtrip.',
 'Color expressions retain native context owners. Lazy phClr substitution, complete brush sampling, image relationships/decoding and effects remain pending.',
 'Background effects including empty effectLst are currently retained and yield unresolved; effect declaration separation and evaluation remain pending. Black-white output modes are not evaluated.',
 'No source fill mutation API or full imported-page painting. Legacy full-line query still reports gradient/pattern as unsupported while the fill-only query resolves their declarations.',
 'Source response adds useBackgroundFill only. Complete text/image/effect import, preset expansion, advanced objects, playback, Agent/Musterwork E0-E3 and performance/installer acceptance remain incomplete.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
