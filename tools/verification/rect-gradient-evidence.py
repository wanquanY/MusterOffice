"""Seal native rectangular fields, precise rates, current public runtimes and retained evidence."""
import hashlib,json,re,subprocess,sys,zipfile
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
from lxml import etree

root=Path('.codex-work/rect-gradient')
previous=Path('docs/reviews/evidence/2026-09-25-office-gradient-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-rectangular-gradient-verification.json')
def entry(path):
    data=Path(path).read_bytes()
    return dict(path=str(path),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
def sha(text):return hashlib.sha256(text.encode()).hexdigest()
assert entry(previous)['sha256']=='d8e0dbdf6cec852bf651299752f87e9338ee625d78a446fc468dbd6288a6baf9'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''README.md
components/skia/mo_gradient.cpp
components/skia/mo_gradient.h
components/skia/mo_gradient_plane.cpp
components/skia/mo_gradient_plane_stage.h
components/skia/mo_office_gradient.cpp
components/skia/mo_skia.cpp
components/skia/mo_skia.h
crates/mo-presentation-compile/src/source_page.rs
crates/mo-presentation-compile/src/source_page/gradient.rs
crates/mo-raster/src/compile.rs
crates/mo-raster/src/gradient.rs
crates/mo-raster/src/gradient_plane.rs
crates/mo-raster/src/gradient_plane_tests.rs
crates/mo-raster/src/lib.rs
crates/mo-raster/src/number.rs
crates/mo-skia-sys/src/ffi.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
packages/raster-component/src/index.ts
tools/components/build-skia.py
tools/mo-cli/src/raster.rs
tools/verification/skia-probe.cpp'''.splitlines())
contracts_changed=['image-raster-request','image-scene-request','page-compile-response','path-raster-request','pptx-page-compile-response','scene-raster-request']
for name in contracts_changed:
    allowed.add('contracts/generated/'+name+'.schema.json')
    allowed.add('packages/contracts/src/generated/'+name+'.ts')
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''components/skia/rect-gradient.patch
crates/mo-raster/src/gradient_rate.rs
crates/mo-raster/src/gradient_rect_tests.rs
crates/mo-presentation-compile/src/source_page/gradient_rect.rs
crates/mo-harfbuzz-sys/tests/rect_gradient.rs
tools/test-support/rect_gradient.rs
docs/implementation/rectangular-gradients.md
packages/contracts/src/generated/pptx-page-compile-response/part-001.ts
packages/contracts/src/generated/pptx-page-compile-response/part-002.ts'''.splitlines())
for name in ['checks.py','components.mjs','contract-compat.mjs','observe.py','parity.mjs','reference.py','regressions.mjs','source-parity.mjs','evidence.py']:
    added.add('tools/verification/rect-gradient-'+name)
assert not added&set(prior)
sources=set(prior)|added
for path in sources:
    if Path(path).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(Path(path).read_text().splitlines())<=2000,path
