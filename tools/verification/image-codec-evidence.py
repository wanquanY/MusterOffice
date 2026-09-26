"""Seal only completed decoder, sanitizer and shared-runtime verification."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/image-codec')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-scene-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-image-codec-verification.json')


def entry(path):
    p = Path(path)
    data = p.read_bytes()
    return dict(path=str(p), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


assert entry(PREVIOUS)['sha256'] == '6857e7eeaf34ed7f64a37ea28aa3e11b0ee36899e290d6922267e9da8bb1e2ce'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'Cargo.lock','Cargo.toml','README.md','contracts/README.md',
    'crates/mo-kernel-api/Cargo.toml','crates/mo-kernel-api/src/lib.rs',
    'crates/mo-skia-sys/Cargo.toml','crates/mo-skia-sys/build.rs',
    'crates/mo-skia-sys/src/ffi.rs','crates/mo-skia-sys/src/lib.rs',
    'crates/mo-wasm/Cargo.toml','crates/mo-wasm/src/lib.rs',
    'docs/README.md','docs/implementation/development.md','docs/implementation/progress.md',
    'packages/raster-component/src/index.ts','tools/components/build-skia.py',
    'tools/mo-contract-codegen/src/main.rs','tools/mo-raster-worker/Cargo.toml',
    'tools/mo-raster-worker/src/main.rs','tools/verification/contracts.py',
    'tools/verification/image-scene-parity.mjs','tools/verification/source-images-regressions.mjs',
}
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
assert changed == allowed, (changed-allowed, allowed-changed)
added = {
    'crates/mo-kernel-api/src/image_decode.rs','crates/mo-wasm/src/image_decode.rs',
    'contracts/generated/image-decode-request.schema.json','contracts/generated/image-decode-response.schema.json',
    'packages/contracts/src/generated/image-decode-request.ts','packages/contracts/src/generated/image-decode-response.ts',
    'tools/components/build-image-codecs.py','tools/components/image_codec_build.py',
    'docs/implementation/image-codec.md',
}
for pattern in ['components/image-codec/**/*','crates/mo-image/**/*','tools/verification/image-codec-*']:
    added |= {str(p) for p in Path('.').glob(pattern) if p.is_file()}
sources = {r['path'] for r in old['sourceFiles']} | added
for name in sources:
    p = Path(name)
    if p.suffix in ['.rs','.cpp','.h','.ts','.py','.mjs']:
        assert len(p.read_text().splitlines()) <= 2000, name
logs = ['tests','clippy','fmt','native-rust-build','wasm-rust-build','other-workers-build',
        'schema-check','types-check','contracts','fixtures','parity','sanitizers',
        'regressions','recent-regressions','scene-regressions','source-resources']
for name in logs:
    value = (ROOT/(name+'.log')).read_text()
    assert not any(x in value for x in ['error:','FAILED','Traceback','AssertionError','\nDiff in ']), name
for name in ['clippy','native-rust-build','wasm-rust-build','other-workers-build']:
    assert 'Finished' in (ROOT/(name+'.log')).read_text(), name
test_log = (ROOT/'tests.log').read_text()
assert 'Doc-tests mo_xml' in test_log and test_log.rstrip().endswith('finished in 0.00s')
tests = re.findall(r'^test (.+) \.\.\. ok$', test_log, re.M)
new_tests = [n for n in tests if n not in old['rustTestNames']]
assert (len(tests), len(new_tests)) == (527,6), (len(tests),len(new_tests))
assert len(re.findall(r'^check contracts/generated/', (ROOT/'schema-check.log').read_text(), re.M)) == 78
assert len(re.findall(r'^check .+\.ts$', (ROOT/'types-check.log').read_text(), re.M)) == 78
contracts = json.loads((ROOT/'contracts.log').read_text())
assert (contracts['schemas'], contracts['negativeMutations']) == (78,9)
assert json.loads((ROOT/'fixtures.log').read_text()) == dict(cases=150,success=41)
markers = {
    'parity':dict(pairedCalls=195,decodeCalls=154,sceneCalls=41,badHostReplies=4),
    'sanitizers':dict(cases=150,addressSanitizer=True,undefinedBehaviorSanitizer=True,noDiagnostics=True),
    'regressions':dict(requests=307,pixelOutputs=21),
    'recent-regressions':dict(requests=211,priorStableResponses=204,textPages=124,textGeometry=41,imageRequests=39,jsonRejections=7),
    'scene-regressions':dict(sceneRequests=140,success=124,preflightFailures=16,jsonRejections=7,pairedCalls=148,worldReferenceCalls=1,wasmHostRejections=1),
    'source-resources':dict(catalogs=19,invalidRequests=6,pairedCalls=50,noOverwriteChecks=19),
}
reports = {}
for name, marker in markers.items():
    assert json.loads((ROOT/(name+'.log')).read_text()) == marker, name
    reports[name] = json.loads((ROOT/(name+'.json')).read_text())
records = 0


def audit(value):
    global records
    if isinstance(value,dict):
        if {'path','byteLength','sha256'} <= value.keys():
            assert entry(value['path']) == value, value['path']
            records += 1
        for v in value.values(): audit(v)
    elif isinstance(value,list):
        for v in value: audit(v)


audit(reports)
audit(old['unchangedComponents'])
audit(old['previousReleaseArtifactsVerifiedUnchanged'])
audit(old['standardInputs'])
builds = {}
for name in ['native','wasm','native-asan']:
    path = ROOT/'component'/(name+'-build.json')
    data = json.loads(path.read_text())
    assert data['imageCodecs']['lock'] == json.loads(Path('components/image-codec/lock.json').read_text())
    for field in ['componentSources','artifacts']:
        audit(data[field])
    audit(data['imageCodecs'])
    builds[name] = entry(path)
schemas = {n:Draft202012Validator(json.loads(Path('contracts/generated/'+n+'.schema.json').read_text()))
           for n in ['image-decode-request','image-decode-response','image-scene-request','image-scene-response']}
for case in reports['parity']['cases']:
    if 'name' in case:
        schemas['image-decode-request'].validate({'sourceSha256':case['input']['sha256']})
        schemas['image-decode-response'].validate(json.loads(Path(case['response']['path']).read_text()))
    else:
        schemas['image-decode-response'].validate(case['response'])
for case in reports['parity']['rendered']:
    schemas['image-scene-request'].validate(json.loads(Path(case['request']['path']).read_text()))
    schemas['image-scene-response'].validate(json.loads(Path(case['response']['path']).read_text()))
markdown = links = 0
for name in sources:
    p = Path(name)
    if p.suffix != '.md': continue
    markdown += 1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)',p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:',target) or target.startswith('#'): continue
        destination = unquote(target.split('#')[0].split('?')[0])
        if not destination: continue
        resolved = p.parent/destination
        assert resolved.exists() or resolved.resolve()==OUTPUT.resolve(), (name,target)
        links += 1
subprocess.run(['git','diff','--check'],check=True)
report = dict(format='musteroffice.image-codec-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Bounded static PNG/JPEG decode, explicit orientation/color normalization and shared scene execution. Not native PPTX image fill or product acceptance.',
    checks=dict(rustTests=527,newRustTests=6,strictClippy=True,rustfmt=True,schemasChecked=78,
        generatedTypeScriptAndTypeCheck=True,genericNegativeMutations=9,decoderFixtures=150,
        successfulImages=41,independentlyComparedPixels=2427,colorChannelTolerance=1,
        newPairedRuntimeCalls=195,decodeCalls=154,decodedSceneCalls=41,wasmBadHostReplies=4,
        asanUbsanCases=150,noSanitizerDiagnostics=True,leakSanitizer=False,
        priorPairedRuntimeCalls=716,currentPairedRuntimeCalls=911,priorSavedResponsesAndPixelsUnchanged=True,
        sourceMarkdown=markdown,localLinks=links),
    sourceFiles=[entry(p) for p in sorted(sources)],changedPreviousSources=sorted(changed),addedSources=sorted(added),
    rustTestNames=tests,newTestNames=new_tests,validationLogs=[entry(ROOT/(n+'.log')) for n in logs],
    reports={n:dict(record=entry(ROOT/(n+'.json')),counts=v) for n,v in markers.items()},
    componentBuilds=builds,codecInputLock=entry('components/image-codec/lock.json'),
    artifacts=[entry(p) for p in ['target/debug/mo-cli','target/debug/mo-raster-worker','target/debug/mo-text-worker',
        'target/wasm32-unknown-unknown/debug/mo_wasm.wasm',ROOT/'wasm-node/mo_wasm.js',ROOT/'wasm-node/mo_wasm_bg.wasm',
        ROOT/'wasm-node/mo_wasm.d.ts',ROOT/'wasm-node/mo_wasm_bg.wasm.d.ts',ROOT/'wasm-node/package.json',
        ROOT/'ts-raster/index.js',ROOT/'component/mo-skia.wasm',ROOT/'component/mo-skia.mjs']],
    componentBytes=dict(wasm=2313369,previousRasterWasm=1810991,delta=502378,
        scope='Raw raster+decode WASM component only; excludes Rust core, host, fonts, media and installer.'),
    unchangedComponents=old['unchangedComponents'],previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],limitations=[
        'Static PNG and baseline/progressive 8-bit JPEG profile only; other formats, APNG/MPO/HDR and unlisted JPEG processes are not complete.',
        'Synthetic small-image numeric references; no Office/WPS application or photographic/CMYK/complex-ICC acceptance.',
        'PPTX resource extraction and decoded scene rendering are separate; crop/tile/stretch/rotation/effects are not yet connected to source-page compilation.',
        'RGBA8 SDR is the explicit output profile; original encoded bytes must remain available for document fidelity and later color pipelines.',
        'Synchronous components require isolated host termination for cancellation during decode; production browser Worker integration and target browser EH checks remain.',
        'No product latency, peak RSS, full kernel, installer-size or E0-E3 replacement acceptance claim.',
        'Complete editing/export/player/advanced content and production Agent/Artifact/Musterwork integration remain incomplete.',
    ])
assert entry(ROOT/'component/mo-skia.wasm')['byteLength'] == report['componentBytes']['wasm']
raw = json.dumps(report,ensure_ascii=False,indent=2)+'\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f: f.write(raw)
elif OUTPUT.exists(): assert OUTPUT.read_text()==raw, 'sealed inputs changed'
print(json.dumps(dict(sources=len(sources),changed=len(changed),added=len(added),boundRecords=records,
                      checks=report['checks'],evidence=entry(OUTPUT) if OUTPUT.exists() else None)))
