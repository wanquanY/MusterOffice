"""Seal the actual group-image implementation, regressions and open differences."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
root=Path('.codex-work/group-image')
previous=Path('docs/reviews/evidence/2026-09-25-source-resource-page-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-group-image-verification.json')
def entry(p):
 b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='4f98b778903e7658eb99ca289e5bdbccd18060ee41714ff28ce4652b7056bdc5'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/source-resource-page.md
crates/mo-presentation-compile/src/source_resource_page/resources.rs'''.splitlines())
assert changed==allowed,changed^allowed
added=set('''crates/mo-presentation-compile/tests/source_group_images.rs
crates/mo-harfbuzz-sys/tests/source_group_images.rs
tools/test-support/source_group_images.rs
docs/implementation/group-image-inheritance.md'''.splitlines())
for name in ['checks.py','fixtures.py','parity.mjs','observe.py','reference.py','evidence.py']:
 added.add('tools/verification/group-image-'+name)
assert not added&set(prior)
sources=set(prior)|added
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:assert len(Path(p).read_text().splitlines())<=2000,p
commands=json.loads((root/'checks.json').read_text());assert len(commands)==8 and all(c['exitCode']==0 for c in commands)
logs={c['name']:entry(c['log']) for c in commands}
for c in commands:
 v=Path(c['log']).read_text();assert not any(t in v for t in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),c['name']
 if c['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in v,c['name']
for name in ['fixtures','parity','observations','reference']:
 path=root/(name+'.log');v=path.read_text();assert v.strip() and not any(t in v for t in ['Traceback','AssertionError','FAILED']),name
 logs[name]=entry(path)
logs['application-convert']=entry(root/'libreoffice-convert.log')
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
new_tests=[n for n in tests if n not in old['rustTestNames']]
assert len(tests)==573 and len(new_tests)==3 and set(old['rustTestNames'])<=set(tests)
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
reports={n:json.loads((root/(n+'.json')).read_text()) for n in ['fixtures','parity','observations','reference']}
f,p,o,r=[reports[n] for n in ['fixtures','parity','observations','reference']]
assert len(f['cases'])==23 and f['officialXsdParts']==161
assert p['pairedCalls']==len(p['cases'])==56 and sum(c['status']=='rendered' for c in p['cases'])==38
assert len(p['equivalent'])==10 and p['previousUnchanged']==32 and p['previousNewlySupported']==['group-fill']
assert len(o['cases'])==20 and len(o['pairs'])==10
assert {c['name']:c['pixelsDifferent'] for c in o['pairs'] if c['pixelsDifferent']}=={'root':60100,'layout-placeholder':11325}
assert len(r['cases'])==5 and r['interiorPixels']==544960 and r['applicationPixelsDifferent']==77422
assert {c['name']:c['applicationPixelsDifferent'] for c in r['cases'] if c['applicationPixelsDifferent']}=={'root':55296,'layout-placeholder':9798,'different-receivers':12328}
records=0
def audit(v):
 global records
 if isinstance(v,dict):
  if {'path','byteLength','sha256'}<=v.keys():
   assert all(entry(v['path'])[k]==v[k] for k in ['path','byteLength','sha256']),v['path'];records+=1
  for x in v.values():audit(x)
 elif isinstance(v,list):
  for x in v:audit(x)
audit(reports)
prior_parity=json.loads(Path(p['previous']['path']).read_text())
for c in prior_parity['cases']:
 current=next(v for v in p['cases'] if v['name']=='previous/'+c['name'])
 for key in ['request','source','fonts']:assert current[key]['sha256']==c[key]['sha256'],c['name']
 if c['name']=='group-fill':assert current['status']=='rendered' and c['status']=='error'
 else:
  assert current['response']['sha256']==c['response']['sha256'],c['name']
  if 'pixels' in c:assert current['pixels']['sha256']==c['pixels']['sha256'],c['name']
assert p['cli']['overwriteExit']==1 and p['cli']['failureOutputAbsent'] is True
assert json.loads(Path(p['cli']['createResponse']['path']).read_text())['status']=='rendered'
failure=json.loads(Path(p['cli']['failureResponse']['path']).read_text());assert failure['status']=='error' and failure['error']['image']['kind']=='stationaryOrientation'
for rec in old['componentBuilds'].values():
 audit(rec);build=json.loads(Path(rec['path']).read_text())
 for key in ['componentSources','artifacts','imageCodecs']:
  if key in build:audit(build[key])
for key in ['previousReleaseArtifactsVerifiedUnchanged','standardInputs','codecInputLock']:audit(old[key])
# Earlier output directories remain immutable. Debug worker/CLI intentionally
# rebuild; every previously recorded WASM/adapter remains independently pinned.
for key,value in old['artifacts'].items():
 if key not in ['nativeWorker','nativeCli']:audit(value)
schema_checks=0
schemas={n:Draft202012Validator(json.loads(Path('contracts/generated/pptx-resource-page-'+n+'.schema.json').read_text())) for n in ['request','raster-response']}
for c in p['cases']:
 schemas['raster-response'].validate(json.loads(Path(c['response']['path']).read_text()));schema_checks+=1
 if c['name'] not in ['previous/duplicate-json','previous/unknown-profile']:
  schemas['request'].validate(json.loads(Path(c['request']['path']).read_text()));schema_checks+=1
assert schema_checks==110
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
artifacts={n:entry(v) for n,v in {
 'nativeWorker':'target/debug/mo-raster-worker','nativeCli':'target/debug/mo-cli',
 'rustWasm':str(root/'wasm-node/mo_wasm_bg.wasm'),'rustWasmGlue':str(root/'wasm-node/mo_wasm.js'),
 'cppWasm':'.codex-work/clips/component/mo-skia.wasm','typescriptRaster':'.codex-work/clips/ts-raster/index.js',
 'hbWasm':'.codex-work/harfbuzz/release/mo-hb.wasm','typescriptText':'.codex-work/text-component/index.js'}.items()}
report=dict(format='musteroffice.group-image-verification/1',previousEvidence=entry(previous),
 scope='Group image property inheritance in the actual source-page engine. Native/WASM and explicit-source controls pass; application root-group and flip differences remain open. Not complete PPT or Musterwork replacement acceptance.',
 sourceFiles=[entry(v) for v in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 rustTestNames=tests,newTestNames=new_tests,
 checks=dict(rustTests=573,newRustTests=3,strictClippy=True,rustfmt=True,schemas=80,contractsUnchanged=True,runtimeSchemaChecks=schema_checks,
             newSourceCases=23,officialXsdParts=161,pairedPublicCalls=56,successfulPairs=38,explicitSourcePairs=10,previousRequestsUnchanged=32,previousRequestsNewlySupported=1,
             independentInteriorPixels=544960,applicationSourceCases=20,applicationSourcePairsEqual=8,applicationSourcePairsDifferent=2,applicationInteriorPixelsDifferent=77422,
             cliNewOutputAndFailurePublication=True,dependenciesUnchanged=True,fixedComponentsUnchanged=True,markdown=markdown,localLinks=links),
 commandChecks=commands,validationLogs=logs,reports={n:entry(root/(n+'.json')) for n in reports},artifacts=artifacts,
 componentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],
 previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
 artifactSizeChange=dict(rustWasmBytes=artifacts['rustWasm']['byteLength'],previousRustWasmBytes=old['artifacts']['rustWasm']['byteLength'],note='One uncompressed Rust WASM artifact; not desktop installer or performance measurement.'),
 limitations=['Native group fill uses receiving-shape image layout and existing draft placement. Root group and horizontal image-fill flip differences require target Office/WPS evidence.',
              'Independent reference excludes AA/shape/clip/texel boundaries. LibreOffice PDF first-page observations are not full visual or editable-save-reopen acceptance.',
              'Background replay, stationary image orientation, complete text/native paints/effects and advanced presentation features remain unfinished.',
              'No new C++/ABI, sanitizer, complete performance/RSS or package-size measurement.',
              'MCP/Skill/Plugin packaging, production host/Artifact/Musterwork integration and replacement acceptance remain unfinished.'])
if '--seal' in sys.argv:
 assert not output.exists(),'evidence is immutable after sealing';output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
else:assert json.loads(output.read_text())==report,'evidence differs from current closure'
print(json.dumps(dict(evidence=entry(output),sources=len(sources),auditedRecords=records)))
