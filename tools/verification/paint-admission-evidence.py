"""Seal shared paint ownership, early admission and unchanged public behavior."""
import hashlib,json,re,statistics,subprocess,sys,zipfile
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
from lxml import etree

root=Path('.codex-work/paint-admission')
previous=Path('docs/reviews/evidence/2026-09-25-native-linear-gradient-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-paint-admission-verification.json')
def entry(path):
    data=Path(path).read_bytes()
    return dict(path=str(path),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
assert entry(previous)['sha256']=='423490428df2329daa7ca8a335258e54ba11289ebd68f0444b9d2acbbdf5e8f3'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''README.md
crates/mo-presentation-compile/src/lib.rs
crates/mo-presentation-compile/src/path_scene.rs
crates/mo-presentation-compile/src/source_page/gradient.rs
crates/mo-presentation-compile/src/source_page/paint.rs
crates/mo-presentation-compile/src/source_page/prepared.rs
crates/mo-raster/src/brush.rs
crates/mo-raster/src/compile.rs
crates/mo-raster/src/gradient.rs
crates/mo-raster/src/gradient_tests.rs
crates/mo-raster/src/lib.rs
crates/mo-render/src/compile.rs
docs/README.md
docs/implementation/development.md
docs/implementation/progress.md'''.splitlines())
assert changed==allowed,(changed-allowed,allowed-changed)
added=set('''crates/mo-raster/src/gradient_stops.rs
crates/mo-raster/src/paint_budget.rs
crates/mo-raster/examples/gradient_storage.rs
crates/mo-presentation-compile/src/path_scene_tests.rs
crates/mo-presentation-compile/tests/paint_admission.rs
tools/test-support/paint_admission.rs
docs/implementation/paint-storage-admission.md
tools/verification/paint-admission-checks.py
tools/verification/paint-admission-parity.mjs
tools/verification/paint-admission-regressions.mjs
tools/verification/paint-storage-measure.py
tools/verification/paint-admission-evidence.py'''.splitlines())
assert not added&set(prior)
sources=set(prior)|added
for path in sources:
    if Path(path).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(Path(path).read_text().splitlines())<=2000,path
for path in ['Cargo.lock','pnpm-lock.yaml','components/skia/lock.json','components/image-codec/lock.json']:
    assert entry(path)==prior[path]
for pattern in ['contracts/generated/*.schema.json','packages/contracts/src/generated/*.ts']:
    paths=list(Path('.').glob(pattern));assert len(paths)==80
    for path in paths:assert entry(path)==prior[str(path)]

commands=json.loads((root/'checks.json').read_text())
assert len(commands)==11 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for command in commands:
    text=Path(command['log']).read_text()
    assert not any(v in text for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),command['name']
    if command['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in text
for name in ['storage-build','storage','parity','image-regressions','text-regressions','contracts']:
    path=root/(name+'.log');text=path.read_text()
    assert text.strip() and not any(v in text for v in ['error:','FAILED','Traceback','AssertionError']),name
    logs[name]=entry(path)
assert 'Finished' in (root/'storage-build.log').read_text()
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=sorted(set(tests)-set(old['rustTestNames']))
assert len(tests)==590 and set(old['rustTestNames'])<=set(tests) and len(new_tests)==6
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
assert len(re.findall(r'^check .+\.ts$',(root/'types-check.log').read_text(),re.M))==80
contracts=json.loads((root/'contracts.log').read_text())
assert (contracts['schemas'],contracts['negativeMutations'])==(80,9)

reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['source-parity','image-regressions','regressions','storage','source-xsd']}
s,i,t,m,x=[reports[n] for n in reports]
assert (s['pairedCalls'],len(s['cases']),s['previousUnchanged'],sum(v['status']=='rendered' for v in s['cases']))==(111,111,107,89)
assert not s['previousNewlySupported']
assert s['cli']['overwriteExit']==1 and s['cli']['failureOutputAbsent']
fresh={v['name'].removeprefix('new/'):v for v in s['cases'] if v['name'].startswith('new/')}
assert set(fresh)=={'at-limit','unpainted','over-limit','background'}
for name,value in fresh.items():
    expected='error' if name in ['over-limit','background'] else 'rendered'
    assert value['status']==expected
    assert value['decodes']==value['rasters']==(0 if expected=='error' else 1)
    if expected=='error':assert 'gradient input stops' in Path(value['response']['path']).read_text()
assert fresh['at-limit']['pixels']['sha256']==fresh['unpainted']['pixels']['sha256']
previous_source=json.loads(Path(s['previous']['path']).read_text())
for value in previous_source['cases']:
    now=next(v for v in s['cases'] if v['name']=='prior/'+value['name'])
    for key in ['request','source','fonts','response','pixels','frame']:
        if key in value:assert now[key]['sha256']==value[key]['sha256'],(value['name'],key)
assert i['pairedCalls']==len(i['cases'])==854
assert {kind:sum(v['kind']==kind for v in i['cases']) for kind in ['image','clip','composite','gradient']}==dict(image=626,clip=100,composite=68,gradient=60)
for value in i['cases']:assert value['priorResponse']['sha256']==value['response']['sha256']
assert t['counts']==dict(requests=307,pageRequests=100,textRequests=207,pixelOutputs=21)

records=0
def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            actual=entry(value['path'])
            assert all(value[k]==actual[k] for k in actual),value['path']
            records+=1
        for child in value.values():audit(child)
    elif isinstance(value,list):
        for child in value:audit(child)
audit(reports)
for name,record in old['componentBuilds'].items():
    audit(record);build=json.loads(Path(record['path']).read_text())
    assert build['lock']==json.loads(Path('components/skia/lock.json').read_text())
    for key in ['componentSources','artifacts','imageCodecs']:audit(build[key])
for key in ['standardInputs','codecInputLock','previousReleaseArtifactsVerifiedUnchanged']:audit(old[key])
for key,value in old['artifacts'].items():
    if not key.startswith('native'):audit(value)

# Re-run official XML validation, rather than treating the earlier summary as proof.
parser=etree.XMLParser(resolve_entities=False,no_network=True)
xsd=etree.XMLSchema(etree.parse('.codex-work/ecma376/xsd/pml.xsd',parser))
xsd_files=[]
for path in sorted((root/'native').glob('*.pptx')):
    parts=[]
    with zipfile.ZipFile(path) as package:
        for name in sorted(package.namelist()):
            if not name.endswith('.xml'):continue
            node=etree.fromstring(package.read(name),parser)
            qname=etree.QName(node)
            if qname.namespace=='http://schemas.openxmlformats.org/presentationml/2006/main' and qname.localname in ['presentation','sld','sldLayout','sldMaster']:
                xsd.assertValid(node);parts.append(name)
    assert len(parts)==7
    xsd_files.append(dict(**entry(path),parts=parts))
assert len(xsd_files)==4
assert x==dict(files=xsd_files,officialXsdParts=28)

schema_checks=0
schemas={p.stem.removesuffix('.schema'):Draft202012Validator(json.loads(p.read_text())) for p in Path('contracts/generated').glob('*.schema.json')}
def validate(schema,record):
    global schema_checks
    schemas[schema].validate(json.loads(Path(record['path']).read_text()));schema_checks+=1
for value in s['cases']:
    validate('pptx-resource-page-raster-response',value['response'])
    if not value['name'].endswith(('/duplicate-json','/unknown-profile')):
        validate('pptx-resource-page-request',value['request'])
assert schema_checks==220,schema_checks

assert len(m['cases'])==14 and len({v['checksum'] for v in m['cases']})==1
assert m['platform']['system']=='Darwin'
measurements={}
for mode,count in [('shared',1),('copied-array-model',64)]:
    rows=[v for v in m['cases'] if v['mode']==mode]
    assert len(rows)==7 and {v['repetition'] for v in rows}==set(range(7))
    for value in rows:
        assert (value['instances'],value['stopsPerInstance'],value['stopBytes'],value['retainedStopBuffers'],value['retainedStopPayloadBytes'])==(64,4096,40,count,count*4096*40)
        log=Path(value['log']['path']).read_text()
        assert json.loads(log.splitlines()[0])=={k:v for k,v in value.items() if k not in ['repetition','maximumResidentBytes','log']}
        rss=[int(line.split()[0]) for line in log.splitlines() if 'maximum resident set size' in line]
        assert rss==[value['maximumResidentBytes']]
    times=sorted(v['constructionNanos'] for v in rows)
    measurements[mode]=dict(stopPayloadBytes=count*4096*40,constructionNanos=dict(median=statistics.median(times),min=min(times),max=max(times)),maximumResidentBytes=sorted(v['maximumResidentBytes'] for v in rows))

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
    'cppWasm':'.codex-work/gradient-field/component/mo-skia.wasm','typescriptRaster':str(root/'ts-raster/index.js'),
    'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
assert artifacts['rustWasm']['byteLength']==6214446 and artifacts['cppWasm']['byteLength']==2325263
assert artifacts['typescriptRaster']['sha256']==old['artifacts']['typescriptRaster']['sha256']
report=dict(format='musteroffice.paint-admission-verification/1',previousEvidence=entry(previous),
    scope='Shared evaluated gradient stops and early logical paint admission. Public wire contracts and rendering remain unchanged. Isolated storage measurement is not whole-page or product performance acceptance. Complete PPT and Musterwork replacement remain open.',
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),rustTestNames=tests,newTestNames=new_tests,
    checks=dict(rustTests=590,netNewRustTests=6,strictClippy=True,rustfmt=True,schemas=80,runtimeSchemaChecks=schema_checks,wireSchemasAndTypesByteIdentical=True,
                sourcePairedCalls=111,previousSourceUnchanged=107,previousImageClipCompositeGradientPairs=854,previousPageTextPairs=307,originalPptxFiles=4,officialXsdParts=28,
                componentArtifactsReusedUnchanged=True,newComponentSanitizerRun=False,thirdPartyLocksUnchanged=True,markdown=markdown,localLinks=links),
    commandChecks=commands,validationLogs=logs,reports={name:entry(root/(name+'.json')) for name in reports},artifacts=artifacts,
    componentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    isolatedStorageMeasurement=dict(report=entry(root/'storage.json'),platform=m['platform'],build=m['build'],scope=m['scope'],measurements=measurements,stopPayloadReductionPercent=98.4375),
    artifactSizeChange=dict(rustWasmBytes=6214446,previousRustWasmBytes=6206176,cppWasmBytes=2325263,previousCppWasmBytes=2325263,note='Uncompressed artifacts; no desktop package size measurement.'),
    limitations=['Sharing applies to a resolved paint and its clones, not independent parsed arrays or serialized JSON arrays. It is not a process-wide memory limit.',
                 'Source geometry paints are admitted before derived scene storage, image decode and shaping; shaped text-derived draws are admitted by the common scene builder after shaping.',
                 'Isolated copied-array control models former stop cloning, not a historical engine binary. Whole-process RSS and construction time do not establish product or complete page speedup.',
                 'Rendering is unchanged. Earlier LibreOffice observations and unresolved Office/WPS compatibility differences remain open; no new target application acceptance is claimed.',
                 'Complete advanced content, production Agent packaging and Musterwork replacement remain incomplete.'],verifiedArtifactRecords=records)
encoded=json.dumps(report,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert output.read_text()==encoded,'sealed evidence changed'
else:
    assert not output.exists(),'refuse to replace sealed evidence';output.write_text(encoded)
print(json.dumps(dict(evidence=entry(output),sources=len(sources),rustTests=len(tests),runtimeSchemaChecks=schema_checks,verifiedArtifacts=records)))
