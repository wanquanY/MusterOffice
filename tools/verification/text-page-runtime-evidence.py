"""Seal the current development text-page runtime, not a product release."""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path('.codex-work/text-page-runtime')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-source-text-page-library-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-text-page-runtime-verification.json')
def entry(path):
    p = Path(path); b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())
assert entry(PREVIOUS)['sha256'] == '5f39994c5f0e0c7fe53fc7e8114d2d1ec650f9a1732ec07c797bb9208364c0b5'
old = json.loads(PREVIOUS.read_text())
allowed = {
    'Cargo.lock', 'README.md', 'crates/mo-harfbuzz-sys/tests/source_text_page.rs',
    'crates/mo-kernel-api/src/lib.rs', 'crates/mo-kernel-api/src/pptx_page.rs',
    'crates/mo-presentation-compile/src/source_frame/types.rs',
    'crates/mo-presentation-compile/src/source_page.rs',
    'crates/mo-presentation-compile/src/source_page/types.rs',
    'crates/mo-presentation-compile/src/source_text/types.rs',
    'crates/mo-presentation-compile/src/source_text_page.rs',
    'crates/mo-presentation-compile/src/source_text_page/types.rs',
    'crates/mo-wasm/src/raster.rs', 'crates/mo-wasm/src/text.rs',
    'docs/README.md', 'docs/implementation/development.md', 'docs/implementation/progress.md',
    'docs/implementation/source-text-page.md', 'packages/contracts/tests/wire.ts',
    'tools/mo-cli/src/main.rs', 'tools/mo-cli/src/raster.rs',
    'tools/mo-contract-codegen/src/main.rs', 'tools/mo-raster-worker/Cargo.toml',
    'tools/mo-raster-worker/src/main.rs',
}
changed = [p['path'] for p in old['sourceFiles'] if entry(p['path']) != p]
assert set(changed) == allowed, (set(changed)-allowed, allowed-set(changed))
added = [
    'crates/mo-kernel-api/src/pptx_text_page.rs',
    'crates/mo-kernel-api/src/pptx_text_page/diagnostic.rs',
    'crates/mo-kernel-api/tests/pptx_text_page.rs',
    'tools/test-support/source_text_page.rs', 'docs/implementation/text-page-runtime.md',
    *['tools/verification/text-page-runtime-'+s for s in
      ['fixtures.py', 'parity.mjs', 'regressions.mjs', 'contracts.py', 'evidence.py']],
    *['contracts/generated/'+s+'.schema.json' for s in
      ['pptx-text-page-request', 'pptx-text-page-raster-response']],
    *['packages/contracts/src/generated/'+s+'.ts' for s in
      ['pptx-text-page-request', 'pptx-text-page-raster-response']],
]
sources = {p['path'] for p in old['sourceFiles']} | set(added)
assert len(sources) == len(old['sourceFiles']) + len(added)
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.py', '.mjs', '.h', '.cpp']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
lock = Path('Cargo.lock').read_text(); start = lock.index('name = "mo-raster-worker"'); end = lock.index('[[package]]', start)
section = lock[start:end]; line = ' "mo-harfbuzz-sys",\n'; assert section.count(line) == 1
restored = lock[:start] + section.replace(line, '') + lock[end:]
assert hashlib.sha256(restored.encode()).hexdigest() == next(p['sha256'] for p in old['sourceFiles'] if p['path'] == 'Cargo.lock')
for record in [*old['previousReleaseArtifactsVerifiedUnchanged'].values(), *old['standardInputs']]:
    assert entry(record['path']) == record
logs = ['api-tests.log', 'workspace-tests.log', 'clippy.log', 'fmt.log', 'workers-build.log',
        'wasm-build.log', 'schema-check.log', 'types-check.log', 'parity.log', 'regressions.log', 'contracts.log']
for name in logs:
    content = (ROOT/name).read_text()
    assert not any(s in content for s in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT/'workspace-tests.log').read_text(), re.M)
