"""Bind current source, artifacts, real path checks and unchanged prior behavior."""
import hashlib,json,re,sys,platform,subprocess
from pathlib import Path
ROOT=Path('.codex-work/native-paths')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-native-path-verification.json')
def read(path):return json.loads(Path(path).read_text())
def sha(raw):return hashlib.sha256(raw).hexdigest()
def entry(path):
    path=Path(path);raw=path.read_bytes();return {'path':str(path),'byteLength':len(raw),'sha256':sha(raw)}
previous=Path('docs/reviews/evidence/2026-09-24-geometry-evaluation-verification.json')
assert entry(previous)['sha256']=='c1670f1ddbfb2b023ba1125c2b04000abebfc7dbc3478f8cdb50a564d09b7742'
prior=read(previous);artifacts={key:entry(item['path']) for key,item in prior['artifacts'].items()}
for key in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:assert artifacts[key]==prior['artifacts'][key],key
def bind_files(value):
    if isinstance(value,list):
        for child in value:bind_files(child)
    elif isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():assert entry(value['path'])=={k:value[k] for k in ['path','byteLength','sha256']}
        for key,path in value.items():
            if key.endswith('Path') and key[:-4]+'Sha256' in value:assert entry(path)['sha256']==value[key[:-4]+'Sha256'],path
            if isinstance(path,(list,dict)):bind_files(path)
def artifact_hashes(report,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm','componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in report for k in ['nativeSha256','nativeCliSha256']) and any(k in report for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for key,artifact in aliases.items():
        if key in report:assert report[key]==artifacts[artifact]['sha256'],key
    bind_files(report['cases'] if 'cases' in report else report['batches'])
regressions={}
for name,old_record in prior['regressionReports'].items():
    snapshot=ROOT/'previous'/(name+'.json');assert entry(snapshot)['sha256']==old_record['report']['sha256']
    old=read(snapshot);path=ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(old_record['report']['path'])
    report=read(path);field='batches' if name=='bidiConformance' else 'cases'
    assert report[field]==old[field],name
    artifact_hashes(report,name in ['placement','groups','pathRaster','sceneRaster','pageRaster'])
    if name=='source':
        for case in report['cases']:
            if 'source' in case:assert entry(case['source'])['sha256']==case['sourceSha256']
        for case in report['outputs']:assert entry(case['output'])['sha256']==case['sha256']
    regressions[name]={'report':entry(path),'batches':len(report[field]),'allPreviousCasesUnchanged':True}
assert sum(r['batches'] for r in regressions.values())==5216
paths=read(ROOT/'parity.json');guides=read(ROOT/'guide-parity.json');independent=read(ROOT/'independent.json');manifest=read(ROOT/'manifest.json')
for report in [paths,guides]:artifact_hashes(report);assert report['exactResponseBytes']
assert len(paths['cases'])==814 and len(guides['cases'])==206 and len(manifest['cases'])==808
assert paths['compiled']==independent['compiled']==762 and paths['unresolved']==independent['unresolved']==46
assert len(independent['cases'])==808 and independent['ownedXsdParts']==1236 and independent['precisionDigits']==80
lookup={c['name']:c for c in paths['cases']}
for case in independent['cases']:
    assert case['sourceSha256']==lookup[case['name']]['sourceSha256'] and case['responseSha256']==lookup[case['name']]['responseSha256']
for case in manifest['cases']:assert entry(case['path'])['sha256']==case['sha256']
for key,file,report in [('nativePaths','parity.json',paths),('nativePathGuides','guide-parity.json',guides)]:regressions[key]={'report':entry(ROOT/file),'batches':len(report['cases'])}
contracts=read(ROOT/'contracts.json');assert [contracts[k] for k in ['schemas','nativePathResponses','nativePathRequests']]==[57,814,812]
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M);assert len(tests)==302
logs=['rust-tests.log','clippy.log','fmt.log','schema-check.log','types-check.log','parity.log','independent.log','regressions.log']
for log in logs:
    raw=(ROOT/log).read_text();assert not any(s in raw for s in ['error:','FAILED','Traceback']),log