for path in ['Cargo.toml','Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:
    assert entry(path)==prior[path]
assert 'features = ["float_roundtrip"]' in Path('Cargo.toml').read_text()
for path in Path('contracts/generated').glob('*.schema.json'):
    data=json.loads(path.read_text())
    if str(path) in changed:
        rule=data['$defs']['GradientField']
        assert len(rule['oneOf'])==2 and rule['oneOf'][1]['properties']['kind']['const']=='rectangular'
        rule['oneOf']=rule['oneOf'][:1]
        assert sha(json.dumps(data,ensure_ascii=False,indent=2)+'\n')==prior[str(path)]['sha256']
    else:assert entry(path)==prior[str(path)]
# Regenerate prior TypeScript from exactly reverted schemas, including automatic splitting.
subprocess.run(['node','tools/verification/rect-gradient-contract-compat.mjs'],check=True,stdout=subprocess.DEVNULL)

commands=json.loads((root/'checks.json').read_text())
assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for command in commands:
    text=Path(command['log']).read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),command['name']
    if command['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in text
for name in ['native-component','wasm-component','asan-component','parity','source-parity','components','image-regressions','text-regressions','reference','contracts','contract-compat','observations']:
    path=root/(name+'.log');text=path.read_text()
    assert text.strip() and not any(v in text for v in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(path)
logs['application-conversion']=entry(root/'libreoffice-convert.log')
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=sorted(set(tests)-set(old['rustTestNames']))
assert len(tests)==601 and set(old['rustTestNames'])<=set(tests) and len(new_tests)==6
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==80
contracts=json.loads((root/'contracts.log').read_text())
assert (contracts['schemas'],contracts['negativeMutations'])==(80,9)

reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['parity','source-parity','components','reference','image-regressions','regressions','contract-compat','observations']}
p,s,c,r,i,t,j,o=[reports[n] for n in reports]
assert (p['pairedCalls'],len(p['cases']),p['verifiedPixels'],p['maximumDifference'],len(p['negatives']))==(121,57,568998,1,7)
assert all(p['cli'][v] for v in ['create','overwriteRefused','failureOutputAbsent'])
assert (s['pairedCalls'],len(s['cases']),s['previousUnchanged'],sum(v['status']=='rendered' for v in s['cases']))==(139,139,121,117)
assert s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
assert (c['triples'],c['legacyFrames'],c['sourceFrames'],len(c['negatives']),c['oldCapabilityRejections'])==(533,376,18,25,2)
assert c['addressSanitizer'] and c['undefinedBehaviorSanitizer'] and not c['leakSanitizer'] and c['noDiagnostics']
assert (len(r['sourceFiles']),r['officialXsdParts'],r['verifiedPixels'],len(r['cases']))==(18,126,1800000,15)
assert max(v['maximumChannelDifference'] for v in r['cases'])==1
assert i['pairedCalls']==len(i['cases'])==1002
for value in i['cases']:assert value['priorResponse']['sha256']==value['response']['sha256']
assert t['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
assert len(j['cases'])==6
for value in j['cases']:
    assert value['priorSchemaSha256']==prior[value['schema']['path']]['sha256']
    for v in value['restoredTypeFiles']:
        assert v['sha256']==prior[v['historicalPath']]['sha256'] and v['byteLength']==prior[v['historicalPath']]['byteLength']
assert len(o['cases'])==17 and all(v['pageCount']==2 and v['pixelsDifferent']>0 for v in o['cases'])
assert max(v['maximumChannelDifference'] for v in o['cases'])==255

records=0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            actual=entry(value['path']);assert all(value[k]==actual[k] for k in actual),value['path'];records+=1
        for child in value.values():audit(child)
    elif isinstance(value,list):
        for child in value:audit(child)
audit(reports)
previous_source=json.loads(Path(s['previous']['path']).read_text())
for value in previous_source['cases']:
    now=next(v for v in s['cases'] if v['name']=='prior/'+value['name'])
    for key in ['request','source','fonts','response','pixels','frame']:
        if key in value:assert now[key]['sha256']==value[key]['sha256']
builds={name:entry(root/'component'/(name+'-build.json')) for name in ['native','wasm','native-asan']}
for name,record in builds.items():
    build=json.loads(Path(record['path']).read_text())
    assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    assert build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
    assert any(v['path']=='components/skia/rect-gradient.patch' for v in build['componentSources'])
for record in old['componentBuilds'].values():
    audit(record);audit(json.loads(Path(record['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,value in old['artifacts'].items():
    if not key.startswith('native'):audit(value)
# The previous verification reports and their owned artifacts remain intact.
for name,record in old['reports'].items():
    audit(record);historical=json.loads(Path(record['path']).read_text())
    if name=='regressions':
        # Mutable target/debug executables were rebuilt above; their old digest
        # remains historical, while all owned inputs and outputs stay sealed.
        historical['artifacts']=[v for v in historical['artifacts'] if v['path'] not in ['target/debug/mo-cli','target/debug/mo-text-worker','target/debug/mo-raster-worker']]
    audit(historical)

parser=etree.XMLParser(resolve_entities=False,no_network=True)
xsd=etree.XMLSchema(etree.parse('.codex-work/ecma376/xsd/pml.xsd',parser));parts=0
for record in r['sourceFiles']:
    with zipfile.ZipFile(record['path']) as package:
        for name in package.namelist():
            if not name.endswith('.xml'):continue
            node=etree.fromstring(package.read(name),parser);q=etree.QName(node)
            if q.namespace=='http://schemas.openxmlformats.org/presentationml/2006/main' and q.localname in ['presentation','sld','sldLayout','sldMaster']:
                xsd.assertValid(node);parts+=1
assert parts==126
schema_checks=0
schemas={p.stem.removesuffix('.schema'):Draft202012Validator(json.loads(p.read_text())) for p in Path('contracts/generated').glob('*.schema.json')}
def validate(schema,record):
    global schema_checks
    schemas[schema].validate(json.loads(Path(record['path']).read_text()));schema_checks+=1
for value in p['cases']:
    validate('path-raster-request',value['request']);validate('path-raster-response',value['response'])
    validate('scene-raster-request',value['sceneRequest']);validate('scene-raster-response',value['sceneResponse'])
for value in p['negatives']:
    validate('path-raster-response',value['response'])
    if value['name']!='wrong-length':validate('path-raster-request',value['request'])
for value in s['cases']:
    validate('pptx-resource-page-raster-response',value['response'])
    if not value['name'].endswith(('/duplicate-json','/unknown-profile')):validate('pptx-resource-page-request',value['request'])
assert schema_checks==517,schema_checks

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
    'cppWasm':str(root/'component/mo-skia.wasm'),'typescriptRaster':str(root/'ts-raster/index.js'),
    'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
assert artifacts['rustWasm']['byteLength']==6280243 and artifacts['cppWasm']['byteLength']==2327852
report=dict(format='musteroffice.rectangular-gradient-verification/1',previousEvidence=entry(previous),
    scope='Native rectangular focus geometry and shared four-edge fields, precise dimensionless rate conversion, V11 and public Native/WASM/CLI. Does not establish target Office/WPS or complete PPT acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),rustTestNames=tests,newTestNames=new_tests,
    checks=dict(rustTests=601,netNewRustTests=6,strictClippy=True,rustfmt=True,schemas=80,extendedFieldSchemas=6,runtimeSchemaChecks=schema_checks,
                sharedPairedCalls=121,sourcePairedCalls=139,previousSourceUnchanged=121,previousImageClipCompositeGradientPairs=1002,previousPageTextPairs=307,
                independentSharedPixels=568998,independentSourcePixels=1800000,maximumReferenceChannelDifference=1,originalPptxFiles=18,officialXsdParts=126,
                componentTriples=533,legacyFrames=376,sourceComponentFrames=18,addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,
                applicationFiles=17,thirdPartyVersionsUnchanged=True,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,reports={name:entry(root/(name+'.json')) for name in reports},artifacts=artifacts,
    componentBuilds=builds,previousComponentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    normativeReferences=[dict(url='https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g',claim='Microsoft Open Specifications 2025-04-17: rectangular focus/anchor interpolation and first color within focus area. Later radial path-bounds correction and tileRect application bugs remain relevant to subsequent work.')],
    artifactSizeChange=dict(rustWasmBytes=6280243,previousRustWasmBytes=6261069,cppWasmBytes=2327852,previousCppWasmBytes=2327536,note='Raw uncompressed components; no installer or full-page performance measurement.'),
    limitations=['Rectangular development profile uses the receiver logical plane. Other native circle/shape gradients, stationary orientation and distinct custom-path bounds remain incomplete.',
                 'Inverted focus rectangles are explicitly diagnosed before image decoding; their target application semantics are not implemented.',
                 'The independent references verify the chosen geometry and curves, not Office/WPS interoperability. LibreOffice differences remain recorded and unresolved.',
                 'Component sanitizers cover executed cases, not all possible document inputs; no LeakSanitizer claim.',
                 'Complete advanced content, production Agent packaging and Musterwork replacement remain incomplete.'],verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),runtimeSchemaChecks=schema_checks,verifiedArtifacts=records)))
