"""Seal the standalone numerical implementation; no page/interop claim."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote

root=Path('.codex-work/elliptic-scalar')
previous=Path('docs/reviews/evidence/2026-09-25-radial-anchor-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-elliptic-scalar-verification.json')
def entry(path):
    p=Path(path);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='617220dd429cb3db454e26d64de21a86b79b2883ed7db9edaae8dc6dd7071735'
old=json.loads(previous.read_text());prior={v['path']:v for v in old['sourceFiles']}
allowed={'README.md','docs/README.md','docs/implementation/progress.md','docs/implementation/development.md'}
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,changed
added={
    'components/skia/mo_elliptic_field.h','components/skia/mo_elliptic_field.cpp',
    'components/skia/mo_elliptic_interval.h','docs/implementation/elliptic-gradient-scalar.md',
    *('tools/verification/elliptic-scalar-'+s for s in ['probe.cpp','checks.py','parity.mjs','benchmark.mjs','evidence.py']),
}
assert not added&set(prior); sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:
        assert len(Path(p).read_text().splitlines())<=2000,p
for p in prior:
    if p not in allowed: assert entry(p)==prior[p],p

records=0
mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(value,historical=False):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys() and not(historical and value['path'] in mutable):
            assert all(value[k]==v for k,v in entry(value['path']).items()),value['path']
            records+=1
        for v in value.values(): audit(v,historical)
    elif isinstance(value,list):
        for v in value: audit(v,historical)

verification=json.loads((root/'verification.json').read_text())
benchmark=json.loads((root/'benchmark.json').read_text())
toolchain=json.loads((root/'toolchain.json').read_text())
assert verification['cases']==1238 and verification['statuses']=={'0':1230,'1':5,'2':1,'3':2}
assert verification['certifiedFastPaths']==838 and verification['maxVisitedNodes']==45
assert verification['exactIntervalComparisons']==12015
assert len(verification['commands'])==4 and all(v['exitCode']==0 for v in verification['commands'])
assert len(verification['results'])==1238
multi=next(v for v in verification['results'] if v['name']=='multiple-crossings')
assert multi['reference']==dict(degree=4,roots=3) and .03045<multi['value']<.03046
for c in verification['commands']:
    text=Path(c['log']['path']).read_text()
    assert not any(x in text for x in ['error:','runtime error:','AddressSanitizer','AssertionError']),c['name']
for name in ['native','native-interval','sanitizer','sanitizer-interval']:
    assert (root/(name+'.log')).read_bytes()==b''
assert toolchain['fastMathRejection']['exitCode']!=0
assert 'Elliptic interval arithmetic requires strict IEEE operations' in Path(toolchain['fastMathRejection']['log']['path']).read_text()
assert len(benchmark['results'])==6
for b in benchmark['results']:
    assert b['count']==16384 and b['width']==b['height']==128 and b['statuses']==[16384,0,0,0]
    for runtime in ['native','wasm']:
        r=b[runtime];assert len(r['samplesMs'])==7
        assert r['minimumMs']==min(r['samplesMs']) and r['maximumMs']==max(r['samplesMs'])
        assert r['medianMs']==sorted(r['samplesMs'])[3]
audit(verification);audit(toolchain)
# Baseline snapshots have their own paths/hashes. The copied historical report
# is retained as historical bytes; its internal mutable build paths are not
# asserted to still contain the baseline binaries after optimization.
baseline=json.loads((root/'baseline/manifest.json').read_text());audit(baseline)
baseline_benchmark=json.loads((root/'baseline/benchmark.json').read_text())
assert [v['name'] for v in baseline_benchmark['results']]==[v['name'] for v in benchmark['results']]

# Existing production artifacts did not change. Older historical target/debug
# paths may refer to still older builds; only that historical indirection skips
# mutable binaries. The immediately previous production binaries are checked.
audit(old['artifacts']);audit(old['nativeQueryArtifacts'])
for record in old['reports'].values(): audit(record);audit(json.loads(Path(record['path']).read_text()),True)
for name,record in old['componentBuilds'].items():
    audit(record);build=json.loads(Path(record['path']).read_text())
    assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    assert build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
for record in old['previousComponentBuilds'].values():audit(record);audit(json.loads(Path(record['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])

markdown=links=0
for name in sorted(sources):
    p=Path(name)
    if p.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            assert (p.parent/target).exists() or (p.parent/target).resolve()==output.resolve(),(name,target)
            links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts=[entry(p) for p in sorted(root.iterdir()) if p.is_file() and p.name not in ['next.md','seal.log','seal-check.log']]
report=dict(format='musteroffice.elliptic-scalar-stage/1',previousEvidence=entry(previous),
    scope='Standalone bounded elliptic first-membership scalar implementation and actual Native/WASM/sanitizer verification; no raster/source-page integration.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    checks=dict(scalarCases=1238,successfulCases=1230,precisionDiagnostics=1,invalidInputs=5,budgetDiagnostics=2,
        exactIntervalComparisons=12015,certifiedCandidateCases=838,maxVisitedNodes=45,
        nativeWasmSanitizerByteParity=True,asanUbsan=True,fastMathRejected=True,
        rustTestsRerun=False,schemasChanged=False,productionArtifactsUnchanged=True,
        markdownFiles=markdown,localLinks=links),
    commandChecks=verification['commands'],reports={n:entry(root/(n+'.json')) for n in ['verification','benchmark','toolchain']},
    artifacts=artifacts,baselineSnapshots=baseline,baselineManifest=entry(root/'baseline/manifest.json'),
    existingProductionArtifacts=old['artifacts'],componentBuilds=old['componentBuilds'],
    benchmark=benchmark,
    size=dict(standaloneProbeWasmBytes=entry(root/'probe.wasm')['byteLength'],
        standaloneNativeProbeBytes=entry(root/'native-probe')['byteLength'],
        existingRustWasmBytes=old['artifacts']['rustWasm']['byteLength'],existingCppWasmBytes=old['artifacts']['cppWasm']['byteLength'],
        note='Standalone test bridges are included; no linked production increment or installer size claim.'),
    limitations=verification['limitations']+[
        'General per-pixel interval solving remains too costly for interactive page rendering; precomputation/specialization/vectorization and full-page measurements remain open.',
        'Exact binary32 geometry and conservative diagnostics do not establish target application compatibility or source-parameter error bounds.',
        'Previous Rust, schema and product regressions were not rerun; all their source files and immediately previous production binaries are unchanged.',
        'Complete advanced presentation content, Agent packages and Musterwork replacement are still incomplete.'],
    verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv: assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to overwrite sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),scalarCases=1238,exactIntervalComparisons=12015,verifiedArtifacts=records)))
