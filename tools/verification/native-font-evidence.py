"""Freeze native typeface and explicit manifest library evidence; no runtime parity claim."""
import hashlib
import json
from pathlib import Path
import platform
import re
import sys

ROOT = Path('.codex-work/text-fonts')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-text-cascade-library-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-native-fonts-library-verification.json')


def entry(path):
    path = Path(path)
    b = path.read_bytes()
    return dict(path=str(path), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


assert entry(PREVIOUS)['sha256'] == '024a0bcf8221864e41f208f8b3b3546b3a0781f65463daa4b2c180f53a0a2a79'
old = json.loads(PREVIOUS.read_text())
changed = [e['path'] for e in old['sourceFiles'] if entry(e['path']) != e]
allowed = {
    'README.md', 'docs/README.md', 'docs/implementation/development.md',
    'docs/implementation/progress.md', 'docs/implementation/text-cascade.md',
    'crates/mo-harfbuzz-sys/Cargo.toml', 'crates/mo-text/src/lib.rs',
    'crates/mo-text/src/paragraph.rs', 'crates/mo-pptx/tests/text_cascade.rs',
    'crates/mo-pptx/src/source/text.rs', 'crates/mo-pptx/src/source/text/cascade.rs',
    'crates/mo-pptx/src/source/text/cascade/types.rs', 'crates/mo-pptx/src/source/text/cascade/chain.rs',
    'crates/mo-pptx/src/source/theme.rs', 'crates/mo-pptx/src/source/theme/defaults.rs',
    'crates/mo-pptx/src/source/theme/read.rs', 'contracts/generated/pptx-source-response.schema.json',
    'packages/contracts/src/generated/pptx-source-response/part-002.ts',
}
assert set(changed) == allowed, (set(changed) - allowed, allowed - set(changed))
added = [
    'crates/mo-text/src/manifest/mod.rs', 'crates/mo-text/src/manifest/types.rs',
    'crates/mo-text/src/manifest/tests.rs', 'crates/mo-pptx/src/source/text/fonts.rs',
    'crates/mo-pptx/tests/support/text.rs', 'crates/mo-pptx/tests/text_fonts.rs',
    'crates/mo-harfbuzz-sys/examples/manifest_shape.rs', 'fixtures/fonts/manifest-paragraph.json',
    'tools/verification/native-font-reference.py', 'tools/verification/manifest-shaping-reference.py',
    'tools/verification/native-font-evidence.py', 'docs/implementation/native-fonts.md',
]
sources = {e['path'] for e in old['sourceFiles']} | set(added)
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
for artifact in old['previousReleaseArtifactsVerifiedUnchanged'].values():
    assert entry(artifact['path']) == artifact, artifact['path']
for standard in old['standardInputs']:
    assert entry(standard['path']) == standard, standard['path']
# No dependency lock or unlisted schema changes may be hidden in this stage.
assert len(list(Path('contracts/generated').glob('*.schema.json'))) == 68

logs = ['native-font-tests.log', 'cascade-regression-tests.log', 'workspace-tests.log',
        'clippy.log', 'wasm-check.log', 'fmt.log', 'schema-check.log', 'schema-write.log',
        'types-write.log', 'types-check.log', 'font-reference.log', 'manifest-reference.log',
        'native-manifest-build.log', 'cascade-reference.log']
for name in logs:
    content = (ROOT / name).read_text()
    assert not any(s in content for s in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
test_names = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
assert len(test_names) == 428
native_tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'native-font-tests.log').read_text(), re.M)
assert len(native_tests) == 10
manifest_tests = [s for s in test_names if s.startswith('manifest::tests::')]
assert len(manifest_tests) == 5

font_reference = json.loads((ROOT / 'font-reference.json').read_text())
assert font_reference['counts'] == dict(cases=30, named=21, unresolved=9, validParts=119,
                                        invalidParts=2, themeBindings=19, supplementalBindings=3,
                                        overrides=1, uniquePackages=27)
