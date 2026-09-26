"""Seal native effect declarations after source, query and artifact regression checks."""
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path
from effect_regression import compare_records, compare_fill_queries
ROOT=Path('.codex-work/source-effects')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-source-effect-verification.json')
read=lambda p:json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
previous=Path('docs/reviews/evidence/2026-09-25-fill-style-verification.json')
assert entry(previous)['sha256']=='30d34732aadf6e2a432606b1b30b9b5fd1943b1c8815a47359a1d881b0c77f9e'
prior=read(previous);artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['nativeWorker','nativeRasterWorker','rustWasmGlue','cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[k]==prior['artifacts'][k],k

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
    assert any(k in report for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in report for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for key,artifact in aliases.items():
        if key in report:assert report[key]==artifacts[artifact]['sha256'],key
    files(report.get('cases',report.get('batches')))
regressions={};query_changes=[]
for name,record in prior['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==record['report']['sha256']
    path=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(record['report']['path'])
    report=read(path);old=read(snapshot);field='batches' if name=='bidiConformance' else 'cases'
    if name=='fillStyles':
        query_changes=compare_fill_queries(report[field],old[field],ROOT/'previous/fill-style-responses');changes=len(query_changes)
    else:changes=compare_records(report[field],old[field])
    artifact_hashes(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster'])
    if name=='source':
        for c in report['cases']:
            if 'source' in c:assert entry(c['source'])['sha256']==c['sourceSha256']
        for c in report['outputs']:assert entry(c['output'])['sha256']==c['sha256']
    regressions[name]={'report':entry(path),'batches':len(report[field]),'previousRequestsCandidatesAndFramesUnchanged':True,'verifiedEvolvedResponseRecords':changes}
assert len(regressions)==37 and sum(r['batches'] for r in regressions.values())==6753
assert len(query_changes)==167
reasons=[k for c in query_changes for k in c['verifiedChanges']]
assert {k:reasons.count(k) for k in set(reasons)}=={'known-empty-background-list':167,'typed-image-effect-diagnostic':2,'remaining-image-extension-diagnostic':1}
parity=read(ROOT/'parity.json');queries=read(ROOT/'fill-parity.json');independent=read(ROOT/'independent.json');manifest=read(ROOT/'manifest.json')
artifact_hashes(parity);artifact_hashes(queries);assert parity['exactIndexAndCandidateBytes']
assert len(parity['cases'])==247 and len(queries['cases'])==108 and len(manifest['cases'])==139 and len(independent['cases'])==108
assert sum(c.get('fillDeclarationsPreserved',False) for c in parity['cases'])==108
assert [independent[k] for k in ['declarations','effectNodes','validXsdParts','negativeXsdCases','semanticRangeRejections']]==[3793,287,862,30,1]
lookup={c['name']:c for c in parity['cases']}
for c in independent['cases']:assert c['sourceSha256']==lookup['inspect-'+c['name']]['sourceSha256']
for c in manifest['cases']:assert entry(c['path'])['sha256']==c['sha256']
files(independent['xsdInputs']);kinds=set()
for c in parity['cases']:
    response=read(c['responsePath'])
    for family in ['surfaces','themes']:
        for part in response.get('index',{}).get(family,{}).values():
            kinds|={n['definition']['kind'] for n in part.get('effectNodes',{}).values()}
assert len(kinds)==30
contracts=read(ROOT/'contracts.json')
assert [contracts[k] for k in ['schemas','sourceEffectResponses','sourceEffectEditRequests','fillStyleResponses','fillStyleRequests','fillStyleEditRequests','fillStyleInspections','sourceFillResponses','sourceFillEditRequests','sourceResponses','sourceRequests']]==[59,247,108,338,333,28,28,287,125,73,95]
source_reference=read('.codex-work/source-fills/independent.json');assert [source_reference[k] for k in ['fillDeclarations','validXsdParts','negativeXsdCases','semanticRangeRejections']]==[3639,999,36,1];files(source_reference['xsdInputs'])
fill_reference=read('.codex-work/fill-resolution/independent.json');assert [fill_reference[k] for k in ['origins','colorContexts','authoredExpectations','validXsdParts']]==[764,37,56,223];files(fill_reference['xsdInputs'])
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'workspace-tests.log').read_text(),re.M);assert len(tests)==341
logs=['workspace-tests.log','clippy.log','fmt.log','schema-check.log','types-check.log','parity.log','independent.log','source-independent.log','fill-independent.log','regressions.log','native-build.log','wasm-build.log','bindgen.log']
for file in logs:
    raw=(ROOT/file).read_text();assert not any(s in raw for s in ['error:','FAILED','Traceback']),file
changed=[]
for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')]:
    old=[s for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(suffix)];assert len(old)==59
    for item in old:
        if entry(item['path'])!=item:changed.append(item['path'])
assert set(changed)=={prefix+name+suffix for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')] for name in ['pptx-source-response','pptx-fill-response']}
for lock in ['Cargo.lock','pnpm-lock.yaml']:assert entry(lock)==next(s for s in prior['sourceFiles'] if s['path']==lock)
for reference in prior['reusedGeometryReferences'].values():files(reference)
component=read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
for build in component['componentBuilds'].values():
    assert build['lock']==read('components/skia/lock.json')
    for record in build['componentSources']+build['artifacts']:assert entry(record['path'])==record
assert read('.codex-work/skia/verification/parity.json')==component['componentReport']
sources={s['path'] for s in prior['sourceFiles']}
moves={f'crates/mo-pptx/src/source/fill/{name}.rs':f'crates/mo-pptx/src/source/paint/{name}.rs' for name in ['frame','read']}
for old,new in moves.items():
    assert old in sources and not Path(old).exists() and Path(new).is_file();sources.remove(old)
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':341,'newRustTests':13,'nativeWasmLogicalBatches':7108,'previousLogicalBatchesRerun':6753,'newSourceEffectBatches':247,'newFillQueryBatches':108,'ownedPptxInputs':139,'editedPptxCandidates':108,'nativeEffectKinds':30,'independentDeclarations':3793,'independentEffectNodes':287,'validXsdParts':862,'negativeXsdCases':30,'semanticRangeRejections':1,'runtimeSchemas':59,'unchangedPreviousSchemas':57,'strictClippy':True,'rustfmt':True,'typescript':True}
regressions['sourceEffects']={'report':entry(ROOT/'parity.json'),'batches':247}
regressions['effectFillQueries']={'report':entry(ROOT/'fill-parity.json'),'batches':108}
result={'format':'musteroffice.source-effect-verification/1','previousEvidence':entry(previous),'checks':checks,
 'scope':'Native effect declarations and physical bindings, flat part catalogs, lossless text edits, explicit empty background list handling. Not effect inheritance, graph linking, brush/effect pixels or imported-page rendering.',
 'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'python':platform.python_version(),'rust':subprocess.check_output(['rustc','--version'],text=True).strip(),'node':subprocess.check_output(['node','--version'],text=True).strip()},
 'artifactSizeScope':'Uncompressed incomplete development binaries; not a full kernel, installer, latency or memory acceptance.',
 'regressionReports':regressions,'effectReport':parity,'effectFillReport':queries,'independentReport':independent,'sourceDeclarationReference':entry('.codex-work/source-fills/independent.json'),'fillProvenanceReference':entry('.codex-work/fill-resolution/independent.json'),'verifiedQueryChanges':query_changes,'manifest':manifest,'contractReport':contracts,'rustTestNames':tests,
 'nativeEffectKinds':sorted(kinds),'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(sources)],'sourceReaderMoves':moves,'reusedGeometryReferences':prior['reusedGeometryReferences'],'reusedComponentEvidence':{**prior['reusedComponentEvidence'],'rerun':False},'dependencyChanges':{'externalRuntimeVersions':[],'cargoAndPnpmLocksUnchanged':True},
 'limitations':['Only native declarations and grammar are typed. Named graph references, cycles, context inheritance/default resolution and effect pixel evaluation remain pending.',
 'Effect lists require the standard default effect graph; XML child order is not a complete rendering pipeline. Empty DAGs are not assumed to be identity effects.',
 'Three-dimensional style properties and unknown properties/extensions are retained source bindings, not implemented 3D rendering.',
 'Color expressions keep native context owners; lazy phClr substitution, complete brushes, image resource decoding and full imported-page painting remain pending.',
 'No new Office/WPS/LibreOffice observation or full application edit roundtrip. Full advanced objects, playback, Agent/Musterwork E0-E3, resource/performance and installer acceptance remain incomplete.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
