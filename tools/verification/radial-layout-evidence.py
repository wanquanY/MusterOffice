"""Seal actual native radial geometry, public runtimes and unchanged prior rendering."""
import hashlib,json,re,subprocess,sys,zipfile
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
from lxml import etree

root=Path('.codex-work/radial-layout')
previous=Path('docs/reviews/evidence/2026-09-25-rectangular-gradient-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-radial-layout-verification.json')
def entry(path):
    data=Path(path).read_bytes()
    return dict(path=str(path),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
assert entry(previous)['sha256']=='a5cb99818cd8d65725e1e3e40e078fabb107141a8d2fdff648a3c9f54df0c79f'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
crates/mo-kernel-api/src/lib.rs
crates/mo-presentation-compile/src/lib.rs
crates/mo-wasm/src/lib.rs
tools/mo-cli/src/main.rs
tools/mo-contract-codegen/src/main.rs
tools/verification/contracts.py'''.splitlines())
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-presentation-compile/src/radial_layout.rs
crates/mo-presentation-compile/src/radial_layout/bounds.rs
crates/mo-presentation-compile/src/radial_layout/source.rs
crates/mo-presentation-compile/src/radial_layout/types.rs
crates/mo-presentation-compile/src/radial_layout/tests.rs
crates/mo-presentation-compile/tests/radial_layout.rs
crates/mo-kernel-api/src/pptx_radial.rs
crates/mo-kernel-api/tests/pptx_radial.rs
tools/test-support/radial_layout.rs
docs/implementation/radial-gradient-layout.md'''.splitlines())
for name in ['checks.py','reference.py','invalid.py','parity.mjs','regressions.mjs','source-regressions.mjs','evidence.py']:
    added.add('tools/verification/radial-layout-'+name)
for name in ['pptx-radial-layout-request','pptx-radial-layout-response']:
    added.add('contracts/generated/'+name+'.schema.json')
    added.add('packages/contracts/src/generated/'+name+'.ts')
assert not added&set(prior)
sources=set(prior)|added
for name in sources:
    if Path(name).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(Path(name).read_text().splitlines())<=2000,name
