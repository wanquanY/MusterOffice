"""Seal target-app observations, corrected geometry and actual public runtimes."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
root=Path('.codex-work/radial-observation')
previous=Path('docs/reviews/evidence/2026-09-25-radial-layout-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-radial-anchor-verification.json')
def entry(path):
    b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='157472cd8f9fa09a1e804a7953b5cd73070a7e0faea04ca22a6bd73c23432178'
old=json.loads(previous.read_text());prior={v['path']:v for v in old['sourceFiles']}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
crates/mo-presentation-compile/src/radial_layout.rs
crates/mo-presentation-compile/src/radial_layout/types.rs
crates/mo-presentation-compile/src/radial_layout/source.rs
crates/mo-kernel-api/src/pptx_radial.rs
contracts/generated/pptx-radial-layout-request.schema.json
packages/contracts/src/generated/pptx-radial-layout-request.ts'''.splitlines())
changed={p for p,r in prior.items() if entry(p)!=r};assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-presentation-compile/tests/radial_anchor.rs
docs/implementation/radial-gradient-calibration.md
tools/verification/radial-observation-fixtures.py
tools/verification/radial-observation-compare.py'''.splitlines())
for name in ['checks.py','parity.mjs','reference.py','regressions.mjs','source-regressions.mjs','contract-compat.mjs','evidence.py']:
    added.add('tools/verification/radial-anchor-'+name)
assert not added&set(prior);sources=set(prior)|added
for p in sources:
    if Path(p).suffix in ['.rs','.cpp','.h','.ts','.mjs','.py']:assert len(Path(p).read_text().splitlines())<=2000,p
for p in ['Cargo.toml','Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:
    assert entry(p)==prior[p]
for p in prior:
    if p.startswith(('components/','contracts/generated/','packages/contracts/src/generated/')) and p not in changed:assert entry(p)==prior[p]
subprocess.run(['node','tools/verification/radial-anchor-contract-compat.mjs'],check=True,stdout=subprocess.DEVNULL)
commands=json.loads((root/'checks.json').read_text());assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for c in commands:
    text=Path(c['log']).read_text();assert not any(s in text for s in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),c['name']
    if c['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in text
for name in ['fixtures','observations','parity','reference','image-regressions','source-parity','text-regressions','contracts','contract-compat']:
    p=root/(name+'.log');s=p.read_text();assert s.strip() and not any(x in s for x in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(p)
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=sorted(set(tests)-set(old['rustTestNames']))
assert len(tests)==611 and len(new_tests)==2 and set(old['rustTestNames'])<=set(tests)
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==82
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==82
contracts=json.loads((root/'contracts.log').read_text());assert (contracts['schemas'],contracts['negativeMutations'])==(82,9)
reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['sources','observations','parity','reference','image-regressions','source-parity','regressions','contract-compat']}
src,o,p,r,i,s,t,compat=[reports[n] for n in reports]
assert len(src['cases'])==11 and src['officialXsdParts']==77
assert len(o['cases'])==11 and o['sampledPixels']==274725
assert o['application']==dict(bundleId='com.kingsoft.wpsoffice.mac',version='12.1.22553',build='22553')
observed={v['name']:v for v in o['cases']}
for name in ['offset-point','external-point','offset-ellipse','external-ellipse','equal-width-offset']:
    models=observed[name]['models'];assert models['anchorFocusExact']['meanAbsolute']<models['squareFocus']['meanAbsolute']/4
assert observed['center-ellipse']['models']['anchorFocusExact']['meanAbsolute']<observed['center-ellipse']['models']['centeredRay']['meanAbsolute']/4
near=observed['near-equal-width']['models'];assert near['anchorFocusExact']['meanAbsolute']>5 and near['anchorFocusRoundedEqual']['meanAbsolute']<.2
assert all(v['models']['anchorFocusExact']['sampledInsideToOutsideTransitions']==0 for v in o['cases'])
assert (p['pairedCalls'],p['priorUnchanged'],p['extendedProfileDiagnostics'])==(82,41,1)
assert len(p['cases'])==82 and sum(v['status']=='evaluated' for v in p['cases'])==69 and sum(v['plans'] for v in p['cases'])==73
for v in p['cases']:
    if 'priorResponse' in v and v['name']!='prior-invalid-profile':assert v['response']['sha256']==v['priorResponse']['sha256']
assert (len(r['sourceFiles']),r['officialXsdParts'],len(r['cases']),len(r['excluded']),r['comparisons'],r['decimalPrecision'])==(40,280,38,4,722,160)
assert i['pairedCalls']==len(i['cases'])==1116
for v in i['cases']:assert v['priorResponse']['sha256']==v['response']['sha256']
assert (s['pairedCalls'],s['previousUnchanged'],len(s['cases']))==(139,139,139)
assert sum(v['status']=='rendered' for v in s['cases'])==117 and s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
assert t['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
for v in t['cases']:assert Path(v['priorResponse']['path']).read_bytes().rstrip(b'\r\n')==Path(v['response']['path']).read_bytes()
for v in json.loads(Path(s['previous']['path']).read_text())['cases']:
    now=next(n for n in s['cases'] if n['name']=='prior/'+v['name'])
    for key in ['source','request','fonts','response','frame','pixels']:
        if key in v:assert now[key]['sha256']==v[key]['sha256']
assert compat['priorSchemaSha256']==prior['contracts/generated/pptx-radial-layout-request.schema.json']['sha256']
for v in compat['restoredTypes']:assert v['sha256']==prior[v['historicalPath']]['sha256']

records=0;mutable={'target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker'}
def audit(value,historical=False):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys() and not(historical and value['path'] in mutable):
            a=entry(value['path']);assert all(value[k]==a[k] for k in a),value['path'];records+=1
        for v in value.values():audit(v,historical)
    elif isinstance(value,list):
        for v in value:audit(v,historical)
audit(reports)
native_artifacts=[entry(p) for p in sorted((root/'native').iterdir()) if p.is_file()];audit(native_artifacts)
for v in old['reports'].values():audit(v);audit(json.loads(Path(v['path']).read_text()),True)
audit(old['nativeQueryArtifacts'])
for name,v in old['componentBuilds'].items():
    audit(v);b=json.loads(Path(v['path']).read_text());assert b['lock']==json.loads(Path('components/skia/lock.json').read_text())
    assert b['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(b[key])
for v in old['previousComponentBuilds'].values():audit(v);audit(json.loads(Path(v['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,v in old['artifacts'].items():
    if not key.startswith('native'):audit(v)
schema_checks=0
schemas={p.stem.removesuffix('.schema'):Draft202012Validator(json.loads(p.read_text())) for p in Path('contracts/generated').glob('*.schema.json')}
def validate(schema,record):
    global schema_checks
    schemas[schema].validate(json.loads(Path(record['path']).read_text()));schema_checks+=1
for v in p['cases']:
    validate('pptx-radial-layout-response',v['response'])
    if v['name'] not in ['prior-invalid-profile','prior-invalid-unknown-field','prior-invalid-noncanonical-fixed','prior-invalid-duplicate-json']:validate('pptx-radial-layout-request',v['request'])
for v in s['cases']:
    validate('pptx-resource-page-raster-response',v['response'])
    if not v['name'].endswith(('/duplicate-json','/unknown-profile')):validate('pptx-resource-page-request',v['request'])
assert schema_checks==436,schema_checks
markdown=links=0
for name in sorted(sources):
    path=Path(name)
    if path.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            assert (path.parent/target).exists() or (path.parent/target).resolve()==output.resolve(),(name,target);links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts={k:entry(v) for k,v in {
    'nativeCli':'target/debug/mo-cli','nativeWorker':'target/debug/mo-raster-worker','nativeTextWorker':'target/debug/mo-text-worker',
    'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
    'cppWasm':'.codex-work/rect-gradient/component/mo-skia.wasm','typescriptRaster':str(root/'ts-raster/index.js'),
    'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
assert artifacts['rustWasm']['byteLength']==6398266 and artifacts['cppWasm']==old['artifacts']['cppWasm']
assert artifacts['typescriptRaster']['sha256']==old['artifacts']['typescriptRaster']['sha256']
report=dict(format='musteroffice.radial-anchor-verification/1',previousEvidence=entry(previous),
    scope='WPS geometry calibration and explicit V2 native anchor-focus queries; production radial pixel shader and complete target application acceptance remain incomplete.',
    sourceFiles=[entry(n) for n in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),rustTestNames=tests,newTestNames=new_tests,
    checks=dict(rustTests=611,netNewRustTests=2,strictClippy=True,rustfmt=True,schemas=82,extendedProfileSchemas=1,priorUnchangedSchemas=81,runtimeSchemaChecks=schema_checks,
                radialPairedCalls=82,radialEvaluatedQueries=69,radialEvaluatedTargets=73,radialErrorQueries=13,priorQueryResponsesUnchanged=41,extendedProfileDiagnostics=1,
                independentTargets=38,independentScalarComparisons=722,decimalPrecision=160,excludedComplexPresets=4,originalPptxFiles=40,officialXsdParts=280,
                wpsObservedFiles=11,wpsSampledPixels=274725,previousImageClipCompositeGradientPairs=1116,previousSourcePairs=139,previousPageTextPairs=307,
                thirdPartyVersionsUnchanged=True,componentArtifactsUnchanged=True,sanitizersRerun=False,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,reports={n:entry(root/(n+'.json')) for n in reports},nativeQueryArtifacts=native_artifacts,artifacts=artifacts,
    componentBuilds=old['componentBuilds'],previousComponentBuilds=old['previousComponentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    artifactSizeChange=dict(rustWasmBytes=6398266,previousRustWasmBytes=6396636,cppWasmBytes=2327852,previousCppWasmBytes=2327852,note='Raw uncompressed modules; no installer or full-page performance measurement.'),
    limitations=['Native radial geometry query only; no production ellipse scalar shader, world painting or page integration completed in this stage.',
                 'WPS observations cover these axis-aligned rectangle cases only and retain resampling/registration uncertainty. PowerPoint has not been tested.',
                 'Near-equal source extents differ from WPS; exact classification remains intentional pending numerical semantics evidence, not declared compatible.',
                 'The ellipse interpolation probe is an observational hypothesis using bounded sampling/bisection, not a certified general quartic solver.',
                 'Four complex presets have parity but no independent geometry oracle here; actual path/tile/stationary and advanced content acceptance remain open.',
                 'Complete Agent packages, advanced presentation content and Musterwork replacement remain incomplete.'],verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),runtimeSchemaChecks=schema_checks,verifiedArtifacts=records)))
