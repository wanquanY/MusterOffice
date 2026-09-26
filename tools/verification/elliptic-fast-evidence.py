"""Seal current C++ optimization, exact-root verification and actual-page observations."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/elliptic-fast')
previous=Path('docs/reviews/evidence/2026-09-26-elliptic-source-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-elliptic-fast-verification.json')
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='2ba61648e76cc98a3418f49625fc571008eb99fd06614d193cc6f706361c5846'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
components/skia/mo_elliptic_field.cpp
components/skia/mo_elliptic_prepared.cpp
components/skia/mo_elliptic_prepared.h'''.splitlines())
added=set('''components/skia/mo_elliptic_geometry.h
docs/implementation/elliptic-gradient-performance.md
tools/verification/elliptic-fast-benchmark.mjs
tools/verification/elliptic-fast-pages-benchmark.mjs
tools/verification/elliptic-fast-checks.py
tools/verification/elliptic-fast-parity.mjs
tools/verification/elliptic-fast-workspace.py
tools/verification/elliptic-fast-replay.mjs
tools/verification/elliptic-fast-components.mjs
tools/verification/elliptic-fast-evidence.py'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
paths={k:root/(k+'.json') for k in ['replay','components','benchmark','pages-benchmark']}
paths['prepared']=root/'prepared/verification.json'
reports={k:json.loads(p.read_text()) for k,p in paths.items()}
prepared=reports['prepared'];assert prepared['cases']==6486 and prepared['priorScalarByteIdentical']==1238
assert prepared['statuses']=={'0':6438,'2':41,'1':5,'3':2}
assert all(c['exitCode']==0 for c in prepared['commands'])
replay=reports['replay'];assert replay['pairedCalls']==355 and replay['componentTriples']==83
assert replay['cli']['overwriteExit']!=0 and replay['cli']['failureOutputAbsent']
assert sum(c['status']==3 for c in replay['frames'])==4
components=reports['components'];assert components['triples']==551 and components['legacyFrames']==490 and components['sourceFrames']==18
assert components['failureRecovery']
for name,count in [('benchmark',7),('pages-benchmark',8)]:
    r=reports[name];assert len(r['results'])==count and r['policy']['samples']==25 and r['policy']['warmups']==3
    for c in r['results']:
        for v in ['before','after']:
            for s in c[v].values():
                assert len(s['samplesMs'])==25 and sorted(s['samplesMs'])[12]==s['medianMs']
                assert sorted(s['samplesMs'])[23]==s['p95Ms'] and min(s['samplesMs'])>0
checks=json.loads((root/'workspace.json').read_text());assert len(checks)==6 and all(c['exitCode']==0 for c in checks)
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==620
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==82
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==82
for name in ['extended-checks','build-native','build-wasm','build-asan','benchmark','pages-benchmark','components','replay']:
    log=root/(name+'.log');text=log.read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','runtime error:','ERROR: AddressSanitizer']),name
for c in checks+prepared['commands']:
    text=Path(c['log']).read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','runtime error:','ERROR: AddressSanitizer']),c['name']
native=[]
for p in sorted((root/'native-pages').iterdir()):
    if not p.is_file():continue
    before=Path('.codex-work/elliptic-source/native-final')/p.name
    assert p.read_bytes()==before.read_bytes(),p
    native.append(dict(current=entry(p),previous=entry(before)))
assert len(list((root/'native-pages').glob('*.pptx')))==29

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
        actual=entry(Path('.codex-work/emsdk')/tool['path'])
        assert actual['sha256']==tool['sha256'] and actual['byteLength']==tool['byteLength']
for p in builds.values():audit_build(p,False)
for r in old['componentBuilds'].values():audit_build(r['path'],True)
current={k:entry(p) for k,p in {
 'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
 'rustWasm':'.codex-work/elliptic-source/wasm-node/mo_wasm_bg.wasm',
 'rustWasmGlue':'.codex-work/elliptic-source/wasm-node/mo_wasm.js',
 'typescriptRaster':'.codex-work/elliptic-source/ts-raster/index.js','cppWasm':str(root/'component/mo-skia.wasm')}.items()}
for k in ['rustWasm','rustWasmGlue','typescriptRaster']:assert current[k]==old['currentArtifacts'][k]
for p in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:assert entry(p)==prior[p]
assert current['cppWasm']['byteLength']==2352940
assert current['cppWasm']['byteLength']-old['currentArtifacts']['cppWasm']['byteLength']==9159
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
    if parts[0]=='component' or any('attempt' in s for s in parts):continue
    if p.name in ['next.md','seal.log','seal-check.log','initial-checks.log']:continue
    artifacts.append(entry(p))
report=dict(format='musteroffice.elliptic-fast-verification/1',previousEvidence=entry(previous),
 scope='Immutable elliptic coefficients and certified nested first-root evaluation in current Native/WASM raster and real PPTX API; not complete product acceptance.',
 sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 workspaceChecks=checks,rustTestNames=tests,checks=dict(rustTests=620,schemas=82,strictClippy=True,rustfmt=True,typescript=True,
    exactReferenceCases=6486,priorScalarResultsUnchanged=1238,publicPairedCallsUnchanged=355,sourcePptxRegeneratedUnchanged=29,
    componentTriples=634,addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,
    cliCreate=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,unchangedDependencies=True,
    lowLevelBenchmarkCases=7,publicPptxBenchmarkCases=8,markdownFiles=markdown,localLinks=links),
 reports={k:entry(p) for k,p in paths.items()},componentBuilds={k:entry(p) for k,p in builds.items()},
 artifacts=artifacts,currentArtifacts=current,nativePages=native,auditedArtifactRecords=records,
 historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(cppWasmBytes=2352940,previousCppWasmBytes=2343781,increase=9159,rustWasmAndTypescriptUnchanged=True,
   note='Raw module bytes, not compressed package or complete host/dependency/font/media closure.'),
 limitations=['Sufficient nesting certificate does not cover every nested family; uncertain cases retain the general solver.',
   'Non-nested and near-degenerate gradients remain costly. No SIMD solver, startup, RSS or installer measurement.',
   'Warm benchmark observations on one machine; sequential order and uncontrolled external load. Control cases have no uniform improvement.',
   'Eight actual PPTX benchmarks are 400x300 gradient-only pages in the Rust WASM API, not Musterwork or its Native host.',
   'Source parameter displacement still does not bound scalar/color/coverage. Correct rounding applies to the encoded binary32 field.',
   'Historical independent source/pixel references apply to unchanged frames/pixels; Office/WPS compatibility is not newly verified.',
   'Stationary/shape gradients, complete advanced editable objects/playback and Agent/Musterwork distribution remain incomplete.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),rustTests=620,pairedCalls=355,auditedArtifactRecords=records)))
