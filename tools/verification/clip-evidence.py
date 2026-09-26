"""Seal shared clipping implementation, current runtimes and bounded verification."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/clips')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-layout-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-shared-clips-verification.json')


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == '6eb86d903ec9feffd5ef961e4e06581e8149738e5bb824ea908d34a0ea41f361'
old = json.loads(PREVIOUS.read_text())
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
stems = ['image-raster-request', 'image-raster-response', 'image-scene-request', 'image-scene-response',
         'page-compile-response', 'page-raster-response', 'path-raster-request', 'path-raster-response',
         'pptx-page-compile-response', 'pptx-page-raster-response', 'pptx-text-page-raster-response',
         'scene-raster-request', 'scene-raster-response']
allowed = {
    'components/skia/mo_skia.cpp', 'components/skia/mo_skia.h', 'crates/mo-kernel-api/src/raster.rs',
    'crates/mo-presentation-compile/src/page.rs', 'crates/mo-presentation-compile/src/path_scene.rs',
    'crates/mo-presentation-compile/src/source_page.rs', 'crates/mo-skia-sys/src/ffi.rs',
    'packages/raster-component/src/index.ts', 'tools/components/build-skia.py',
    'tools/mo-cli/src/raster.rs', 'tools/verification/skia-probe.cpp',
    'README.md', 'docs/README.md', 'docs/implementation/progress.md', 'docs/implementation/development.md',
}
for name in ['compile', 'gradient_tests', 'image_tests', 'lib', 'stroke_tests', 'tests', 'types']:
    allowed.add('crates/mo-raster/src/'+name+'.rs')
for name in ['compile', 'lib', 'tests', 'types']:
    allowed.add('crates/mo-render/src/'+name+'.rs')
for stem in stems:
    allowed.add('contracts/generated/'+stem+'.schema.json')
    allowed.add('packages/contracts/src/generated/'+stem+'.ts')
assert changed == allowed, (changed-allowed, allowed-changed)
added = {'components/skia/mo_clip.h', 'crates/mo-raster/src/clip.rs', 'crates/mo-raster/src/clip_tests.rs',
         'crates/mo-render/src/clip_tests.rs', 'docs/implementation/shared-clips.md'}
for name in ['fixtures', 'parity', 'components', 'image-regressions']:
    added.add('tools/verification/clip-'+name+'.mjs')
added.add('tools/verification/clip-evidence.py')
sources = {r['path'] for r in old['sourceFiles']} | added
for name in sources:
    p = Path(name)
    if p.suffix in ['.rs', '.cpp', '.h', '.ts', '.py', '.mjs']:
        assert len(p.read_text().splitlines()) <= 2000, name
for name in ['Cargo.lock', 'pnpm-lock.yaml', 'components/skia/lock.json']:
    assert entry(name) == next(r for r in old['sourceFiles'] if r['path'] == name)
logs = ['build-native', 'build-wasm', 'build-asan', 'tests', 'build-workspace', 'clippy', 'fmt',
        'rust-wasm', 'schema-check', 'types-check', 'contracts', 'parity', 'components',
        'image-regressions', 'regressions', 'recent-regressions']
for name in logs:
    value = (ROOT/(name+'.log')).read_text()
    assert not any(v in value for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
    if name in ['build-workspace', 'clippy', 'rust-wasm']:
        assert 'Finished' in value, name
value = (ROOT/'tests.log').read_text()
assert 'Doc-tests mo_xml' in value
rust_tests = re.findall(r'^test (.+) \.\.\. ok$', value, re.M)
new_tests = [n for n in rust_tests if n not in old['rustTestNames']]
assert (len(rust_tests), len(new_tests)) == (552, 7)
assert len(re.findall(r'^check contracts/generated/', (ROOT/'schema-check.log').read_text(), re.M)) == 78
assert len(re.findall(r'^check .+\.ts$', (ROOT/'types-check.log').read_text(), re.M)) == 78
contract = json.loads((ROOT/'contracts.log').read_text())
assert (contract['schemas'], contract['negativeMutations']) == (78, 9)
reports = {n: json.loads((ROOT/(n+'.json')).read_text()) for n in ['parity', 'components', 'image-regressions', 'regressions', 'recent-regressions']}
p = reports['parity']
assert (len(p['cases']), p['pairedCalls'], p['referencePixels'], p['maxChannelDeviation'], len(p['negatives'])) == (50, 242, 36864, 0, 6)
assert len(p['cliChecks']) == 2 and all(c['exclusivePublication'] for c in p['cliChecks'])
c = reports['components']
assert c['triples'] == 125 and len(c['negatives']) == 22 and c['noDiagnostics']
assert c['addressSanitizer'] and c['undefinedBehaviorSanitizer'] and not c['leakSanitizer']
assert reports['image-regressions']['pairedCalls'] == 626
assert json.loads((ROOT/'regressions.log').read_text()) == dict(requests=307, pixelOutputs=21)
assert json.loads((ROOT/'recent-regressions.log').read_text()) == dict(requests=211, priorStableResponses=204, textPages=124, textGeometry=41, imageRequests=39, jsonRejections=7)
records = 0


def audit(value):
    global records
    if isinstance(value, dict):
        if {'path', 'byteLength', 'sha256'} <= value.keys():
            actual = entry(value['path'])
            assert all(actual[k] == value[k] for k in actual), value['path']
            records += 1
        for v in value.values():
            audit(v)
    elif isinstance(value, list):
        for v in value:
            audit(v)


audit(reports)
builds = {}
for name in ['native', 'wasm', 'native-asan']:
    path = ROOT/'component'/(name+'-build.json')
    build = json.loads(path.read_text())
    for key in ['componentSources', 'artifacts', 'imageCodecs']:
        audit(build[key])
    builds[name] = entry(path)
for key in ['previousReleaseArtifactsVerifiedUnchanged', 'standardInputs', 'codecInputLock']:
    audit(old[key])
schema_checks = 0
for c in p['cases']:
    for scene in [False, True]:
        stem = ('image-scene' if scene else 'image-raster') if c['images'] else ('scene-raster' if scene else 'path-raster')
        for suffix, key in [('request', 'sceneRequest' if scene else 'request'), ('response', 'sceneResponse' if scene else 'pathResponse')]:
            Draft202012Validator(json.loads(Path('contracts/generated/'+stem+'-'+suffix+'.schema.json').read_text())).validate(json.loads(Path(c[key]['path']).read_text()))
            schema_checks += 1
markdown = links = 0
for name in sources:
    pth = Path(name)
    if pth.suffix != '.md':
        continue
    markdown += 1
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', pth.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'):
            continue
        destination = unquote(target.split('#')[0].split('?')[0])
        if destination:
            resolved = pth.parent/destination
            assert resolved.exists() or resolved.resolve() == OUTPUT.resolve(), (name, target)
            links += 1
subprocess.run(['git', 'diff', '--check'], check=True)
wasm = entry(ROOT/'component/mo-skia.wasm')
old_wasm = entry('.codex-work/image-domain/component/mo-skia.wasm')
assert old_wasm['byteLength'] == 2320035
report = dict(
    format='musteroffice.shared-clips-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Reusable filled-path intersections in shared DrawIR/Rust/Native/WASM/CLI. Not complete source image pages or Office/WPS/Musterwork acceptance.',
    sourceFiles=[entry(n) for n in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=rust_tests, newTestNames=new_tests,
    checks=dict(rustTests=552, newRustTests=7, strictClippy=True, rustfmt=True, schemas=78, generatedTypeScript=True,
                runtimeSchemaChecks=schema_checks, genericNegativeMutations=9, newCases=50, newPairedCalls=242,
                independentMaskPixels=36864, maxChannelDeviation=0, newPreflightFailures=6,
                componentTriples=125, rawNegativeFrames=22, sanitizerDiagnostics=False,
                oldRegressionPairedCalls=1144, cliExclusivePublicationModes=2,
                thirdPartyDependencyResolutionUnchanged=True, markdown=markdown, localLinks=links),
    componentBuilds=builds, reports={n: entry(ROOT/(n+'.json')) for n in reports},
    validationLogs={n: entry(ROOT/(n+'.log')) for n in logs},
    artifacts={n: entry(path) for n, path in {
        'nativeWorker': 'target/debug/mo-raster-worker', 'nativeCli': 'target/debug/mo-cli',
        'nativeTextWorker': 'target/debug/mo-text-worker', 'rustWasm': str(ROOT/'wasm-node/mo_wasm_bg.wasm'),
        'rustWasmGlue': str(ROOT/'wasm-node/mo_wasm.js'), 'typescriptAdapter': str(ROOT/'ts-raster/index.js'),
        'cppWasm': str(ROOT/'component/mo-skia.wasm'), 'cppWasmGlue': str(ROOT/'component/mo-skia.mjs'),
    }.items()},
    isolatedComponentBytes=dict(previous=old_wasm, current=wasm, delta=wasm['byteLength']-old_wasm['byteLength'],
                               scope='uncompressed C++ WASM component only; not installer, full runtime or speed/RSS benchmark'),
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'], codecInputLock=old['codecInputLock'],
    limitations=['Clip input coordinates are bounded, but no analytic coverage guarantee for arbitrary AA curves.',
                 'Source image fill rectangle and rotation basis still require integration into ordered page rendering.',
                 'No new installer, whole-document latency/RSS benchmark or application interoperability acceptance.',
                 'Full advanced PPT, Agent packages, production host and Musterwork replacement gates remain open.'],
)
if '--seal' in sys.argv:
    assert not OUTPUT.exists(), 'evidence is immutable after sealing'
    OUTPUT.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
else:
    assert json.loads(OUTPUT.read_text()) == report, 'completed evidence differs from current closure'
print(json.dumps(dict(evidence=entry(OUTPUT), sources=len(sources), auditedRecords=records)))