for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')]:
    old=[s for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(suffix)];assert len(old)==55
    for record in old:assert entry(record['path'])==record
    assert {str(p) for p in Path(prefix).glob('*'+suffix)}-{s['path'] for s in old}=={prefix+'pptx-paths-query'+suffix,prefix+'pptx-paths-response'+suffix}
lock=Path('Cargo.lock').read_text()
for package,dependency in [('mo-kernel-api','mo-geometry'),('mo-presentation-compile','mo-pptx')]:
    start=lock.index('name = "'+package+'"');end=lock.index('[[package]]',start);section=lock[start:end];line=' "'+dependency+'",\n';assert section.count(line)==1
    lock=lock[:start]+section.replace(line,'')+lock[end:]
assert sha(lock.encode())==next(s['sha256'] for s in prior['sourceFiles'] if s['path']=='Cargo.lock')
assert entry('pnpm-lock.yaml')==next(s for s in prior['sourceFiles'] if s['path']=='pnpm-lock.yaml')
for reference in prior['reusedGeometryReferences'].values():bind_files(reference)
# Components were not rebuilt. The prior evidence already contains the complete
# pinned component/source binding; verify those sources remain identical.
component_seal=read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
for build in component_seal['componentBuilds'].values():
    assert build['lock']==read('components/skia/lock.json')
    for record in build['componentSources']+build['artifacts']:assert entry(record['path'])==record
assert read('.codex-work/skia/verification/parity.json')==component_seal['componentReport']
sources={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':302,'newRustTests':10,'nativeWasmLogicalBatches':sum(r['batches'] for r in regressions.values()),'previousLogicalBatchesRerun':5216,
        'newPathBatches':814,'newGuideBatches':206,'repeatedGuideBatchesNotAdded':602,'pptxInputs':808,'newOwnedInputs':206,'compiledInputs':762,'unresolvedInputs':46,
        'compiledPaths':independent['paths'],'controlPoints':independent['controlPoints'],'arcSegments':independent['arcSegments'],'ellipseSamples':independent['ellipseSamples'],'ownedXsdParts':1236,
        'runtimeSchemas':57,'previousSchemasUnchanged':55,'strictClippy':True,'rustfmt':True,'typescript':True}
assert checks['nativeWasmLogicalBatches']==6236
result={'format':'musteroffice.native-path-verification/1','previousEvidence':entry(previous),'checks':checks,
        'scope':'Actual custom native path-space scaling and certified polar elliptic arc compilation to local Q32 EMU. No complete imported-page painting or target application acceptance.',
        'artifacts':artifacts,'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
        'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'python':platform.python_version(),
                       'rust':subprocess.check_output(['rustc','--version'],text=True).strip(),'node':subprocess.check_output(['node','--version'],text=True).strip()},
        'artifactSizeScope':'Uncompressed incomplete development binaries; no full kernel, installer, latency or memory acceptance.',
        'regressionReports':regressions,'pathReport':paths,'guideReport':guides,'manifest':manifest,'independentReport':independent,'contractReport':contracts,
        'rustTestNames':tests,'validationLogs':[entry(ROOT/p) for p in logs],'sourceFiles':[entry(p) for p in sorted(sources)],
        'reusedGeometryReferences':prior['reusedGeometryReferences'],'reusedComponentEvidence':{**prior['reusedComponentEvidence'],'rerun':False},
        'dependencyChanges':{'externalRuntimeVersions':[],'internalEdges':['mo-kernel-api -> mo-geometry','mo-presentation-compile -> mo-pptx'],'pnpmLockUnchanged':True},
        'limitations':['Bounds are relative to evaluator binary64 outputs, not exact source formulas, world/device transforms, raster coverage or Office/WPS pixels.',
                       'One-zero-radius arcs and explicit zero path dimensions remain diagnosed. Preset expansion and source geometry inheritance remain incomplete.',
                       'Original fill/stroke/extrusion modifiers are retained, not resolved into full paints. No imported-page text/image/effect compilation or rendering in this milestone.',
                       'No new external Office/WPS/LibreOffice observation, kernel frame comparison, or application edit roundtrip.',
                       'Advanced objects/playback, browser lifecycle, Agent/Musterwork E0-E3 and complete performance/installer acceptance remain incomplete.']}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(sources),'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
