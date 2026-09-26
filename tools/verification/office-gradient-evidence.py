"""Seal Office channel curves, exact JSON inputs and actual public runtimes."""
import hashlib,json,re,subprocess,sys,zipfile
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
from lxml import etree

root=Path('.codex-work/office-gradient')
previous=Path('docs/reviews/evidence/2026-09-25-paint-admission-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-office-gradient-verification.json')
def entry(path):
    data=Path(path).read_bytes()
    return dict(path=str(path),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
def sha(text):return hashlib.sha256(text.encode()).hexdigest()
assert entry(previous)['sha256']=='7830da05acc16068f3398dd9615fdde74e066cbb0f6833c021bff0472de87b54'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''Cargo.toml
README.md
components/skia/mo_gradient.cpp
components/skia/mo_gradient.h
components/skia/mo_skia.cpp
components/skia/mo_skia.h
crates/mo-harfbuzz-sys/tests/source_gradient_page.rs
crates/mo-presentation-compile/src/source_page/gradient.rs
crates/mo-raster/src/brush.rs
crates/mo-raster/src/compile.rs
crates/mo-raster/src/gradient.rs
crates/mo-raster/src/lib.rs
crates/mo-skia-sys/src/ffi.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md
packages/raster-component/src/index.ts
tools/components/build-skia.py
tools/mo-cli/src/raster.rs'''.splitlines())
contracts_changed=['image-raster-request','image-scene-request','page-compile-response','path-raster-request','pptx-page-compile-response','scene-raster-request']
for name in contracts_changed:
    allowed.add('contracts/generated/'+name+'.schema.json')
    allowed.add('packages/contracts/src/generated/'+name+'.ts')
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''components/skia/mo_office_gradient.cpp
components/skia/mo_office_gradient_stage.h
components/skia/office-gradient.patch
crates/mo-raster/src/office_gradient.rs
crates/mo-harfbuzz-sys/tests/office_gradient.rs
tools/test-support/office_gradient.rs
docs/implementation/office-gradient-interpolation.md'''.splitlines())
for name in ['checks.py','components.mjs','json.mjs','observe.py','parity.mjs','reference.py','regressions.mjs','source-parity.mjs','evidence.py']:
    added.add('tools/verification/office-gradient-'+name)
assert not added&set(prior)
sources=set(prior)|added
for path in sources:
    if Path(path).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(Path(path).read_text().splitlines())<=2000,path
for path in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:
    assert entry(path)==prior[path]
assert sha(Path('Cargo.toml').read_text().replace('serde_json = { version = "=1.0.151", features = ["float_roundtrip"] }','serde_json = "=1.0.151"'))==prior['Cargo.toml']['sha256']
for path in Path('contracts/generated').glob('*.schema.json'):
    data=json.loads(path.read_text())
    if str(path) in changed:
        rule=data['$defs']['GradientInterpolation']
        assert rule['oneOf'][0]==dict(type='string',enum=['srgb','linearSrgb'])
        assert rule['oneOf'][1]['const']=='officeGamma1875' and len(rule['oneOf'])==2
        data['$defs']['GradientInterpolation']=dict(type='string',enum=['srgb','linearSrgb'])
        assert sha(json.dumps(data,ensure_ascii=False,indent=2)+'\n')==prior[str(path)]['sha256']
    else:assert entry(path)==prior[str(path)]
for path in Path('packages/contracts/src/generated').glob('*.ts'):
    text=path.read_text().replace('("srgb" | "linearSrgb") | "officeGamma1875"','"srgb" | "linearSrgb"')
    assert sha(text)==prior[str(path)]['sha256']