new = [s for s in tests if s not in old['rustTestNames']]
assert len(tests) == 463 and len(new) == 2
assert len(re.findall(r'^check contracts/generated/', (ROOT/'schema-check.log').read_text(), re.M)) == 70
assert len(re.findall(r'^check .+\.ts$', (ROOT/'types-check.log').read_text(), re.M)) == 70
parity = json.loads((ROOT/'parity.json').read_text())
assert parity['counts'] == dict(parityRequests=27, rendered=11, cliRequests=27, overwriteRejections=11, workerFramingRejections=5)
assert parity['batchRequests'] == 2 and parity['fault']['replacementVerified']
assert parity['editRoundtrip']['nativeWasmPackagesIdentical'] and parity['editRoundtrip']['staleBindingRejected']
regressions = json.loads((ROOT/'regressions.json').read_text())
assert regressions['counts'] == dict(requests=307, pageRequests=100, textRequests=207, pixelOutputs=21)
contracts = json.loads((ROOT/'contracts.json').read_text())
assert (contracts['validRequests'], contracts['rejectedRequests'], contracts['responses']) == (23, 4, 30)
def audit(value):
    if isinstance(value, dict):
        if {'path', 'sha256', 'byteLength'} <= value.keys():
            assert entry(value['path']) == value, value['path']
        for v in value.values(): audit(v)
    elif isinstance(value, list):
        for v in value: audit(v)
for value in [parity, regressions, old['nativeComponents']]: audit(value)
report = dict(
    format='musteroffice.text-page-runtime-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Current development Native CLI/combined worker and Rust WASM text-page rendering with explicit source/font resources; editable native leaf update followed by actual rendering. Not a release or Musterwork acceptance.',
    checks=dict(rustTests=463, newRustTests=2, strictClippy=True, rustfmt=True, schemasChecked=70,
                generatedTypeScriptAndTypeCheck=True, newNativeWasmRequests=27, newRenderedPages=11,
                oldNativeWasmRequests=307, oldPixelOutputs=21, totalCurrentParityRequests=334,
                cliRequests=27, overwriteRejections=11, workerFramingRejections=5,
                realFontAllocationFailure=True, rasterHostFailure=True, editThenRender=True),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=tests, newTestNames=new, validationLogs=[entry(ROOT/n) for n in logs],
    parity=entry(ROOT/'parity.json'), parityEvidence=parity,
    regressions=entry(ROOT/'regressions.json'), regressionEvidence=regressions,
    contracts=entry(ROOT/'contracts.json'), contractEvidence=contracts,
    nativeComponents=old['nativeComponents'],
    rawRustWasm=entry('target/wasm32-unknown-unknown/debug/mo_wasm.wasm'),
    wasmBindgen=entry('.codex-work/toolchain/bin/wasm-bindgen'),
    wasmDeclarations=[entry(ROOT/'wasm-node'/n) for n in ['mo_wasm.d.ts', 'mo_wasm_bg.wasm.d.ts', 'package.json']],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'], standardInputs=old['standardInputs'],
    dependencyChanges='One local runtime dependency from mo-raster-worker to the existing mo-harfbuzz-sys. Exact Cargo.lock reconstruction verifies no external version change.',
    limitations=[
        '27 new operation and 307 old operation requests executed in both current Native and WASM. Historical 10064 runtime batches were not all replayed and are not counted here.',
        'Source/shape/text profiles remain draft; static solid text only, retaining complete phase-one scope. Advanced text, graphics, animation, media, SmartArt and equations are not completed.',
        '11 successful synthetic-font page images, 10 matched to previous independent-coordinate/interior-pixel oracle; one follows a real native leaf-text edit. Exact cross-runtime pixels do not establish Office/WPS or real-font visual fidelity.',
        'Typed operation stages and source object/native-reason diagnostics are implemented. Non-declaration paint run locations and actionable missing-font selection contracts still require refinement.',
        'Debug Native and debug Rust WASM plus existing pinned components were used; old 11 release artifacts are unchanged. No product latency/RSS/installer claims.',
        'Core owns source/font/render computation; hosts own explicit resources, component lifetimes and publication. Production worker pools, user cancellation, permissions, task persistence, Artifact/CAS and Musterwork integration remain pending.',
        'CLI refuses existing output, failure has no image, and transport limits are checked before allocation. These checks do not prove total resident-memory bounds.',
        'No Office/WPS edit/save/reopen/playback acceptance, production Agent distribution, full capability completion or E0-E3 signoff.',
    ])
raw = json.dumps(report, ensure_ascii=False, indent=2)+'\n'; assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as file: file.write(raw)
print(json.dumps(dict(sources=len(sources), checks=report['checks'], evidence=entry(OUTPUT) if '--seal' in sys.argv else None)))
