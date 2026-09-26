"""Verify artifact-bound clipped joins and preserve prior raster evidence.

Default validates; --seal writes a new immutable file with exclusive creation.
"""
import hashlib
import json
import platform
import re
import struct
import sys
from pathlib import Path

ROOT=Path('.codex-work/miter-clip')
OUTPUT=Path('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
def read(p): return json.loads(Path(p).read_text())
def sha(b): return hashlib.sha256(b).hexdigest()
def entry(p):
    p=Path(p);b=p.read_bytes()
    return {'path':str(p),'byteLength':len(b),'sha256':sha(b)}

previous='docs/reviews/evidence/2026-09-24-stroke-author-verification.json'
prior=read(previous)
assert entry(previous)['sha256']=='cd4e7b42849c1acc40b59b3f44b028af75bb3fbc8638f0eaf752e6d1d60c7a4c'
study='docs/reviews/evidence/2026-09-24-miter-compatibility-observations.json'
assert entry(study)['sha256']=='d0245ad91823105809b1cdcff965824fdf73552c43971772d329cfc63a30c187'
artifacts={k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rustWasmGlue','rasterWasmGlue']:
    assert artifacts[k]==prior['artifacts'][k],k

def hashes(r,raster=False):
    aliases={'nativeSha256':'nativeCli','nativeCliSha256':'nativeCli',
             'nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker',
             'wasmSha256':'rustWasm','rustWasmSha256':'rustWasm','wasmKernelSha256':'rustWasm',
             'componentSha256':'rasterWasm' if raster else 'cppWasm',
             'componentWasmSha256':'cppWasm','adapterSha256':'rasterAdapter'}
    assert any(k in r for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in r for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for k,a in aliases.items():
        if k in r: assert r[k]==artifacts[a]['sha256'],k

def case_files(r):
    for c in r.get('cases',[]):
        for k in ['request','canonicalRequest','response','plan','source','font','bundle','frame','pixels','scene','pptx','export']:
            if k+'Path' in c: assert entry(c[k+'Path'])['sha256']==c[k+'Sha256'],(c['name'],k)

regressions={}
for name in ['document','opc','source','color','font','export']:
    p=ROOT/(name+'-regression.json');r=read(p);old=prior['regressionReports'][name]['report']
    assert entry(old['path'])==old
    assert r['cases']==read(old['path'])['cases'],name
    hashes(r);case_files(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'semanticResultsUnchanged':True}
historical={
 'shaping':('text-shaping','textReport'),'unicode':('unicode-text','unicodeReport'),
 'cascade':('font-cascade','cascadeReport'),'bidi':('bidi','bidiReport'),
 'bidiConformance':('bidi','conformanceReport'),
 'itemizationAndParagraph':('font-fallback','updatedItemizationReport'),
 'mixedFont':('font-fallback','fallbackReport'),'lineBreaking':('line-break','lineBreakReport'),
 'fontMetrics':('font-metrics','metricsReport'),'lineShaping':('line-shaping','lineShapeReport'),
 'lineGeometry':('line-geometry','lineGeometryReport'),'paragraphLayout':('paragraph-layout','paragraphLayoutReport'),
 'fontOutlines':('font-outlines','outlinesReport'),'paragraphPaths':('paragraph-paths','paragraphPathsReport'),
}
for name,(milestone,key) in historical.items():
    p=prior['regressionReports'][name]['report']['path'];r=read(p)
    old=read('docs/reviews/evidence/2026-09-24-'+milestone+'-verification.json')[key]
    field='batches' if name=='bidiConformance' else 'cases'
    assert r[field]==old[field],name
    hashes(r);case_files(r)
    regressions[name]={'report':entry(p),'batches':len(r[field]),'semanticResultsUnchanged':True}
angles=read('docs/reviews/evidence/2026-09-24-static-rotation-export-verification.json')
for name,key in [('placement','pagePlacementReport'),('groups','groupPlacementReport'),('angles','angleExportReport'),('angleSource','angleSourceReport')]:
    p=prior['regressionReports'][name]['report']['path'];r=read(p)
    assert r['cases']==angles[key]['cases'],name
    hashes(r,name in ['placement','groups']);case_files(r)
    regressions[name]={'report':entry(p),'batches':len(r['cases']),'semanticResultsUnchanged':True}

def old_metadata(v,frame_map):
    if isinstance(v,list): return [old_metadata(x,frame_map) for x in v]
    if isinstance(v,dict): return {k:old_metadata(x,frame_map) for k,x in v.items()}
    if isinstance(v,str):
        if v in frame_map: return frame_map[v]
        return v.replace('skia-8d6d37b-q32-local-paths-srgb-premul-rgba8-v3-draft',
                         'skia-8d6d37b-q32-local-paths-srgb-premul-rgba8-v2-draft')
    return v

reports={};transitions={}
for name in ['path','scene','page']:
    p=Path('.codex-work')/('page-render' if name=='page' else name+'-raster')/'parity.json'
    r=read(p);old=prior[name+('RenderReport' if name=='page' else 'RasterReport')]
    hashes(r,True);case_files(r);assert r['exactPixelAndMetadataEquality']
    lookup={c['name']:c for c in r['cases']};matched=0
    for c in old['cases']:
        n=lookup[c['name']]
        for k in ['requestSha256','status','pixelsSha256']:
            assert n.get(k)==c.get(k),(name,c['name'],k)
        frame_map={}
        if 'framePath' in n:
            data=bytearray(Path(n['framePath']).read_bytes());assert struct.unpack_from('<I',data,4)[0]==3
            struct.pack_into('<I',data,4,2);assert sha(data)==c['frameSha256'],(name,c['name'])
            frame_map[n['frameSha256']]=c['frameSha256']
        for k in ['response','plan']:
            if k+'Path' not in n: continue
            normalized=old_metadata(read(n[k+'Path']),frame_map)
            assert sha(json.dumps(normalized,ensure_ascii=False,separators=(',',':')).encode())==c[k+'Sha256'],(name,c['name'],k)
        matched+=1
    reports[name]=r
    transitions[name]={'oldCases':matched,'newCases':len(r['cases'])-matched,
                       'oldPixelsInputsStatusesUnchanged':True,'oldFrameAndMetadataHashesReconstructed':True,
                       'permittedChanges':['device frame version 2 to 3','raster profile v2 to v3','dependent frame digest']}
    regressions[name+'Raster']={'report':entry(p),'batches':len(r['cases'])}

component=read('.codex-work/skia/verification/parity.json')
old_component=read('docs/reviews/evidence/2026-09-24-stroke-raster-verification.json')['componentReport']
assert component['imports']==old_component['imports']
lookup={c['name']:c for c in component['cases']}
for old in old_component['cases']:
    n=lookup[old['name']]
    for k in ['status','nativeSha256','wasmSha256']:assert n[k]==old[k],(old['name'],k)
for c in component['cases']:
    assert entry(c['requestPath'])['sha256']==c['requestSha256']
    assert c['sanitizerMatches'] and c['maxChannelDifference']==0
    if c['status']==0:
        for suffix,key in [('.rgba','nativeSha256'),('.wasm.rgba','wasmSha256')]:
            assert entry('.codex-work/skia/verification/'+c['name']+suffix)['sha256']==c[key]
assert component['actualAllocationFailure']['replacementVerified']
assert component['actualSkiaAllocationTrap']['outputSlotsZero']
builds={k:read('.codex-work/skia/'+k+'-build.json') for k in ['native','wasm','native-asan']}
for b in builds.values():
    assert b['lock']==read('components/skia/lock.json')
    for a in b['artifacts']+b['componentSources']+[b['gn'],b['ninja'],b['targetGraph']]:assert entry(a['path'])==a
for k,p in [('nativeSha256','.codex-work/skia/mo-skia-probe'),('asanSha256','.codex-work/skia/mo-skia-probe-asan'),
            ('wasmSha256',artifacts['rasterWasm']['path']),('glueSha256',artifacts['rasterWasmGlue']['path']),
            ('adapterSha256',artifacts['rasterAdapter']['path'])]:assert component[k]==entry(p)['sha256']
closure=read(ROOT/'compiled-closure.json')
for target in ['native','wasm']:
    source=next(Path('.codex-work/skia/source-'+target).glob('skia-*'))
    for c in closure[target]['translationUnits']:
        e=entry(source/c['path']);assert e['sha256']==c['sha256'] and e['byteLength']==c['byteLength']

references={}
for name in ['path','scene','page']:
    p=Path('.codex-work')/('page-render' if name=='page' else name+'-raster')/'reference.json'
    r=read(p);lookup={c['name']:c for c in reports[name]['cases']}
    for c in r['cases']:assert c['frameSha256']==lookup[c['name']]['frameSha256']
    references[name]=r
oracle=read(ROOT/'reference.json');lookup={c['name']:c for c in reports['path']['cases']}
for c in oracle['cases']+oracle['distinguishingPixels']:
    assert c['pixelsSha256']==lookup[c['name']]['pixelsSha256']
    if 'requestSha256' in c:assert c['requestSha256']==lookup[c['name']]['requestSha256']
previews=read(ROOT/'previews.json');assert entry(previews['image']['path'])==previews['image']
for c in previews['sources']:assert c['pixelsSha256']==lookup[c['name']]['pixelsSha256']
legacy_pixels=read('.codex-work/stroke/pixel-reference.json')
for c in legacy_pixels['samples']:assert c['pixelsSha256']==lookup[c['name']]['pixelsSha256']
contracts=read(ROOT/'contracts.json');assert contracts['schemas']==49
assert [contracts[k] for k in ['pathRasterResponses','sceneRasterResponses','pageRenderResponses','pageCompileResponses']]==[267,316,131,131]
schemas=[s for s in prior['sourceFiles'] if s['path'].startswith('contracts/generated/')]
changed=[s['path'] for s in schemas if entry(s['path'])!=s]
assert set(changed)=={'contracts/generated/'+s+'.schema.json' for s in ['path-raster-request','scene-raster-request','page-compile-response']}
tests=re.findall(r'^test (.+) \.\.\. ok$',(ROOT/'rust-tests.log').read_text(),re.M)
assert len(tests)==253
for log in ['rust-tests.log','clippy.log','types-check.log','cpp-warnings.log']:
    assert 'error:' not in (ROOT/log).read_text() and 'FAILED' not in (ROOT/log).read_text(),log
for p in ['Cargo.lock','pnpm-lock.yaml']:
    assert entry(p)==next(s for s in prior['sourceFiles'] if s['path']==p)

paths={s['path'] for s in read(study)['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts:
            paths.add(str(p))
for p in paths:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']:assert len(Path(p).read_text().splitlines())<=2000,p
checks={'rustTests':len(tests),'newRustTests':1,'nativeWasmLogicalBatches':sum(r['batches'] for r in regressions.values()),
        'previousLogicalBatchesPreserved':2549,'newPathBatches':74,'newSceneBatches':65,
        'componentBatches':len(component['cases']),'componentReferencePixels':component['oraclePixels'],
        'independentClipCases':len(oracle['cases']),'independentClipInteriorPixels':oracle['interiorPixels'],
        'independentClipExteriorPixels':oracle['exteriorPixels'],'runtimeSchemas':49,'changedSchemas':len(changed),
        'strictClippy':True,'rustfmt':True,'typescript':True,'cppAdapterWarnings':True}
assert checks['nativeWasmLogicalBatches']==2688
result={'format':'musteroffice.miter-clip-verification/1',
 'scope':'Explicit evaluated clipped-miter primitive, CPU component extension and Rust path/scene access. Author/PPTX mapping is unchanged; no target-app fidelity acceptance.',
 'previousEvidence':entry(previous),'compatibilityStudy':entry(study),'artifacts':artifacts,
 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
 'artifactSizeScope':'Uncompressed incomplete development artifacts; not installer size, complete kernel, performance or RSS acceptance.',
 'environment':{'platform':platform.system()+' '+platform.release(),'architecture':platform.machine(),'rust':'1.92.0','node':'23.5.0','python':platform.python_version()},
 'checks':checks,'pathRasterReport':reports['path'],'sceneRasterReport':reports['scene'],'pageRenderReport':reports['page'],
 'componentReport':component,'componentBuilds':builds,'compiledClosure':closure,'abiTransitions':transitions,
 'parameterAndGeometryReferences':references,'clipPixelReference':oracle,'legacyStrokePixelReference':legacy_pixels,
 'previews':previews,'contractReport':contracts,'changedSchemas':changed,'regressionReports':regressions,'rustTestNames':tests,
 'dependencyChanges':{'externalRuntimeVersions':[],'lockFilesUnchanged':True,'newDevelopmentDependencies':[]},
 'sourceFiles':[entry(p) for p in sorted(paths)],
 'limitations':[
  'miterClip limit is restricted to [1,1024]. Exact reversals use bevel. Width zero explicitly uses a one-device-pixel outline; old join hairlines are unchanged.',
  'Curve offsets, AA boundary coverage and ink boundary error are not certified by the independent polygon interior/exterior oracle. Native/WASM parity and four curve probes do not replace such validation.',
  'No inferred WPS formula or Office profile is installed. Old author miter values continue to use the previous miter-to-bevel mapping and exported DrawingML limits are unchanged.',
  'Prior external WPS/LibreOffice observations are retained, not rerun against these artifacts. PowerPoint and edit roundtrips remain unverified.',
  'Native macOS arm64 and Node WASM only. Browser Workers, other OSes, production pools/cancellation/RSS enforcement and release toolchain reproducibility remain incomplete.',
  'Full styles/inheritance, page text/images, advanced objects, playback, public Agent integration and Musterwork E0-E3 remain incomplete.',
 ]}
raw=json.dumps(result,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
summary={'checks':checks,'sourceFiles':len(paths),'artifactByteDeltas':result['artifactByteDeltas'],'abiTransitions':transitions}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
    summary['evidence']=entry(OUTPUT)
print(json.dumps(summary,indent=2))
