"""Seal shared coordinate correctness, real pixel integration, and measured cost."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/gradient-coordinates')
previous=Path('docs/reviews/evidence/2026-09-26-transform-edit-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-gradient-coordinate-verification.json')
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='ae15d6aa2969ec3d64ba06e742e15eef5abe9d862027cd61c2f24f7553470bd2'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
docs/implementation/source-transform-edit.md
components/skia/elliptic-gradient.patch
components/skia/mo_gradient.h
components/skia/mo_gradient_plane.cpp
components/skia/mo_gradient_plane_stage.h
tools/components/build-skia.py'''.splitlines())
added=set('''components/skia/mo_gradient_coordinates.h
components/skia/mo_gradient_coordinates.cpp
components/skia/mo_gradient_coordinate_exact.h
docs/implementation/gradient-coordinate-precision.md
tools/verification/gradient-coordinate-checks.py
tools/verification/gradient-coordinate-probe.cpp
tools/verification/gradient-coordinate-parity.mjs
tools/verification/gradient-coordinate-skia-trace.cpp
tools/verification/gradient-coordinate-trace.py
tools/verification/gradient-coordinate-components.mjs
tools/verification/gradient-coordinate-render.mjs
tools/verification/gradient-coordinate-replay.mjs
tools/verification/gradient-coordinate-reference.py
tools/verification/gradient-coordinate-benchmark.mjs
tools/verification/gradient-coordinate-workspace.py
tools/verification/gradient-coordinate-evidence.py
tools/verification/gradient_coordinate_delta.mjs'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'workspace.json').read_text())
assert [c['name'] for c in checks]==['fmt','tests','clippy','native-build','schema-check','types-check']
assert all(c['exitCode']==0 for c in checks)
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==636
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==83
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==83
paths={k:root/(k+'.json') for k in ['coordinates','trace','components','render','replay','reference','pages-benchmark']}
reports={k:json.loads(p.read_text()) for k,p in paths.items()}
numeric=reports['coordinates'];render=reports['render'];components=reports['components'];replay=reports['replay'];reference=reports['reference'];benchmark=reports['pages-benchmark']
assert numeric['counts']['total']==7513 and numeric['counts']['success']==7466 and numeric['counts']['invalid']==47
assert numeric['counts']['rigidPairs']==32 and all(c['exitCode']==0 for c in numeric['commands'])
assert len(reports['trace']['cases'])==4
assert components['triples']==554 and components['legacyTriples']==551 and components['failureRecovery']
assert {c['name']:c['status'] for c in components['cases'] if c['name'].startswith('exact-')}=={'exact-below-budget':0,'exact-over-budget':3}
assert render['exactCases']==8 and render['discrepancyCases']==0 and render['rigidTransformInvarianceAchieved']
assert render['comparedPixels']==960000 and render['beforeNativeWasmPages']==2 and render['afterNativeWasmPages']==8
assert replay['pairedCalls']==355 and replay['componentTriples']==83
assert replay['cli']['failureOutputAbsent'] and replay['cli']['overwriteExit']!=0
assert reference['verifiedPixels']==1848597 and reference['maximumChannelDifference']<=1 and reference['groupInheritedControlEqual']
assert len(benchmark['results'])==8
for c in benchmark['results']:
    rendered=next(r for r in replay['cases'] if r['kind']=='source-parity' and r['name']=='new/'+c['name'])
    assert c['after']['pixelSha256']==rendered['pixels']['sha256']
    assert c['before']['pixelSha256']==c['pixels']['sha256']
    for v in ['before','after']:
        for kind in ['total','rasterCallback','remaining']:assert len(c[v][kind]['samplesMs'])==25
for name in ['checks','trace','components','render','replay','reference','benchmark','workspace']:
    assert not any(s in (root/(name+'.log')).read_text() for s in ['error:','FAILED','Traceback','AssertionError','runtime error:','ERROR: AddressSanitizer']),name
for c in checks:
    assert not any(s in Path(c['log']).read_text() for s in ['error:','FAILED','Traceback','AssertionError']),c['name']
records=0;historical_changes=set();mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(v,historical=False):
    global records
    if isinstance(v,dict):
        if {'path','byteLength','sha256'}<=v.keys():
            if historical and v['path'] in allowed|mutable:historical_changes.add(v['path'])
            else:
                actual=entry(v['path']);assert all(v[k]==value for k,value in actual.items()),v['path'];records+=1
        for x in v.values():audit(x,historical)
    elif isinstance(v,list):
        for x in v:audit(x,historical)
audit(reports);audit(old,True)
for r in old['reports'].values():audit(json.loads(Path(r['path']).read_text()),True)
builds={k:root/'component'/f'{k}-build.json' for k in ['native','wasm','native-asan']}
def audit_build(path,historical):
    b=json.loads(Path(path).read_text());assert b['lock']==json.loads(Path('components/skia/lock.json').read_text())
    for k in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(b[k],historical)
    for tool in b['lock']['emsdkTools']:
        actual=entry(Path('.codex-work/emsdk')/tool['path']);assert actual['sha256']==tool['sha256'] and actual['byteLength']==tool['byteLength']
for p in builds.values():audit_build(p,False)
for r in old['componentBuilds'].values():audit_build(r['path'],True)
current={k:entry(p) for k,p in {
 'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
 'rustWasm':'.codex-work/transform-edit/wasm-node/mo_wasm_bg.wasm','rustWasmGlue':'.codex-work/transform-edit/wasm-node/mo_wasm.js',
 'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js','cppWasm':str(root/'component/mo-skia.wasm')}.items()}
for k in ['rustWasm','rustWasmGlue','typescriptRaster']:assert current[k]==old['currentArtifacts'][k]
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(p)==prior[p]
markdown=links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md':continue
    markdown+=1
    for t in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',t) or t.startswith('#'):continue
        t=unquote(t.split('#')[0].split('?')[0])
        if t:assert (p.parent/t).exists() or (p.parent/t).resolve()==output.resolve(),(name,t);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[]
for p in sorted(root.rglob('*')):
    if not p.is_file():continue
    parts=p.relative_to(root).parts
    if parts[0] in ['component','attempts']:continue
    if p.name in ['next.md','seal.log','seal-check.log']:continue
    artifacts.append(entry(p))
def changes(cases):
    changed=[c for c in cases if c['delta']['differentPixels']]
    return dict(cases=len(changed),pixels=sum(c['delta']['differentPixels'] for c in changed),
        maximumChannelDifference=max([c['delta']['maxChannelDifference'] for c in changed]+[0]))
report=dict(format='musteroffice.gradient-coordinate-verification/1',previousEvidence=entry(previous),
 scope='Correctly rounded encoded gradient coordinates, shared Native/WASM shader integration, strict rigid transform pixels, bounded exact fallback, regression observations and measured performance. Not complete product acceptance.',
 sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 workspaceChecks=checks,rustTestNames=tests,
 checks=dict(rustTests=636,schemas=83,strictClippy=True,rustfmt=True,typescript=True,
   exactCoordinateCases=numeric['counts'],actualOldSkiaDifferencePairs=4,
   beforeAfterNativeWasmRenderedPages=10,independentRigidPixels=960000,exactRigidCases=8,failedRigidCases=0,
   publicNativeWasmCalls=355,componentTriples=637,independentColorReferencePixels=1848597,
   addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,exactCoordinateFallbackBudget=65536,
   cliCreate=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,officeWpsNewAcceptance=False,
   unchangedDependencies=True,markdownFiles=markdown,localLinks=links),
 pixelChanges=dict(legacyComponents=changes(components['cases']),publicCalls=changes(replay['cases']),sourceComponents=changes(replay['frames'])),
 reports={k:entry(p) for k,p in paths.items()},componentBuilds={k:entry(p) for k,p in builds.items()},
 artifacts=artifacts,currentArtifacts=current,auditedArtifactRecords=records,historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(cppWasmBytes=current['cppWasm']['byteLength'],previousCppWasmBytes=2352940,increase=current['cppWasm']['byteLength']-2352940,
   rustWasmBytes=6696950,rustWasmAndTypescriptUnchanged=True,note='Raw module bytes, not complete host/dependency/font/media closure or installer size.'),
 performance=[dict(name=c['name'],beforeMs=c['before']['total']['medianMs'],afterMs=c['after']['total']['medianMs'],speedup=c['speedup']) for c in benchmark['results']],
 closedFinding=dict(previousStatus='failed-independent-exactness',previousAffectedCases=4,previousDifferentPixels=12,
   currentAffectedCases=0,currentDifferentPixels=0,scope='Two specified PPTX gradients and four rigid transforms each; not every possible transform, input, or target app.'),
 limitations=['Correct rounding applies to encoded binary32 coordinates; source displacement and unified color/coverage remain outside this certificate.',
   'Warm one-machine 400x300 WASM gradient-page observations do not establish product throughput, cold startup, RSS or installer size.',
   'Exact fallback has a fixed per-frame cap; wider workload admission and optimization remain required.',
   'No new Office/WPS editing/playback interoperability or stationary-gradient observation.',
   'Full structural and advanced editable objects, playback, Agent distribution and Musterwork replacement acceptance remain required.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),auditedArtifactRecords=records,checks=report['checks'],size=report['size'])))