for c in font_reference['cases']:
    assert entry(c['sourcePath'])['sha256'] == c['sourceSha256']
    assert entry(c['responsePath'])['sha256'] == c['responseSha256']
manifest_reference = json.loads((ROOT / 'manifest-reference.json').read_text())
for e in manifest_reference['inputs'] + [manifest_reference['result']]:
    assert entry(e['path']) == e
assert manifest_reference['verifiedFaces'] == manifest_reference['shapingRuns'] == 1

assert entry('.codex-work/text-cascade/reference.json') == old['reference']
for c in old['independentReference']['cases']:
    for kind in ['source', 'response']:
        path = Path(c[kind + 'Path'])
        assert entry(path)['sha256'] == c[kind + 'Sha256']
        assert entry(ROOT / 'previous/cascade-fixtures' / path.name)['sha256'] == c[kind + 'Sha256']

component_record = Path('.codex-work/harfbuzz/release/native-build.json')
component = json.loads(component_record.read_text())
assert component['lock'] == json.loads(Path('components/harfbuzz/lock.json').read_text())
assert component['faultTests'] is False and component['sanitizers'] is False
library = entry('.codex-work/harfbuzz/release/libmo_harfbuzz.a')
bound = next(e for e in component['artifacts'] if e['path'].endswith('/libmo_harfbuzz.a'))
assert all(library[k] == bound[k] for k in ['sha256', 'byteLength'])
report = dict(
    format='musteroffice.native-fonts-library-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Native typeface Rust library and explicit font manifest paragraph binding. Real Native HarfBuzz probe; no new CLI/WASM operation, runtime parity or source-page text render.',
    checks=dict(rustTests=428, newRustTests=15, strictClippy=True, rustfmt=True,
                wasmTargetCompilation=['mo-pptx', 'mo-text'], schemaGenerationCheck=True,
                typescriptGenerationAndCheck=True, schemaCount=68, expandedSchemas=1,
                oldCascadePackagesUnchanged=24, realNativeManifestProbes=1,
                newRuntimeParityBatches=0, previousFrozenRuntimeParityBatches=10064),
    changedPreviousSources=changed, addedSources=added,
    sourceFiles=[entry(p) for p in sorted(sources)], validationLogs=[entry(ROOT / p) for p in logs],
    rustTestNames=test_names, newNativeFontTestNames=native_tests, newManifestTestNames=manifest_tests,
    fontReference=entry(ROOT / 'font-reference.json'), independentFontReference=font_reference,
    manifestReference=entry(ROOT / 'manifest-reference.json'), independentManifestReference=manifest_reference,
    nativeDevelopmentProbe=dict(executable=entry('target/debug/examples/manifest_shape'),
                                componentLibrary=library, componentBuildRecord=entry(component_record),
                                host=dict(system=platform.system(), machine=platform.machine(),
                                          systemRelease=platform.release(), macOS=platform.mac_ver()[0])),
    oldCascadeReferenceVerifiedUnchanged=old['reference'],
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'],
    limitations=[
        'Native theme member/supplement and fontRef precedence remains a declared draft; no Office/WPS behavioral acceptance.',
        'No automatic native font slot/script/language mapping or source paragraph/page binding. Explicit manifest is a computation subset, not full FontManifest delivery.',
        'Exact name/resource/axis checks do not prove host-selected bold/italic visual semantics, embedding permission or substitution layout equivalence.',
        'Two intentionally invalid font scope/metadata probes are listed. Projection success is not full XSD validity.',
        'One owned Latin/space real Native probe and WASM target compilation do not establish new runtime parity, page rendering or product performance.',
        'No new release build, complete installer size, Office/WPS round trip or Musterwork E0-E3 acceptance.',
    ],
)
raw = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f:
        f.write(raw)
print(json.dumps(dict(checks=report['checks'], sourceFiles=len(sources),
                     evidence=entry(OUTPUT) if '--seal' in sys.argv else None), indent=2))
