"""Bind completed image scene execution, independent reference and regressions."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote

ROOT=Path('.codex-work/image-scene')
PREVIOUS=Path('docs/reviews/evidence/2026-09-25-source-images-verification.json')
OUTPUT=Path('docs/reviews/evidence/2026-09-25-image-scene-verification.json')
def entry(path):
    p=Path(path);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256']=='048e582e9e13359fece03a826b43ccea3a998e05b153d394f8d3a6a8f4a472ed'
old=json.loads(PREVIOUS.read_text())
allowed={
    'Cargo.lock','README.md','contracts/README.md','crates/mo-kernel-api/src/lib.rs',
    'crates/mo-raster/src/image.rs','crates/mo-render/Cargo.toml','crates/mo-render/src/compile.rs',
    'crates/mo-render/src/lib.rs','crates/mo-render/src/tests.rs','crates/mo-wasm/src/raster.rs',
    'docs/README.md','docs/implementation/development.md','docs/implementation/progress.md',
    'tools/mo-contract-codegen/src/main.rs','tools/mo-raster-worker/src/main.rs',
    'tools/verification/contracts.py','tools/verification/image-brush-reference.py','tools/verification/source-images-parity.mjs',
}
changed={r['path'] for r in old['sourceFiles'] if entry(r['path'])!=r}
assert changed==allowed,(changed-allowed,allowed-changed)
added={
    'crates/mo-render/src/image.rs','crates/mo-render/src/image_tests.rs','crates/mo-kernel-api/src/image_scene.rs',
    'contracts/generated/image-scene-request.schema.json','contracts/generated/image-scene-response.schema.json',
    'packages/contracts/src/generated/image-scene-request.ts','packages/contracts/src/generated/image-scene-response.ts',
    'docs/implementation/image-scene.md','tools/verification/image-scene-fixtures.py',
    'tools/verification/image-scene-parity.mjs','tools/verification/image-scene-reference.py',
    'tools/verification/image_reference.py','tools/verification/image-scene-evidence.py',
}
sources={r['path'] for r in old['sourceFiles']}|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:assert len(Path(p).read_text().splitlines())<=2000,p
logs=['workspace-tests','clippy','fmt','workers-build','schema-write','schema-check','types-write','types-check',
      'contracts','wasm-build','bindgen','fixtures','parity','reference','regressions','recent-regressions','source-resources']
for name in logs:
    s=(ROOT/(name+'.log')).read_text()
    assert not any(x in s for x in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),name
for name in ['clippy','workers-build','wasm-build']:assert 'Finished' in (ROOT/(name+'.log')).read_text(),name
test_log=(ROOT/'workspace-tests.log').read_text()
assert 'Doc-tests mo_xml' in test_log and test_log.rstrip().endswith('finished in 0.00s')
tests=re.findall(r'^test (.+) \.\.\. ok$',test_log,re.M)
new_tests=[n for n in tests if n not in old['rustTestNames']]
assert (len(tests),len(new_tests))==(521,6)
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M))==76
assert len(re.findall(r'^check .+\.ts$',(ROOT/'types-check.log').read_text(),re.M))==76
contracts=json.loads((ROOT/'contracts.log').read_text());assert (contracts['schemas'],contracts['negativeMutations'])==(76,9)
expected={
    'parity':dict(sceneRequests=140,success=124,preflightFailures=16,jsonRejections=7,pairedCalls=148,worldReferenceCalls=1,wasmHostRejections=1),
    'reference':dict(cases=124,exactWorldVertices=560,comparedPixels=31744,channelTolerance=1),
    'regressions':dict(requests=307,pixelOutputs=21),
    'recent-regressions':dict(requests=211,priorStableResponses=204,textPages=124,textGeometry=41,imageRequests=39,jsonRejections=7),
    'source-resources':dict(catalogs=19,invalidRequests=6,pairedCalls=50,noOverwriteChecks=19),
}
assert json.loads((ROOT/'fixtures.log').read_text())==dict(success=124,failures=16,cases=140)
reports={}
for name,marker in expected.items():
    assert json.loads((ROOT/(name+'.log')).read_text())==marker,name
    reports[name]=json.loads((ROOT/(name+'.json')).read_text())
    assert all(reports[name]['counts'][k]==v for k,v in marker.items()),name
records=0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():assert entry(value['path'])==value,value['path'];records+=1
        for v in value.values():audit(v)
    elif isinstance(value,list):
        for v in value:audit(v)
audit(reports)
audit(old['unchangedComponents']);audit(old['standardInputs']);audit(old['previousReleaseArtifactsVerifiedUnchanged'])
markdown,links=0,0
for name in sources:
    p=Path(name)
    if p.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        destination=unquote(target.split('#')[0].split('?')[0])
        if not destination:continue
        resolved=p.parent/destination
        assert resolved.exists() or resolved.resolve()==OUTPUT.resolve(),(name,target)
        links+=1
subprocess.run(['git','diff','--check'],check=True)
report=dict(format='musteroffice.image-scene-verification/1',previousEvidence=entry(PREVIOUS),
    scope='Shared scene lowering and resource-bound image execution, Native Worker/Rust WASM; not source PPTX image decode or rendering.',
    checks=dict(rustTests=521,newRustTests=6,strictClippy=True,rustfmt=True,schemasChecked=76,
        generatedTypeScriptAndTypeCheck=True,genericNegativeMutations=9,newPairedRuntimeCalls=148,
        imageScenes=124,imageScenePreflightFailures=16,jsonRejections=7,wasmHostRejections=1,
        independentWorldVertices=560,independentlyComparedPixels=31744,channelTolerance=1,
        priorPairedRuntimeCalls=568,currentPairedRuntimeCalls=716,priorSavedResponsesAndPixelsUnchanged=True,
        sourceMarkdown=markdown,localLinks=links),
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new_tests,validationLogs=[entry(ROOT/(n+'.log')) for n in logs],
    reports={n:dict(record=entry(ROOT/(n+'.json')),counts=v['counts']) for n,v in reports.items()},
    artifacts=[entry(p) for p in ['target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker',
        'target/wasm32-unknown-unknown/debug/mo_wasm.wasm',ROOT/'wasm-node/mo_wasm.js',ROOT/'wasm-node/mo_wasm_bg.wasm',
        ROOT/'wasm-node/mo_wasm.d.ts',ROOT/'wasm-node/mo_wasm_bg.wasm.d.ts',ROOT/'wasm-node/package.json']],
    unchangedComponents=old['unchangedComponents'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],limitations=[
        'Prepared normalized RGBA8 only. Encoded image decoding, orientation/ICC and source PPTX crop/tile/stretch remain unconnected.',
        'World image brushes are already evaluated and are independent of path transforms. Native author semantics must be lowered before this API.',
        'Independent pixels use exact integer rectangular coverage after nontrivial affine scene transforms. No new arbitrary curve AA or image stroke visual acceptance is claimed.',
        'Geometry and independent image-affine bounds are checked separately; they do not bound filtering, coverage or final-color error.',
        'PreparedImages can be reused in Rust. JSON APIs still validate resources per call; production caches, browser Worker and cancellation host remain work.',
        'No new Office/WPS application, product latency/RSS, complete kernel or Musterwork installer measurement.',
        'Complete editing/export/player/advanced content and production Agent/Artifact/Musterwork E0-E3 acceptance remain incomplete.',
    ])
raw=json.dumps(report,ensure_ascii=False,indent=2)+'\n';assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:f.write(raw)
elif OUTPUT.exists():assert OUTPUT.read_text()==raw,'sealed evidence differs from current inputs'
print(json.dumps(dict(sources=len(sources),changed=len(changed),added=len(added),boundRecords=records,checks=report['checks'],evidence=entry(OUTPUT) if OUTPUT.exists() else None)))
