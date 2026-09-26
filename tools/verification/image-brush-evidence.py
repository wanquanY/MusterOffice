"""Bind image rendering, independent reference and preserved regressions."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/image-brush')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-character-spacing-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-image-brush-completed-verification.json')
UNFINALIZED = Path('docs/reviews/evidence/2026-09-25-image-brush-verification.json')
def entry(path):
    p = Path(path); b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())

assert entry(PREVIOUS)['sha256'] == '3d8ef124acce831347b8bc967901a0b66c9e7e78ca74d26ab3f833c371e38220'
old = json.loads(PREVIOUS.read_text())
allowed = {
 'Cargo.lock', 'README.md', 'contracts/README.md',
 'components/skia/mo_skia.cpp', 'components/skia/mo_skia.h',
 'contracts/generated/page-compile-response.schema.json', 'contracts/generated/path-raster-request.schema.json',
 'contracts/generated/pptx-page-compile-response.schema.json', 'contracts/generated/scene-raster-request.schema.json',
 'crates/mo-kernel-api/src/lib.rs', 'crates/mo-raster/src/brush.rs', 'crates/mo-raster/src/compile.rs',
 'crates/mo-raster/src/gradient.rs', 'crates/mo-raster/src/lib.rs', 'crates/mo-skia-sys/Cargo.toml',
 'crates/mo-skia-sys/src/ffi.rs', 'crates/mo-skia-sys/src/lib.rs', 'crates/mo-wasm/src/raster.rs',
 'docs/README.md', 'docs/implementation/development.md', 'docs/implementation/progress.md',
 'packages/contracts/src/generated/page-compile-response.ts', 'packages/contracts/src/generated/path-raster-request.ts',
 'packages/contracts/src/generated/pptx-page-compile-response.ts', 'packages/contracts/src/generated/scene-raster-request.ts',
 'packages/raster-component/src/index.ts', 'tools/components/build-skia.py', 'tools/mo-contract-codegen/src/main.rs',
 'tools/mo-raster-worker/src/main.rs', 'tools/verification/contracts.py', 'tools/verification/skia-probe.cpp',
 'tools/verification/text-page-runtime-regressions.mjs',
}
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
assert changed == allowed, (changed - allowed, allowed - changed)
added = {
 'components/skia/mo_image.cpp', 'components/skia/mo_image.h',
 'crates/mo-raster/src/image.rs', 'crates/mo-raster/src/image/compile.rs', 'crates/mo-raster/src/image/resources.rs',
 'crates/mo-raster/src/image_tests.rs', 'crates/mo-kernel-api/src/image_raster.rs',
 'crates/mo-skia-sys/examples/image_raster.rs', 'docs/implementation/image-raster.md',
 'contracts/generated/image-raster-request.schema.json', 'contracts/generated/image-raster-response.schema.json',
 'packages/contracts/src/generated/image-raster-request.ts', 'packages/contracts/src/generated/image-raster-response.ts',
 'tools/verification/image-brush-fixtures.py', 'tools/verification/image-brush-parity.mjs',
 'tools/verification/image-brush-reference.py', 'tools/verification/image-brush-regressions.mjs',
 'tools/verification/image-brush-evidence.py',
}
sources = {r['path'] for r in old['sourceFiles']} | added
for p in sources:
    if Path(p).suffix in ['.rs', '.cpp', '.h', '.ts', '.py', '.mjs']:
        assert len(Path(p).read_text().splitlines()) <= 2000, p
for r in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(), *old['standardInputs']]:
    assert entry(r['path']) == r
logs = ['workspace-tests.log', 'clippy.log', 'fmt.log', 'workers-build.log', 'wasm-build.log',
 'bindgen.log', 'schema-check.log', 'types-check.log', 'contracts.log', 'parity.log', 'reference.log',
 'regressions.log', 'recent-regressions.log', 'native-build.log', 'wasm-component-build.log', 'asan-component-build.log']
for name in logs:
    s = (ROOT / name).read_text()
    assert not any(v in s for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
new_tests = [n for n in tests if n not in old['rustTestNames']]
assert len(tests) == 505 and len(new_tests) == 6
assert 'Finished' in (ROOT / 'clippy.log').read_text()
assert len(re.findall(r'^check contracts/generated/', (ROOT / 'schema-check.log').read_text(), re.M)) == 72
assert len(re.findall(r'^check .+\.ts$', (ROOT / 'types-check.log').read_text(), re.M)) == 72
contracts = json.loads((ROOT / 'contracts.log').read_text())
assert (contracts['schemas'], contracts['positiveInputs'], contracts['negativeMutations']) == (72, 1, 9)
reports = {n: json.loads((ROOT / (n + '.json')).read_text()) for n in ['parity', 'reference', 'regressions', 'recent-regressions']}
assert reports['parity']['counts'] == dict(rustRequests=39, imageCases=30, preflightFailures=9,
    componentRejections=23, jsonRejections=7, hostFailureRejections=1)
assert reports['reference']['comparedPixels'] == 7680 and reports['reference']['affineCorners'] == 124
assert reports['regressions']['counts'] == dict(requests=307, pageRequests=100, textRequests=207, pixelOutputs=21)
assert reports['recent-regressions']['counts'] == dict(requests=165, textPages=124, geometry=41)
# Require completion markers, not merely absence of error text. An empty log
# while a replay is still running must never be included in a final seal.
for name, expected in {
 'parity': dict(rustRequests=39, imageCases=30, componentRejections=23),
 'reference': dict(cases=30, comparedPixels=7680, affineCorners=124),
 'regressions': dict(requests=307, pixelOutputs=21),
 'recent-regressions': dict(requests=165, textPages=124, geometry=41),
}.items():
    assert json.loads((ROOT / (name + '.log')).read_text()) == expected, name
records = 0
def audit(v):
    global records
    if isinstance(v, dict):
        if {'path', 'byteLength', 'sha256'} <= v.keys():
            assert entry(v['path']) == v, v['path']; records += 1
        for x in v.values(): audit(x)
    elif isinstance(v, list):
        for x in v: audit(x)
audit(reports)
def schema(name):
    return Draft202012Validator(json.loads(Path('contracts/generated/' + name + '.schema.json').read_text()))
request_schema, response_schema = schema('image-raster-request'), schema('image-raster-response')
for c in reports['parity']['cases']:
    request_schema.validate(json.loads(Path(c['request']['path']).read_text()))
    response_schema.validate(json.loads(Path(c['runtime']['path']).read_text()))
builds = {}
for name in ['native', 'wasm', 'native-asan']:
    p = ROOT / 'component' / (name + '-build.json'); b = json.loads(p.read_text())
    for key in ['componentSources', 'artifacts']: audit(b[key])
    builds[name] = dict(record=entry(p), artifacts=b['artifacts'], sourceFiles=b['componentSources'],
        compiler=b['compiler'], profile=b['profile'], sanitizers=b['sanitizers'])
# Dependency graph contains no new package/version. Only example dev-dependency
# edges to already locked serde/serde_json are added to mo-skia-sys.
native_links = subprocess.check_output(['otool', '-L', str(ROOT / 'component/mo-skia-probe')], text=True).splitlines()[1:]
old_links = subprocess.check_output(['otool', '-L', '.codex-work/skia/mo-skia-probe'], text=True).splitlines()[1:]
assert native_links == old_links
component = entry(ROOT / 'component/mo-skia.wasm'); prior_component = entry('.codex-work/skia/mo-skia.wasm')
assert (component['byteLength'], prior_component['byteLength']) == (1810991, 1808300)
report = dict(format='musteroffice.image-brush-verification/1', previousEvidence=entry(PREVIOUS),
 supersedesUnfinalizedRecord=entry(UNFINALIZED),
 supersededRecordReason='The earlier candidate captured the zero-byte regressions log before the final replay completed. It is retained as history, not valid final evidence. This record requires and binds every completion marker.',
 scope='Shared raw RGBA image resources, Q32 affine brushes, premultiplied filtering, Native Worker / Rust WASM APIs; not complete PPTX picture import.',
 checks=dict(rustTests=505, newRustTests=6, strictClippy=True, rustfmt=True, schemasChecked=72,
    generatedTypeScriptAndTypeCheck=True, genericContractNegativeMutations=9,
    imageNativeWasmRequests=39, imageSuccess=30, imagePreflightFailures=9, jsonRejections=7,
    wasmHostFailureRejections=1, directNativeWasmAsanRejections=23,
    oldNativeWasmRequests=472, currentPairedRuntimeRequests=518,
    independentlyComparedPixels=7680, independentlyCheckedAffineCorners=124,
    channelRoundingTolerance=1, priorResponsesAndPixelsUnchanged=True),
 sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
 rustTestNames=tests, newTestNames=new_tests, validationLogs=[entry(ROOT / n) for n in logs],
 reports={n: dict(record=entry(ROOT / (n + '.json')), evidence=v) for n, v in reports.items()},
 componentBuilds=builds, nativeDynamicLinks=native_links,
 artifacts=[entry(p) for p in ['target/debug/mo-raster-worker', 'target/debug/mo-text-worker', 'target/debug/mo-cli',
   'target/debug/examples/image_raster', 'target/wasm32-unknown-unknown/debug/mo_wasm.wasm',
   ROOT / 'wasm-node/mo_wasm.js', ROOT / 'wasm-node/mo_wasm_bg.wasm', ROOT / 'wasm-node/mo_wasm.d.ts',
   ROOT / 'wasm-node/mo_wasm_bg.wasm.d.ts', ROOT / 'wasm-node/package.json', ROOT / 'ts-raster/index.js']],
 previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'], standardInputs=old['standardInputs'],
 componentSize=dict(current=component, previous=prior_component, uncompressedWasmDeltaBytes=2691,
    productInstallerEstimate=False),
 limitations=[
  'Already decoded/normalized sRGB RGBA8 only. PPTX picture relationships, decoding/ICC, native crop/tile/effects lowering and real application fidelity remain unconnected.',
  'Exact affine-input quantization bound excludes shader inverse/sample arithmetic and pixel coverage. Independent current reference uses integer rectangular path coverage, not new AA or stroke-image acceptance.',
  'Straight RGBA8 is converted once per resource per component call; premul resources are borrowed in native calls. Wasm transfers, per-call JSON digest validation, resident image cache and frame budgets remain work.',
  'ASan/UBSan probes run on all 30 successful image cases and 23 direct rejections. LeakSanitizer is unavailable on this macOS toolchain and is not claimed.',
  '472 old requests are current replays; historical larger totals are separate. 518 paired runtime requests = 472 old + 39 image + 7 malformed JSON; the extra injected bad-host check is WASM-only.',
  'No new product latency/RSS or installer measurement. Component bytes are development build observations, not complete office-kernel packaging results.',
  'Full editing/export/player/advanced content and SDK/MCP/Skill/Plugin/Artifact/Musterwork E0-E3 replacement acceptance remain incomplete.',
 ])
raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f: f.write(raw)
elif OUTPUT.exists():
    assert OUTPUT.read_text() == raw, 'sealed evidence differs from current inputs'
print(json.dumps(dict(sources=len(sources), changed=len(changed), added=len(added), boundRecords=records,
    checks=report['checks'], evidence=entry(OUTPUT) if OUTPUT.exists() else None)))
