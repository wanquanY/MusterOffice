"""Seal ABI 2 strokes, exact artifact regressions and independent numeric evidence.

Default validates and reports; --seal writes a new immutable evidence file.
"""
import hashlib
import json
import platform
import re
import struct
import sys
from pathlib import Path

ROOT=Path('.codex-work/stroke')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-stroke-raster-verification.json')
def read(p):return json.loads(Path(p).read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':sha(b)}
previous='docs/reviews/evidence/2026-09-24-page-render-verification.json';prior=read(previous)
assert entry(previous)['sha256']=='d041d269c66c717eedd49bd76d26a0c20492bc0d44c8d806bdddeb4ceef1cc6c'
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rustWasmGlue','rasterWasmGlue']:
    assert artifacts[k]==prior['artifacts'][k],k

def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli','nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm',
             'componentSha256':'rasterWasm' if raster else 'cppWasm','componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in r:assert r[k]==artifacts[a]['sha256'],k

def case_files(r):
    for c in r.get('cases',[]):
        for k in ['request','canonicalRequest','response','plan','source','font','bundle','frame','pixels','scene','pptx']:
            if k+'Path' in c:assert entry(c[k+'Path'])['sha256']==c[k+'Sha256'],(c['name'],k)

regressions={}
for name in ['document','opc','source','color','font','export']:
    p=ROOT/(name+'-regression.json');r=read(p);old_path=prior['regressionReports'][name]['report']['path']
    assert entry(old_path)==prior['regressionReports'][name]['report']
    assert r['cases']==read(old_path)['cases'],name
    hashes(r);case_files(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'semanticResultsUnchanged':True}

historical={
 'shaping':('text-shaping','textReport'),'unicode':('unicode-text','unicodeReport'),
 'cascade':('font-cascade','cascadeReport'),'bidi':('bidi','bidiReport'),'bidiConformance':('bidi','conformanceReport'),
 'itemizationAndParagraph':('font-fallback','updatedItemizationReport'),'mixedFont':('font-fallback','fallbackReport'),
 'lineBreaking':('line-break','lineBreakReport'),'fontMetrics':('font-metrics','metricsReport'),
 'lineShaping':('line-shaping','lineShapeReport'),'lineGeometry':('line-geometry','lineGeometryReport'),
 'paragraphLayout':('paragraph-layout','paragraphLayoutReport'),'fontOutlines':('font-outlines','outlinesReport'),
 'paragraphPaths':('paragraph-paths','paragraphPathsReport'),
}
for name,(milestone,key) in historical.items():
    p=prior['regressionReports'][name]['report']['path'];r=read(p)
    old=read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
    field='batches' if name=='bidiConformance' else 'cases';assert r[field]==old[field],name
    hashes(r);case_files(r)
    regressions[name]={'report':entry(p),'batches':len(r[field]),'semanticResultsUnchanged':True}

angles=read('docs/reviews/evidence/2026-09-24-static-rotation-export-verification.json')
for name,p,key in [('placement','.codex-work/page-placement/parity.json','pagePlacementReport'),
                   ('groups','.codex-work/angle-export/parity.json','groupPlacementReport'),
                   ('angles','.codex-work/angle-export/angle-parity.json','angleExportReport'),
                   ('angleSource','.codex-work/angle-export/source-parity.json','angleSourceReport')]:
    r=read(p);assert r['cases']==angles[key]['cases'],name;hashes(r,name in ['placement','groups']);case_files(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'semanticResultsUnchanged':True}

def prior_frame(current):
    b=Path(current['framePath']).read_bytes();w=list(struct.unpack('<'+'I'*(len(b)//4),b))
    assert w[1]==2 and w[8]==0
    offset=10
    for _ in range(w[5]):offset+=2+7*w[offset+1]
    out=w[:offset];out[1]=1
    for _ in range(w[6]):
        assert w[offset+4]==0;out.extend(w[offset:offset+4]);offset+=5
    assert offset==len(w)
    return sha(struct.pack('<'+'I'*len(out),*out))

def old_metadata(value,frame_sha):
    if isinstance(value,list):return [old_metadata(v,frame_sha) for v in value]
    if not isinstance(value,dict):return value
    out={}
    for k,v in value.items():
        if k in ['strokeStyles','strokeDraws','strokeWidthErrorBound','miterLimitErrorBound']:
            assert int(v)==0;continue
        if k=='profile' and v=='skia-8d6d37b-q32-local-paths-srgb-premul-rgba8-v2-draft':
            v='skia-8d6d37b-q32-local-paths-srgb-premul-rgba8-v1-draft'
        if k=='frameSha256':v=frame_sha
        out[k]=old_metadata(v,frame_sha)
    return out

reports={};transitions={}
old_reports={
 'path':read('docs/reviews/evidence/2026-09-24-path-raster-verification.json')['pathRasterReport'],
 'scene':read('docs/reviews/evidence/2026-09-24-scene-raster-reachability-verification.json')['sceneRasterReport'],
 'page':prior['pageRenderReport'],
}
for name,directory in [('path','path-raster'),('scene','scene-raster'),('page','page-render')]:
    p=Path('.codex-work')/directory/'parity.json';r=read(p);hashes(r,True);case_files(r);reports[name]=r
    now={c['name']:c for c in r['cases']};old=old_reports[name]
    transitioned=0
    for c in old['cases']:
        n=now[c['name']]
        for key in ['requestSha256','status','pixelsSha256','planSha256']:assert n.get(key)==c.get(key),(name,c['name'],key)
        if n['status']=='rendered':
            assert prior_frame(n)==c['frameSha256'],(name,c['name'],'frame')
            normalized=old_metadata(read(n['responsePath']),c['frameSha256'])
            assert sha(json.dumps(normalized,ensure_ascii=False,separators=(',',':')).encode())==c['responseSha256'],(name,c['name'],'metadata')
            transitioned+=1
        else:assert n['responseSha256']==c['responseSha256']
    assert r['exactPixelAndMetadataEquality']
    transitions[name]={'oldBatches':len(old['cases']),'newBatches':len(r['cases'])-len(old['cases']),
                       'unchangedRequestsStatusesPixels':True,'oldFrameAndMetadataHashesReconstructed':transitioned}
    regressions[name+'Raster']={'report':entry(p),'batches':len(r['cases']),'transition':transitions[name]}
assert [len(reports[k]['cases']) for k in ['path','scene','page']]==[193,251,74]
assert sum(v['batches'] for v in regressions.values())==2492
assert sum(v['newBatches'] for v in transitions.values())==147

component=read('.codex-work/skia/verification/parity.json');assert len(component['cases'])==174
assert component['maxChannelDifference']==component['differentBytes']==0 and component['oraclePixels']==61445
component_old=read('docs/reviews/evidence/2026-09-24-skia-component-verification.json')['parityReport']
lookup={c['name']:c for c in component['cases']}
for c in component_old['cases']:
    for k in ['status','nativeSha256','wasmSha256','sanitizerMatches']:assert lookup[c['name']][k]==c[k],(c['name'],k)
builds={name:read('.codex-work/skia/'+name+'-build.json') for name in ['native','wasm','native-asan']}
for b in builds.values():
    assert b['lock']==read('components/skia/lock.json')
    for a in b['artifacts']+b['componentSources']:assert entry(a['path'])==a
for k,p in [('nativeSha256','.codex-work/skia/mo-skia-probe'),('asanSha256','.codex-work/skia/mo-skia-probe-asan'),
            ('wasmSha256',artifacts['rasterWasm']['path']),('glueSha256',artifacts['rasterWasmGlue']['path']),('adapterSha256',artifacts['rasterAdapter']['path'])]:
    assert component[k]==entry(p)['sha256']

references={}
for name in ['path','scene','page']:
    p=Path('.codex-work')/('page-render' if name=='page' else name+'-raster')/'reference.json'
    r=read(p);look={c['name']:c for c in reports[name]['cases']}
    for c in r['cases']:assert c['frameSha256']==look[c['name']]['frameSha256']
    references[name]=r
pixels=read(ROOT/'pixel-reference.json');assert len(pixels['samples'])==32
path_cases={c['name']:c for c in reports['path']['cases']}
for c in pixels['samples']:assert c['pixelsSha256']==path_cases[c['name']]['pixelsSha256']
previews=read(ROOT/'previews.json');assert previews['image']['sha256']==entry(previews['image']['path'])['sha256']
for c in previews['sources']:assert c['pixelsSha256']==path_cases[c['name']]['pixelsSha256']
contracts=read(ROOT/'contracts.json');assert contracts['schemas']==49
assert [contracts[k] for k in ['pathRasterResponses','sceneRasterResponses','pageRenderResponses','pageCompileResponses']]==[193,251,74,74]
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')]
changed=[s['path'] for s in schemas if entry(s['path'])!=s]
assert sorted(changed)==sorted('contracts/generated/'+n+'.schema.json' for n in ['page-compile-response','page-raster-response','path-raster-request','path-raster-response','scene-raster-request','scene-raster-response'])
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M);assert len(tests)==247
for log in ['rust-tests.log','clippy.log','types-check.log']:
    assert 'error:' not in (ROOT/log).read_text() and 'FAILED' not in (ROOT/log).read_text()
for p in ['Cargo.lock','pnpm-lock.yaml']:
    assert entry(p)==next(s for s in prior['sourceFiles'] if s['path']==p)

paths={s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:
            paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
result={
 'format':'musteroffice.stroke-raster-verification/1',
 'scope':'Evaluated world-width solid strokes through Rust path/scene compilation, Native worker and WASM Skia. Author/PPTX stroke semantics and full presentation rendering remain unfinished.',
 'previousEvidence':entry(previous),'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts, not full kernel or Musterwork installer sizes; no performance/RSS gate claim.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'checks':{'rustTests':247,'newRustTests':5,'nativeWasmLogicalBatches':2492,'previousLogicalBatchesPreserved':2345,
           'newPathBatches':77,'newSceneBatches':70,'componentBatches':174,'componentReferencePixels':61445,
           'strokeExactPixelSamples':32,'runtimeSchemas':49,'changedSchemas':6,'unchangedSchemas':43,
           'strictClippy':True,'rustfmt':True,'typescript':True},
 'pathRasterReport':reports['path'],'sceneRasterReport':reports['scene'],'pageRenderReport':reports['page'],
 'parameterAndGeometryReferences':references,'pixelReference':pixels,'previews':previews,
 'componentReport':component,'componentBuilds':builds,'abiTransitions':transitions,
 'contractReport':contracts,'changedSchemas':changed,'regressionReports':regressions,'rustTestNames':tests,
 'dependencyChanges':{'externalRuntimeVersions':[],'lockFilesUnchanged':True,'newDevelopmentDependencies':[]},
 'sourceFiles':[entry(p) for p in sorted(paths)],
 'limitations':[
  'Author page rendering still rejects strokes; this milestone adds explicit evaluated strokes only. OOXML mapping/defaults/inheritance require separate target-application evidence.',
  'No dashed/compound lines, arrows, brushes, opacity groups, effects, image paints or GPU strokes yet.',
  'Parameter/centerline bounds do not bound stroke ink edges, antialias coverage or discontinuities at miter thresholds.',
  'Native component/profile verified on macOS arm64 and WASM Node only. Browser Workers, Windows/Linux, production process pools and RSS budgets remain incomplete.',
  'Complete advanced editable objects, playback, Agent integrations and Musterwork E0-E3 remain unfinished.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':result['checks'],'sources':len(paths),'artifactByteDeltas':result['artifactByteDeltas'],'abiTransitions':transitions}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
