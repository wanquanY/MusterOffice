"""Seal actual PPTX integration, public entry points, explicit source deltas and artifacts."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
root=Path('.codex-work/elliptic-source')
previous=Path('docs/reviews/evidence/2026-09-25-elliptic-render-verification.json')
output=Path('docs/reviews/evidence/2026-09-26-elliptic-source-verification.json')
def entry(path):
    path=Path(path);b=path.read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='da043994396cfdb078140390584b1a95220ab525cefdda2d76f8968d6e04e7b7'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
reports={k:json.loads((root/(k+'.json')).read_text()) for k in ['runtime','source-parity','reference','parameter-reference','components','contract-compat','previews']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
crates/mo-kernel-api/src/pptx_page.rs
crates/mo-presentation-compile/src/native_paths.rs
crates/mo-presentation-compile/src/native_paths/tests.rs
crates/mo-presentation-compile/src/source_page.rs
crates/mo-presentation-compile/src/source_page/gradient.rs
crates/mo-presentation-compile/src/source_page/paint.rs
crates/mo-presentation-compile/src/source_page/prepared.rs
crates/mo-presentation-compile/src/source_page/types.rs
crates/mo-raster/src/compile.rs
crates/mo-raster/src/gradient.rs
crates/mo-raster/src/gradient_plane.rs
crates/mo-raster/src/lib.rs
crates/mo-raster/src/paint_precision.rs
crates/mo-raster/src/types.rs
crates/mo-skia-sys/src/ffi.rs
tools/mo-cli/src/raster.rs'''.splitlines())
compat=reports['contract-compat'];assert len(compat['cases'])==14
for c in compat['cases']:
    allowed.add(c['schema']['path'])
    allowed.update(r['historicalPath'] for r in c['restoredTypeFiles'])
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-harfbuzz-sys/tests/elliptic_gradient.rs
crates/mo-presentation-compile/src/source_page/gradient_circle.rs
crates/mo-presentation-compile/tests/elliptic_page.rs
crates/mo-raster/src/gradient_elliptic.rs
crates/mo-raster/src/gradient_elliptic_tests.rs
crates/mo-raster/src/profiles.rs
docs/implementation/elliptic-source-pages.md
tools/test-support/elliptic_page.rs
tools/verification/elliptic-parameter-reference.py
tools/verification/elliptic-source-checks.py
tools/verification/elliptic-source-components.mjs
tools/verification/elliptic-source-contract-compat.mjs
tools/verification/elliptic-source-fixtures.py
tools/verification/elliptic-source-parity.mjs
tools/verification/elliptic-source-previews.py
tools/verification/elliptic-source-reference.py
tools/verification/elliptic-source-runtime.mjs
tools/verification/elliptic-source-final-checks.py
tools/verification/elliptic-source-evidence.py'''.splitlines())
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:
        assert len(Path(p).read_text().splitlines())<=2000,p
checks=json.loads((root/'checks.json').read_text());final=json.loads((root/'final-checks.json').read_text())
assert len(checks)==11 and len(final)==7 and all(c['exitCode']==0 for c in checks+final)
for c in checks+final:
    text=Path(c['log']).read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','runtime error:','AddressSanitizer']),c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M);assert len(tests)==620
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==82
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==82
runtime=reports['runtime'];source=reports['source-parity'];ref=reports['reference'];param=reports['parameter-reference'];comp=reports['components']
assert runtime['pairedCalls']==183 and runtime['priorUnchanged']==121 and runtime['lowLevelEquivalent']==20
assert runtime['lateFailures']==2 and runtime['recovery']
assert source['pairedCalls']==172 and source['previousUnchanged']==138 and len(source['newlyAccepted'])==1
assert source['cli']['overwriteExit']!=0 and source['cli']['failureOutputAbsent']
assert ref['officialXsdParts']==203 and len(ref['sourceFiles'])==29 and len(ref['cases'])==15 and ref['verifiedPixels']==7125
assert max(c['maximumChannelDifference'] for c in ref['cases'])==1
assert param['comparisons']==24576 and len(param['cases'])==8
assert comp['triples']==83 and sum(c['status']==3 for c in comp['cases'])==4
assert len(reports['previews'])==29

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
audit(reports)
# The last immutable seal is retained. Its source modifications and rebuilt
# debug hosts are explicit deltas; frozen C++/WASM/corpus records are unchanged.
audit(old,True)
for r in old['reports'].values():audit(json.loads(Path(r['path']).read_text()),True)
for r in old['componentBuilds'].values():
    build=json.loads(Path(r['path']).read_text())
    assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    for key in ['componentSources','artifacts','imageCodecs','gn','ninja','targetGraph']:audit(build[key])
    # Tool paths in the dependency lock are relative to the explicitly selected
    # SDK root, whereas build artifacts above are workspace-rooted.
    for tool in build['lock']['emsdkTools']:
        actual=entry(Path('.codex-work/emsdk')/tool['path'])
        assert actual['sha256']==tool['sha256'] and actual['byteLength']==tool['byteLength']
        records+=1
current={k:entry(p) for k,p in {
 'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
 'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
 'cppWasm':'.codex-work/elliptic-render/component/mo-skia.wasm','typescriptRaster':str(root/'ts-raster/index.js')}.items()}
assert current['cppWasm']==old['currentArtifacts']['cppWasm']
assert current['typescriptRaster']['sha256']==old['currentArtifacts']['typescriptRaster']['sha256']
for dependency in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json']:
    assert entry(dependency)==prior[dependency]
markdown=links=0
for name in sources:
    path=Path(name)
    if path.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:assert (path.parent/target).exists() or (path.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[]
for p in sorted(root.rglob('*')):
    if not p.is_file():continue
    parts=p.relative_to(root).parts
    if parts[0] in ['attempt1','before-profile-registry','native'] or any('attempt' in s for s in parts):continue
    if p.name in ['next.md','seal.log','seal-check.log']:continue
    artifacts.append(entry(p))
report=dict(format='musteroffice.elliptic-source-verification/1',previousEvidence=entry(previous),
 scope='Rust V12 evaluated fields, parameter bounds, original PPTX circle layouts and Native/WASM/CLI publication. Development profile, not full Office/WPS or Musterwork acceptance.',
 sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 commandChecks=checks+final,rustTestNames=tests,
 checks=dict(rustTests=620,schemas=82,strictClippy=True,rustfmt=True,typescript=True,changedSchemas=14,
    reconstructedHistoricalTypeFiles=16,sourcePptx=29,officialXsdParts=203,publicPairedCalls=355,
    previousGenericCallsUnchanged=121,previousSourceCallsUnchanged=138,newlyAcceptedPreviousSources=1,
    sourceReferencePixels=7125,maximumSampledChannelDifference=1,parameterGeometrySamples=24576,
    componentTriples=83,addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,
    cliCreate=True,cliOverwriteRefused=True,cliFailureOutputAbsent=True,unchangedDependencies=True,
    markdownFiles=markdown,localLinks=links),
 reports={k:entry(root/(k+'.json')) for k in reports},artifacts=artifacts,currentArtifacts=current,
 componentBuilds=old['componentBuilds'],auditedArtifactRecords=records,
 historicalChangesExplicitlyAccounted=sorted(historical_changes),
 size=dict(rustWasmBytes=current['rustWasm']['byteLength'],previousRustWasmBytes=old['currentArtifacts']['rustWasm']['byteLength'],
   cppWasmBytes=current['cppWasm']['byteLength'],cppWasmIncrease=0,typescriptRasterUnchanged=True,
   note='Raw modules, not complete dependency closure or a Musterwork installer; no new end-to-end performance measurement.'),
 limitations=['General elliptic raster performance remains the prior measured bottleneck; no new speedup claim.',
   'Parameter displacement and quantized-field root enclosure are separate; neither guarantees source scalar, pixel color or coverage.',
   'Source reference samples axis-aligned rectangles every 16 pixels; rotated/custom-path AA has parity and structural checks, not a new independent coverage oracle.',
   'WPS near-equal-focus discrepancy, stationary orientation, shape paths and PowerPoint observation remain open.',
   'Advanced editable content, playback, Agent distribution and Musterwork replacement acceptance are incomplete.'])
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sourceFiles=len(sources),rustTests=620,pairedCalls=355,auditedArtifactRecords=records)))
