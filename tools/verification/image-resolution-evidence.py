"""Bind completed image resolution tests to actual source/runtime artifacts."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/image-resolution')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-codec-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-image-resolution-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == 'ce3a23b922d4987db5020dde39df4b32bfb43816ff87de3207a716c2e07bb070'
old = json.loads(PREVIOUS.read_text())
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
allowed = {
    'crates/mo-image/src/lib.rs','crates/mo-image/src/tests.rs',
    'contracts/generated/image-decode-response.schema.json',
    'packages/contracts/src/generated/image-decode-response.ts',
    'tools/verification/image-codec-parity.mjs',
    'README.md','docs/README.md','contracts/README.md',
    'docs/implementation/image-codec.md','docs/implementation/development.md',
    'docs/implementation/progress.md',
}
assert changed == allowed, (changed-allowed,allowed-changed)
added = {
    'crates/mo-image/src/resolution.rs','crates/mo-image/src/resolution/parse.rs',
    'crates/mo-image/src/resolution/tests.rs','docs/implementation/image-resolution.md',
    'tools/verification/image-resolution-fixtures.py','tools/verification/image-resolution-evidence.py',
}
sources = {r['path'] for r in old['sourceFiles']} | added
for path in sources:
    p = Path(path)
    if p.suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(p.read_text().splitlines())<=2000, path
logs = ['tests','clippy','fmt','native-build','wasm-build','schema-check','types-check',
        'contracts','fixtures','parity','codec-regressions']
for name in logs:
    value=(ROOT/(name+'.log')).read_text()
    assert not any(v in value for v in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']),name
    if name in ['clippy','native-build','wasm-build']: assert 'Finished' in value,name
test_log=(ROOT/'tests.log').read_text()
assert 'Doc-tests mo_xml' in test_log and test_log.rstrip().endswith('finished in 0.00s')
tests=re.findall(r'^test (.+) \.\.\. ok$',test_log,re.M)
new_tests=[name for name in tests if name not in old['rustTestNames']]
assert (len(tests),len(new_tests))==(534,7)
assert len(re.findall(r'^check contracts/generated/',(ROOT/'schema-check.log').read_text(),re.M))==78
assert len(re.findall(r'^check .+\.ts$',(ROOT/'types-check.log').read_text(),re.M))==78
contracts=json.loads((ROOT/'contracts.log').read_text())
assert (contracts['schemas'],contracts['negativeMutations'])==(78,9)
assert json.loads((ROOT/'fixtures.log').read_text())==dict(cases=102,success=78)
markers = {
    'parity':dict(pairedCalls=184,decodeCalls=106,sceneCalls=78,badHostReplies=4),
    'codec-regressions':dict(pairedCalls=195,decodeCalls=154,sceneCalls=41,badHostReplies=4),
}
reports = {}
for name,marker in markers.items():
    assert json.loads((ROOT/(name+'.log')).read_text())==marker
    reports[name]=json.loads((ROOT/('parity.json' if name=='parity' else 'codec-regressions/parity.json')).read_text())
assert reports['codec-regressions']['priorFieldsAndPixelsUnchanged']
records=0


def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'}<=value.keys():
            assert entry(value['path'])==value,value['path']
            records+=1
        for v in value.values(): audit(v)
    elif isinstance(value,list):
        for v in value: audit(v)


audit(reports)
audit(old['unchangedComponents'])
audit(old['previousReleaseArtifactsVerifiedUnchanged'])
audit(old['standardInputs'])
audit(old['componentBuilds'])
# Fixed native, WASM and sanitizer component/dependency/source closures must all
# stay byte-identical. New Rust metadata work does not imply new sanitizer runs.
for build in old['componentBuilds'].values():
    value=json.loads(Path(build['path']).read_text())
    for key in ['componentSources','artifacts','imageCodecs']: audit(value[key])
schemas={name:Draft202012Validator(json.loads(Path('contracts/generated/'+name+'.schema.json').read_text()))
         for name in ['image-decode-response','image-scene-request','image-scene-response']}
for report in reports.values():
    for case in report['cases']:
        response=json.loads(Path(case['response']['path']).read_text()) if 'name' in case else case['response']
        schemas['image-decode-response'].validate(response)
    for case in report['rendered']:
        for key,schema in [('request','image-scene-request'),('response','image-scene-response')]:
            schemas[schema].validate(json.loads(Path(case[key]['path']).read_text()))
fixture_manifest=json.loads((ROOT/'cases/manifest.json').read_text())
pixel_count=sum(c['width']*c['height'] for c in fixture_manifest if 'expected' in c)
for case in fixture_manifest:
    assert entry(case['path'])['sha256']==case['sha256']
    if 'expected' in case:
        response=json.loads((ROOT/'runtime'/(case['name']+'.json')).read_text())
        assert response['info']['resolution']==case['resolution']
        assert (ROOT/'runtime'/(case['name']+'.rgba')).read_bytes()==Path(case['expected']).read_bytes()
markdown=links=0
for name in sources:
    p=Path(name)
    if p.suffix!='.md': continue
    markdown+=1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'): continue
        destination=unquote(target.split('#')[0].split('?')[0])
        if not destination: continue
        resolved=p.parent/destination
        assert resolved.exists() or resolved.resolve()==OUTPUT.resolve(),(name,target)
        links+=1
subprocess.run(['git','diff','--check'],check=True)
report=dict(
    format='musteroffice.image-resolution-verification/1',previousEvidence=entry(PREVIOUS),
    scope='PNG pHYs, JPEG JFIF and Exif IFD0 raw resolution metadata with exact, explicit consensus physical pixel size. Not PPTX image page compilation or Office/WPS acceptance.',
    checks=dict(rustTests=len(tests),newRustTests=len(new_tests),strictClippy=True,rustfmt=True,
                schemasChecked=78,generatedTypeScriptAndTypeCheck=True,genericNegativeMutations=9,
                newFixtures=102,successfulNewImages=78,independentlyComparedNewPixels=pixel_count,
                newPairedCalls=184,priorCodecPairedCalls=195,currentPairedCalls=379,wasmBadHostReplies=8,
                priorMetadataExceptResolutionAndPixelsUnchanged=True,sourceMarkdown=markdown,localLinks=links),
    sourceFiles=[entry(n) for n in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new_tests,
    validationLogs={n:entry(ROOT/(n+'.log')) for n in logs},
    reports={n:entry(ROOT/('parity.json' if n=='parity' else 'codec-regressions/parity.json')) for n in reports},
    fixtureManifest=entry(ROOT/'cases/manifest.json'),
    componentBuilds=old['componentBuilds'],codecInputLock=old['codecInputLock'],
    unchangedComponents=old['unchangedComponents']+[entry('.codex-work/image-codec/'+p) for p in
        ['component/mo-skia.wasm','component/mo-skia.mjs','ts-raster/index.js']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],
    artifacts={n:entry(p) for n,p in {
        'nativeDebugWorker':'target/debug/mo-raster-worker',
        'rustDebugWasm':str(ROOT/'wasm-node/mo_wasm_bg.wasm'),
        'rustWasmGlue':str(ROOT/'wasm-node/mo_wasm.js'),
    }.items()},
    limitations=['No new C++ code or codec dependency; prior 150-case ASan/UBSan evidence is historical, not rerun here.',
                 'No complete source PPTX image paint/crop/tile integration yet.',
                 'Consensus/default rules are explicit draft semantics, not measured Office/WPS resolution precedence.',
                 'No installer, RSS, latency, browser Worker or full-format acceptance conclusion.'],
)
if '--seal' in sys.argv:
    assert not OUTPUT.exists(),'evidence is immutable after sealing'
    OUTPUT.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
else:
    assert json.loads(OUTPUT.read_text())==report,'completed evidence does not match current closure'
print(json.dumps(dict(evidence=entry(OUTPUT),sources=len(sources),auditedRecords=records)))