commands=json.loads((root/'checks.json').read_text())
assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for command in commands:
    text=Path(command['log']).read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),command['name']
    if command['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in text
for name in ['native-build','wasm-build','asan-build','parity','source-parity','components','json-regression','image-regressions','text-regressions','reference','contracts','observations']:
    path=root/(name+'.log');text=path.read_text()
    assert text.strip() and not any(v in text for v in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(path)
logs['application-conversion']=entry(root/'libreoffice-convert.log')
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=sorted(set(tests)-set(old['rustTestNames']))
assert len(tests)==595 and set(old['rustTestNames'])<=set(tests) and len(new_tests)==5
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==80
contracts=json.loads((root/'contracts.log').read_text())
assert (contracts['schemas'],contracts['negativeMutations'])==(80,9)

reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['parity','source-parity','components','reference','image-regressions','regressions','json-regression','observations']}
p,s,c,r,i,t,j,o=[reports[n] for n in reports]
assert (p['pairedCalls'],len(p['cases']),p['verifiedPixels'],p['maximumDifference'],len(p['negatives']))==(157,74,314568,1,9)
assert p['cli']['create'] and p['cli']['overwriteRefused']
assert (s['pairedCalls'],len(s['cases']),s['previousUnchanged'],len(s['previousColorCorrected']),sum(v['status']=='rendered' for v in s['cases']))==(121,121,95,16,99)
assert s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
assert (c['triples'],c['legacyFrames'],len(c['negatives']),c['oldCapabilityRejections'])==(398,228,22,2)
assert c['addressSanitizer'] and c['undefinedBehaviorSanitizer'] and not c['leakSanitizer'] and c['noDiagnostics']
assert (len(r['sourceFiles']),r['officialXsdParts'],r['verifiedPixels'],len(r['cases']))==(10,70,1200000,10)
assert max(v['maximumChannelDifference'] for v in r['cases'])==1
assert i['pairedCalls']==len(i['cases'])==854
for value in i['cases']:assert value['priorResponse']['sha256']==value['response']['sha256']
assert t['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)
assert len(j['cases'])==2
for value in j['cases']:
    assert value['before']['calls']==1 and value['after']['calls']==0
    assert json.loads(Path(value['before']['response']['path']).read_text())['status']=='rendered'
    assert json.loads(Path(value['after']['response']['path']).read_text())['status']=='error'
    assert value['after']['pixels']['byteLength']==0
assert len(o['cases'])==10 and all(v['pageCount']==2 and v['pixelsDifferent']>0 for v in o['cases'])

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
    for key in ['request','source','fonts']:assert now[key]['sha256']==value[key]['sha256']
    if value['name'] not in s['previousColorCorrected']:
        for key in ['response','pixels','frame']:
            if key in value:assert now[key]['sha256']==value[key]['sha256']
builds={name:entry(root/'component'/(name+'-build.json')) for name in ['native','wasm','native-asan']}
for name,record in builds.items():
    build=json.loads(Path(record['path']).read_text())
    assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    assert build['sanitizers']==(name=='native-asan')
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
    assert any(v['path']=='components/skia/mo_office_gradient_stage.h' for v in build['componentSources'])
for record in old['componentBuilds'].values():
    audit(record);audit(json.loads(Path(record['path']).read_text())['artifacts'])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,value in old['artifacts'].items():
    if not key.startswith('native'):audit(value)

# Re-run official schemas against each native source, not only a stored count.
parser=etree.XMLParser(resolve_entities=False,no_network=True)
xsd=etree.XMLSchema(etree.parse('.codex-work/ecma376/xsd/pml.xsd',parser));parts=0
for record in r['sourceFiles']:
    with zipfile.ZipFile(record['path']) as package:
        for name in package.namelist():
            if not name.endswith('.xml'):continue
            node=etree.fromstring(package.read(name),parser);q=etree.QName(node)
            if q.namespace=='http://schemas.openxmlformats.org/presentationml/2006/main' and q.localname in ['presentation','sld','sldLayout','sldMaster']:
                xsd.assertValid(node);parts+=1
assert parts==70
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
    if value['name']!='nan-color':validate('path-raster-request',value['request'])
for value in s['cases']:
    validate('pptx-resource-page-raster-response',value['response'])
    if not value['name'].endswith(('/duplicate-json','/unknown-profile')):validate('pptx-resource-page-request',value['request'])
assert schema_checks==553,schema_checks

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
assert artifacts['rustWasm']['byteLength']==6261069 and artifacts['cppWasm']['byteLength']==2327536
report=dict(format='musteroffice.office-gradient-verification/1',previousEvidence=entry(previous),
    scope='Explicit Office 15/8 channel interpolation, native source eligibility and round-trip-safe JSON float parsing. Scalar geometry, clipping, composition and native source declarations remain intact. Not target Office/WPS or complete PPT acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),rustTestNames=tests,newTestNames=new_tests,
    checks=dict(rustTests=595,netNewRustTests=5,strictClippy=True,rustfmt=True,schemas=80,changedSchemaEnums=6,runtimeSchemaChecks=schema_checks,
                sharedPairedCalls=157,sourcePairedCalls=121,previousSourceUnchanged=95,previousSourceColorCorrected=16,previousImageClipCompositeGradientPairs=854,previousPageTextPairs=307,
                independentSharedPixels=314568,independentSourcePixels=1200000,maximumReferenceChannelDifference=1,originalPptxFiles=10,officialXsdParts=70,
                componentTriples=398,legacyFrames=228,addressSanitizer=True,undefinedBehaviorSanitizer=True,leakSanitizer=False,jsonBeforeAfterCases=2,
                applicationFiles=10,thirdPartyVersionsUnchanged=True,serdeJsonFloatRoundtripEnabled=True,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,reports={name:entry(root/(name+'.json')) for name in reports},artifacts=artifacts,
    componentBuilds=builds,previousComponentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    normativeReferences=[dict(url='https://learn.microsoft.com/en-us/answers/questions/2248059/non-preset-a-tilerect-behaves-strange-in-case-of-g',claim='Microsoft Open Specifications 2025-04-17: 1.875 RGB interpolation for endpoint pairs and symmetric triples; alpha linear. Later 2025-04-30 correction says radial center uses shape path bounds; path implementation remains open.')],
    artifactSizeChange=dict(rustWasmBytes=6261069,previousRustWasmBytes=6214446,cppWasmBytes=2327536,previousCppWasmBytes=2325263,note='Raw uncompressed components. Rust change includes exact decimal parsing feature; no installer or page performance measurement.'),
    limitations=['Generic radial geometry is not native DrawingML path-gradient semantics. Native circle/rect/shape and stationary orientation remain incomplete.',
                 'The explicit special-curve selector compares all RGBA working values; target application behavior for unequal endpoint alpha and extended channels still needs measurement.',
                 'Independent mathematical references and CPU/WASM parity do not establish Office/WPS interoperability. LibreOffice color and alpha differences remain recorded and unresolved.',
                 'Component sanitizers cover the executed cases, not all possible document inputs; no LeakSanitizer claim.',
                 'Complete advanced content, production Agent packaging and Musterwork replacement remain incomplete.'],verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),runtimeSchemaChecks=schema_checks,verifiedArtifacts=records)))
