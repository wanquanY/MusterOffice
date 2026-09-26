"""Seal one reviewed source/resource page implementation and its exact checks."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from urllib.parse import unquote
from jsonschema import Draft202012Validator
root=Path('.codex-work/resource-page')
previous=Path('docs/reviews/evidence/2026-09-25-image-world-verification.json')
output=Path('docs/reviews/evidence/2026-09-25-source-resource-page-verification.json')
def entry(path):
 b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
assert entry(previous)['sha256']=='a4e166e7426b36f2ffc495221bf566073c329ca895056430fac0ad2a256ced71'
old=json.loads(previous.read_text());prior={r['path']:r for r in old['sourceFiles']}
changed={p for p,r in prior.items() if entry(p)!=r}
allowed=set('''Cargo.lock
crates/mo-harfbuzz-sys/Cargo.toml
crates/mo-kernel-api/src/lib.rs
crates/mo-kernel-api/src/pptx_page.rs
crates/mo-kernel-api/src/pptx_text_page.rs
crates/mo-kernel-api/src/pptx_text_page/diagnostic.rs
crates/mo-presentation-compile/Cargo.toml
crates/mo-presentation-compile/src/lib.rs
crates/mo-presentation-compile/src/path_scene.rs
crates/mo-presentation-compile/src/source_page.rs
crates/mo-presentation-compile/src/source_page/layers.rs
crates/mo-presentation-compile/src/source_page/paint.rs
crates/mo-presentation-compile/src/source_page/types.rs
crates/mo-presentation-compile/src/source_text_page.rs
crates/mo-presentation-compile/src/source_text_page/types.rs
crates/mo-wasm/src/image_decode.rs
crates/mo-wasm/src/raster.rs
packages/contracts/tests/wire.ts
tools/mo-cli/src/main.rs
tools/mo-cli/src/raster.rs
tools/mo-contract-codegen/src/main.rs
tools/mo-raster-worker/src/main.rs
tools/verification/contracts.py
README.md
docs/README.md
docs/implementation/progress.md
docs/implementation/development.md
contracts/README.md'''.splitlines())
# Generated old contracts expand the common page binding/diagnostic types.
for p in prior:
 if p.startswith(('contracts/generated/pptx-page-','contracts/generated/pptx-text-page-raster-response','packages/contracts/src/generated/pptx-page-','packages/contracts/src/generated/pptx-text-page-raster-response')):allowed.add(p)
assert changed<=allowed,changed-allowed
added=set('''crates/mo-harfbuzz-sys/tests/source_resource_page.rs
crates/mo-kernel-api/src/pptx_resource_page.rs
crates/mo-kernel-api/src/pptx_resource_page/diagnostic.rs
crates/mo-kernel-api/tests/pptx_resource_page.rs
crates/mo-presentation-compile/src/source_page/objects.rs
crates/mo-presentation-compile/src/source_page/prepared.rs
crates/mo-presentation-compile/src/source_page/emit.rs
crates/mo-presentation-compile/src/source_resource_page.rs
crates/mo-presentation-compile/src/source_resource_page/resources.rs
crates/mo-presentation-compile/src/source_resource_page/types.rs
crates/mo-presentation-compile/tests/source_resource_page.rs
docs/implementation/source-resource-page.md
tools/test-support/source_resource_page.rs
fixtures/presentations/resource-page/transparent.png
fixtures/presentations/resource-page/cyan.png
fixtures/presentations/resource-page/images.json'''.splitlines())
for name in ['images.py','checks.py','fixtures.py','parity.mjs','reference.py','evidence.py']:added.add('tools/verification/resource-page-'+name)
for p in Path('contracts/generated').glob('pptx-resource-page-*.schema.json'):added.add(str(p))
for p in Path('packages/contracts/src/generated').rglob('*.ts'):
 if p.name.startswith('pptx-resource-page-') or p.parent.name.startswith(('pptx-resource-page-','pptx-page-','pptx-text-page-raster-response')):
  if str(p) not in prior:added.add(str(p))
assert not added&set(prior)
sources=set(prior)|added
for p in sources:
 if Path(p).suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:assert len(Path(p).read_text().splitlines())<=2000,p
# Exact lockfile delta: only an existing workspace library becomes an explicit
# dev dependency of native text tests. No registry package/version changed.
lock=Path('Cargo.lock').read_text();a=lock.index('name = "mo-harfbuzz-sys"');b=lock.index('\n[[package]]',a)
restored=lock[:a]+lock[a:b].replace(' "mo-image",\n','')+lock[b:]
assert hashlib.sha256(restored.encode()).hexdigest()==prior['Cargo.lock']['sha256']
commands=json.loads((root/'checks.json').read_text());assert len(commands)==10 and all(c['exitCode']==0 for c in commands)
validation_logs={c['name']:entry(c['log']) for c in commands}
for name in ['fixtures','parity','reference','contracts']:
 path=root/(name+'.log');value=path.read_text()
 assert value.strip() and not any(t in value for t in ['error:','FAILED','Traceback','AssertionError']),name
 validation_logs[name]=entry(path)
for c in commands:
 value=Path(c['log']).read_text()
 assert not any(t in value for t in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),c['name']
 if c['name'] in ['tests','clippy','native-build','rust-wasm']:assert 'Finished' in value,c['name']
tests=re.findall(r'^test (.+) \.\.\. ok$',(root/'tests.log').read_text(),re.M)
assert len(tests)==570,len(tests)
new_tests=[t for t in tests if t not in old['rustTestNames']]
assert len(new_tests)==10 and set(old['rustTestNames'])<=set(tests),new_tests
assert 'Doc-tests mo_xml' in (root/'tests.log').read_text()
assert len(re.findall(r'^check contracts/generated/',(root/'schema-check.log').read_text(),re.M))==80
reports={name:json.loads((root/(name+'.json')).read_text()) for name in ['fixtures','parity','reference']}
f,p,r=reports['fixtures'],reports['parity'],reports['reference']
assert len(f['cases'])==24 and f['officialXsdParts']==168
assert p['pairedCalls']==33 and sum(c['status']=='rendered' for c in p['cases'])==17
assert p['cli']['overwriteExit']==1 and p['cli']['failureOutputAbsent'] is True
assert json.loads(Path(p['cli']['createResponse']['path']).read_text())['status']=='rendered'
failure=json.loads(Path(p['cli']['failureResponse']['path']).read_text())
assert failure['status']=='error' and failure['error']['image']['kind']=='stationaryOrientation'
assert len(r['cases'])==5 and r['interiorPixels']>500000
records=0
def audit(v):
 global records
 if isinstance(v,dict):
  if {'path','byteLength','sha256'}<=v.keys():assert all(entry(v['path'])[k]==v[k] for k in ['path','byteLength','sha256']),v['path'];records+=1
  for x in v.values():audit(x)
 elif isinstance(v,list):
  for x in v:audit(x)
audit(reports)
# Preserve fixed component closures, including earlier sanitizer artifacts;
# this stage does not claim a new sanitizer execution.
for rec in old['componentBuilds'].values():
 audit(rec);build=json.loads(Path(rec['path']).read_text())
 for key in ['componentSources','artifacts','imageCodecs']:
  if key in build:audit(build[key])
for key in ['previousReleaseArtifactsVerifiedUnchanged','standardInputs','codecInputLock']:audit(old[key])
for key in ['cppWasm','cppWasmGlue','typescriptAdapter']:audit(old['artifacts'][key])
response_schema=Draft202012Validator(json.loads(Path('contracts/generated/pptx-resource-page-raster-response.schema.json').read_text()))
request_schema=Draft202012Validator(json.loads(Path('contracts/generated/pptx-resource-page-request.schema.json').read_text()))
schema_checks=0
for c in p['cases']:
 response_schema.validate(json.loads(Path(c['response']['path']).read_text()));schema_checks+=1
 if c['name'] not in ['duplicate-json','unknown-profile']:
  request_schema.validate(json.loads(Path(c['request']['path']).read_text()));schema_checks+=1
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
report=dict(format='musteroffice.source-resource-page-verification/1',previousEvidence=entry(previous),
 scope='One production source-page engine for image resources, shapes and optional text, executed through Native/WASM public JSON and CLI boundaries. Draft static subset, not complete PPT or Musterwork replacement acceptance.',
 sourceFiles=[entry(v) for v in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
 rustTestNames=tests,newTestNames=new_tests,
 checks=dict(rustTests=570,newRustTests=10,strictClippy=True,rustfmt=True,schemas=80,runtimeSchemaChecks=schema_checks,
             sourceCases=24,officialXsdParts=168,pairedPublicCalls=33,successfulPairs=17,independentInteriorPixels=r['interiorPixels'],cliNewOutputAndFailurePublication=True,
             registryDependenciesUnchanged=True,fixedComponentsUnchanged=True,markdown=markdown,localLinks=links),
 commandChecks=commands,validationLogs=validation_logs,
 reports={n:entry(root/(n+'.json')) for n in reports},artifacts=artifacts,
 componentBuilds=old['componentBuilds'],standardInputs=old['standardInputs'],codecInputLock=old['codecInputLock'],
 previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
 artifactSizeChange=dict(rustWasmBytes=artifacts['rustWasm']['byteLength'],previousRustWasmBytes=old['artifacts']['rustWasm']['byteLength'],
                        note='A specific uncompressed release WASM artifact, not a desktop installer or performance estimate.'),
 limitations=['Static PNG/JPEG, supported native geometry/solid outlines and current text profile only; advanced PPT coverage and product integration remain open.',
 'grpFill/useBgFill image paint space, translucent background replay and stationary object-image orientation remain explicit prerequisites; no substitute rendering.',
 'Pixel oracle covers five original static patterns, excluding AA/clip/texel boundaries. Synthetic fonts and Native/WASM parity are not Office/WPS fidelity or edit-save-reopen acceptance.',
 'No new C++/ABI/component selection, sanitizer run, overall performance/RSS or desktop package measurement.',
 'MCP/Skill/Plugin packaging, Musterwork host/Artifact integration and replacement acceptance remain unfinished.'])
if '--seal' in sys.argv:
 assert not output.exists(),'evidence is immutable after sealing';output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
else:assert json.loads(output.read_text())==report,'evidence differs from current closure'
print(json.dumps(dict(evidence=entry(output),sources=len(sources),auditedRecords=records)))