for path in ['Cargo.toml','Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:
    assert entry(path)==prior[path]
for path in prior:
    if path.startswith(('components/','contracts/generated/','packages/contracts/src/generated/','packages/raster-component/')):
        assert entry(path)==prior[path]

commands=json.loads((root/'checks.json').read_text())
assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for command in commands:
    text=Path(command['log']).read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),command['name']
    if command['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in text
for name in ['reference','invalid','parity','image-regressions','source-parity','text-regressions','contracts']:
    path=root/(name+'.log');text=path.read_text()
    assert text.strip() and not any(v in text for v in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(path)
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=sorted(set(tests)-set(old['rustTestNames']))
assert len(tests)==609 and set(old['rustTestNames'])<=set(tests) and len(new_tests)==8
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==82
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==82
contracts=json.loads((root/'contracts.log').read_text())
assert (contracts['schemas'],contracts['negativeMutations'])==(82,9)
cancel=json.loads((root/'native/cancellation.json').read_text());assert cancel['checkpoints']==101

reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['parity','reference','invalid','source-parity','image-regressions','regressions']}
p,r,n,s,i,t=[reports[name] for name in reports]
assert (p['pairedCalls'],p['evaluated'],p['plans'],len(p['cases']))==(42,29,31,42)
assert sum(v['status']=='error' for v in p['cases'])==13
assert (len(r['sourceFiles']),r['officialXsdParts'],r['decimalPrecision'],r['comparisons'],len(r['cases']),len(r['excluded']))==(29,203,160,513,27,4)
assert (len(n['cases']),n['officialXsdParts'])==(4,28)
assert (s['pairedCalls'],len(s['cases']),s['previousUnchanged'],sum(v['status']=='rendered' for v in s['cases']))==(139,139,139,117)
assert s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
assert i['pairedCalls']==len(i['cases'])==1116
for v in i['cases']:assert v['priorResponse']['sha256']==v['response']['sha256']
assert t['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
for v in t['cases']:
    # Earlier JSON files include a terminating newline; runtime returns omit it.
    assert Path(v['priorResponse']['path']).read_bytes().rstrip(b'\r\n')==Path(v['response']['path']).read_bytes()
previous_source=json.loads(Path(s['previous']['path']).read_text())
for v in previous_source['cases']:
    now=next(c for c in s['cases'] if c['name']=='prior/'+v['name'])
    for key in ['request','source','fonts','response','pixels','frame']:
        if key in v:assert now[key]['sha256']==v[key]['sha256']
for v in p['cases']:
    if v['status']=='evaluated':
        query=json.loads((root/'native'/(v['name']+'.query.json')).read_text())
        assert json.loads(Path(v['request']['path']).read_text())['fills']==query
        assert json.loads(Path(v['response']['path']).read_text())['plans']==json.loads((root/'native'/(v['name']+'.plan.json')).read_text())

records=0
mutable={'target/debug/mo-cli','target/debug/mo-text-worker','target/debug/mo-raster-worker'}
def audit(value,historical=False):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys() and not(historical and value['path'] in mutable):
            actual=entry(value['path']);assert all(value[k]==actual[k] for k in actual),value['path'];records+=1
        for child in value.values():audit(child,historical)
    elif isinstance(value,list):
        for child in value:audit(child,historical)
audit(reports)
native_artifacts=[entry(path) for path in sorted((root/'native').iterdir()) if path.is_file()]
audit(native_artifacts)
for record in old['reports'].values():
    audit(record);audit(json.loads(Path(record['path']).read_text()),historical=True)
# No component or dependency changes. Keep previous sanitizer results historical.
for name,record in old['componentBuilds'].items():
    audit(record);build=json.loads(Path(record['path']).read_text())
    assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    assert build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
for record in old['previousComponentBuilds'].values():
    audit(record);audit(json.loads(Path(record['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,value in old['artifacts'].items():
    if not key.startswith('native'):audit(value)

parser=etree.XMLParser(resolve_entities=False,no_network=True)
xsd=etree.XMLSchema(etree.parse('.codex-work/ecma376/xsd/pml.xsd',parser));parts=0
for record in r['sourceFiles']+[v['source'] for v in n['cases']]:
    with zipfile.ZipFile(record['path']) as package:
        for name in package.namelist():
            if not name.endswith('.xml'):continue
            node=etree.fromstring(package.read(name),parser);q=etree.QName(node)
            if q.namespace=='http://schemas.openxmlformats.org/presentationml/2006/main' and q.localname in ['presentation','sld','sldLayout','sldMaster']:
                xsd.assertValid(node);parts+=1
assert parts==231
schema_checks=0
schemas={p.stem.removesuffix('.schema'):Draft202012Validator(json.loads(p.read_text())) for p in Path('contracts/generated').glob('*.schema.json')}
assert len(schemas)==82
def validate(schema,record):
    global schema_checks
    schemas[schema].validate(json.loads(Path(record['path']).read_text()));schema_checks+=1
for v in p['cases']:
    validate('pptx-radial-layout-response',v['response'])
    if v['name'] not in ['invalid-profile','invalid-unknown-field','invalid-noncanonical-fixed','invalid-duplicate-json']:
        validate('pptx-radial-layout-request',v['request'])
for v in s['cases']:
    validate('pptx-resource-page-raster-response',v['response'])
    if not v['name'].endswith(('/duplicate-json','/unknown-profile')):validate('pptx-resource-page-request',v['request'])
assert schema_checks==356,schema_checks

markdown=links=0
for name in sorted(sources):
    path=Path(name)
    if path.suffix!='.md':continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'):continue
        target=unquote(target.split('#')[0].split('?')[0])
        if target:
            assert (path.parent/target).exists() or (path.parent/target).resolve()==output.resolve(),(name,target)
            links+=1
subprocess.run(['git','diff','--check'],check=True)
artifacts={name:entry(path) for name,path in {
    'nativeWorker':'target/debug/mo-raster-worker','nativeCli':'target/debug/mo-cli','nativeTextWorker':'target/debug/mo-text-worker',
    'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
    'cppWasm':'.codex-work/rect-gradient/component/mo-skia.wasm','typescriptRaster':str(root/'ts-raster/index.js'),
    'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
assert artifacts['rustWasm']['byteLength']==6396636 and artifacts['cppWasm']==old['artifacts']['cppWasm']
assert artifacts['typescriptRaster']['sha256']==old['artifacts']['typescriptRaster']['sha256']
report=dict(format='musteroffice.radial-layout-verification/1',previousEvidence=entry(previous),
    scope='Actual local DrawingML circle geometry and source binding with public Native/WASM/CLI queries; no final radial shading, world paint or target application acceptance.',
    sourceFiles=[entry(path) for path in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),rustTestNames=tests,newTestNames=new_tests,
    checks=dict(rustTests=609,netNewRustTests=8,strictClippy=True,rustfmt=True,schemas=82,priorSchemasUnchanged=80,runtimeSchemaChecks=schema_checks,
                radialPairedCalls=42,radialEvaluatedQueries=29,radialEvaluatedTargets=31,radialErrorQueries=13,independentTargets=27,independentScalarComparisons=513,
                decimalPrecision=160,excludedComplexPresets=4,cancellationCheckpoints=101,originalPptxFiles=33,officialXsdParts=parts,
                previousImageClipCompositeGradientPairs=1116,previousSourcePairs=139,previousPageTextPairs=307,thirdPartyVersionsUnchanged=True,
                componentArtifactsUnchanged=True,sanitizersRerun=False,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,reports={name:entry(root/(name+'.json')) for name in reports},nativeQueryArtifacts=native_artifacts,artifacts=artifacts,
    componentBuilds=old['componentBuilds'],previousComponentBuilds=old['previousComponentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    references=[dict(url='https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g',claim='Microsoft 2025-04-17 outer circle and inner ellipse; 2025-04-30 actual path bounds correction explicitly concerns center. Custom tileRect application bugs acknowledged.'),
                dict(url='https://learn.microsoft.com/en-us/answers/questions/2247174/how-is-attribute-filltorect-evaluated-for-a-gradie',claim='Focus scale and point conversion; source floating near-zero tolerance is not modeled by this exact-rational draft profile.')],
    artifactSizeChange=dict(rustWasmBytes=6396636,previousRustWasmBytes=6280243,cppWasmBytes=2327852,previousCppWasmBytes=2327852,note='Raw uncompressed modules; no installer or whole-page performance measurement.'),
    limitations=['Geometry only. Native circle shading, world placement, stationary orientation and shape gradients still require implementation.',
                 'Applying tight path bounds to tileRect and outer radius, including fill-none paths, is a development policy still needing target application calibration.',
                 'Exact zero-sum focus classification differs from the published floating near-zero branch; no Office/WPS compatibility claim.',
                 'Four complex presets have runtime parity but no independent geometric oracle here. Error bounds are relative to evaluated guides, not application or pixel errors.',
                 'The public query is atomic, but JSON/WASM wrapper cancellation is not live; explicit cancellation is tested through native library callbacks.',
                 'Complete advanced content, production Agent distribution and Musterwork replacement remain incomplete.'],verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),runtimeSchemaChecks=schema_checks,verifiedArtifacts=records)))
