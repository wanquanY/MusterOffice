"""Record immutable component evidence after builds and pixel/failure verification."""
import hashlib
import json
import platform
from pathlib import Path

root = Path('.codex-work/skia')
output = Path('docs/reviews/evidence/2026-09-24-skia-component-verification.json')
assert not output.exists(), 'Evidence is immutable; use a new record for a new milestone.'


def read(path):
    return json.loads(Path(path).read_text())


def entry(path):
    p = Path(path)
    data = p.read_bytes()
    return {'path': str(p), 'byteLength': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


prior_path = Path('docs/reviews/evidence/2026-09-24-paragraph-paths-verification.json')
prior = read(prior_path)
for artifact in prior['artifacts'].values():
    assert entry(artifact['path']) == artifact
changed = []
for source in prior['sourceFiles']:
    if entry(source['path']) != source:
        changed.append(source['path'])
assert set(changed) == {'docs/implementation/dependencies.md', 'docs/implementation/development.md',
                        'docs/implementation/progress.md', 'package.json'}
builds = {name: read(root / (name + '-build.json')) for name in ['native', 'wasm', 'native-asan']}
lock = read('components/skia/lock.json')
for build in builds.values():
    assert build['lock'] == lock
    for artifact in build['artifacts'] + build['componentSources']:
        assert entry(artifact['path']) == artifact
    for key in ['gn', 'ninja', 'targetGraph']:
        assert entry(build[key]['path']) == build[key]
artifacts = {
    'nativeProbe': entry(root / 'mo-skia-probe'),
    'nativeSanitizerProbe': entry(root / 'mo-skia-probe-asan'),
    'wasm': entry(root / 'mo-skia.wasm'),
    'glue': entry(root / 'mo-skia.mjs'),
    'typescriptAdapter': entry('.codex-work/raster-component/index.js'),
    'nativeArchive': builds['native']['artifacts'][0],
    'wasmArchive': builds['wasm']['artifacts'][0],
}
report = read(root / 'verification/parity.json')
for key, name in [('nativeSha256', 'nativeProbe'), ('asanSha256', 'nativeSanitizerProbe'),
                  ('wasmSha256', 'wasm'), ('glueSha256', 'glue'), ('adapterSha256', 'typescriptAdapter')]:
    assert report[key] == artifacts[name]['sha256']
assert len(report['cases']) == 148 and len(report['paragraphMapping']) == 29
assert report['oraclePixels'] == 61444 and report['maxChannelDifference'] == 0
assert report['actualAllocationFailure']['status'] == 2 and report['actualAllocationFailure']['replacementVerified']
assert report['actualSkiaAllocationTrap']['outputSlotsZero'] and report['actualSkiaAllocationTrap']['reuseRejected']
assert len(report['protocolFailures']) == 4
for case in report['cases']:
    assert entry(case['requestPath'])['sha256'] == case['requestSha256']
    if case['status'] == 0:
        for suffix, key in [('.rgba', 'nativeSha256'), ('.wasm.rgba', 'wasmSha256')]:
            assert entry(root / 'verification' / (case['name'] + suffix))['sha256'] == case[key]
    assert case['sanitizerMatches']
previews = read(root / 'verification/previews.json')
assert len(previews) == 7
for preview in previews:
    assert entry(preview['path']) == {k: preview[k] for k in ['path', 'byteLength', 'sha256']}
    assert preview['pixelsSha256'] == entry(Path(preview['path']).with_suffix('.rgba'))['sha256']
closure = read(root / 'verification/compiled-closure.json')
for target, details in closure.items():
    source = next((root / ('source-' + target)).glob('skia-*'))
    for unit in details['translationUnits']:
        actual = entry(source / unit['path'])
        assert actual['sha256'] == unit['sha256'] and actual['byteLength'] == unit['byteLength']
        assert not unit['path'].startswith('third_party/')
assert len(closure['native']['translationUnits']) == 497
assert len(closure['wasm']['translationUnits']) == 496
files = [p for base in ['components/skia', 'packages/raster-component'] for p in Path(base).rglob('*') if p.is_file()]
files += [Path(p) for p in ['tools/components/build-skia.py', 'tools/verification/skia-probe.cpp',
          'tools/verification/skia-fixtures.py', 'tools/verification/skia-parity.mjs',
          'tools/verification/skia-previews.py', 'tools/verification/skia-evidence.py',
          'docs/implementation/skia-component.md', 'docs/implementation/dependencies.md',
          'docs/implementation/development.md', 'docs/implementation/progress.md', 'docs/README.md', 'package.json']]
for p in files:
    if p.suffix in ['.cpp', '.h', '.ts', '.mjs', '.py']:
        assert len(p.read_text().splitlines()) <= 2000
assert 'error TS' not in (root / 'verification/typecheck.log').read_text()
result = {
    'format': 'musteroffice.skia-component-verification/1',
    'scope': 'Independent CPU device-space path raster component and thin WASM host boundary. Not Rust public rendering, full Draw IR, production worker or target-application acceptance.',
    'previousEvidence': entry(prior_path),
    'environment': {'platform': platform.system() + ' ' + platform.release(), 'architecture': platform.machine(),
                    'node': '23.5.0', 'python': platform.python_version()},
    'artifacts': artifacts, 'builds': builds,
    'checks': {'componentNativeWasmBatches': 148, 'nativeSanitizerBatches': 148, 'paragraphScenes': 29,
               'independentReferencePixels': 61444, 'maximumObservedChannelDifference': 0,
               'actualOutputAllocationFailure': True, 'actualUpstreamAllocationAbort': True,
               'protocolFailureCases': 4, 'strictTypescript': True, 'strictCppAdapterWarnings': True,
               'previousMainKernelArtifactsAndRuntimeSourcesUnchanged': True,
               'mainKernelBatchesRerunThisStage': 0},
    'parityReport': report, 'previews': previews,
    'visualInspection': {'renderer': 'Actual pinned component RGBA, encoded as PNG by Python stdlib',
                         'names': ['paragraph-bidi-brackets', 'paragraph-cjk-punctuation', 'paragraph-cubic'],
                         'observation': 'Full glyph paths visible, including inner contour holes. No Office/WPS comparison claimed.'},
    'compiledTranslationUnits': closure,
    'buildToolSources': {
        'skiaArchive': entry(root / f"skia-{lock['commit']}.tar.gz"),
        'gn': {'upstream': 'https://gn.googlesource.com/gn', 'mirror': 'https://github.com/ArthurSonzogni/gn',
               'commit': lock['gnCommit'], 'objectIntegrityChecked': True,
               'license': entry(root / 'gn-source/LICENSE'), 'buildCommand': 'python3 build/gen.py && ninja -C out gn -j 8'},
        'ninja': {'url': 'https://github.com/ninja-build/ninja/releases/download/v1.13.2/ninja-mac.zip',
                  'archive': entry(root / 'ninja.zip'), 'license': entry(root / 'ninja-COPYING')},
    },
    'previousMainKernelArtifacts': prior['artifacts'],
    'sourceFiles': [entry(p) for p in sorted(set(files))],
    'limitations': [
        'New batch counts are independent component tests, not added to the prior 1883 Rust kernel batches. No existing Rust/Cargo/HarfBuzz/runtime contract changed; previous artifact and source hashes were checked rather than claiming a full kernel rerun.',
        'Current API covers solid antialiased path fill, nonzero/evenodd, translations and fixed source-over to premultiplied RGBA8 sRGB. Complete brushes, images, user clipping, transforms, effects, hinting, color fonts, GPU and presentation playback remain incomplete.',
        'Diagnostic scene conversion uses exact-origin subtraction and Fraction scaling before float32. A production Rust Draw IR compiler and complete coordinate error budget remain to be implemented.',
        'macOS arm64 and Node WASM were exercised. Windows, Linux and browser execution are not verified. Native isolation here is a test process with timeout, not a production pool with cancellation and RSS enforcement.',
        'WASM linear memory is capped at 256 MiB, excluding JS inputs/copies and other modules. Native Skia allocations are not fully accounted in the host resource budget.',
        'Heap exhaustion tests use actual allocation failure/abort. The upstream-abort test reserves transport slots before exhaustion to reach a Skia allocation; it is not an exhaustive per-allocation failure campaign.',
        'The native static archive still contains unused platform code; dead-strip yields a probe with only libc++ and libSystem dynamic dependencies. Future exported capabilities require a new closure audit.',
        'Build integration modifies three GN files only, adding explicit SDK activation and thread settings. All Skia drawing source remains unchanged. The existing SDK binary archive digest is unavailable; full release toolchain reproducibility and runtime notices are not claimed.',
        'Observed channel equality applies to this corpus and configuration. It does not establish cross-device bit equality or Office/WPS visual quality.',
        'Recorded byte sizes are uncompressed incomplete component artifacts. They are not complete kernel distribution, product performance evidence or a Musterwork installer estimate.',
        'Full advanced editable objects, animation/media, Agent integrations, Office/WPS interoperability and Musterwork E0-E3 replacement gates remain incomplete.',
    ],
}
encoded = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in encoded
output.write_text(encoded)
print(json.dumps({'evidence': entry(output), 'sourceFiles': len(files),
                  'artifacts': {k: v['byteLength'] for k, v in artifacts.items()}}, indent=2))
