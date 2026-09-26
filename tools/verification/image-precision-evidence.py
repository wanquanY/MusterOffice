"""Freeze image parameter precision, runtime parity and bounded reference checks."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote
from jsonschema import Draft202012Validator

ROOT = Path('.codex-work/image-paint')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-shared-clips-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-image-precision-verification.json')

def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())

assert entry(PREVIOUS)['sha256'] == 'a6ff284905c99582c2a2d38bb5d50514f0e337ddbbf641c65d294e90d019e3e0'
old = json.loads(PREVIOUS.read_text())
changed = {r['path'] for r in old['sourceFiles'] if entry(r['path']) != r}
stems = ['image-raster-request', 'image-raster-response', 'image-scene-request', 'image-scene-response',
         'page-compile-response', 'path-raster-request', 'pptx-page-compile-response', 'scene-raster-request']
allowed = {'crates/mo-raster/src/image.rs', 'crates/mo-raster/src/image/compile.rs',
           'crates/mo-raster/src/image_tests.rs', 'crates/mo-render/src/image_tests.rs',
           'README.md', 'docs/README.md', 'docs/implementation/progress.md', 'docs/implementation/development.md'}
for stem in stems:
    allowed.add('contracts/generated/'+stem+'.schema.json')
    allowed.add('packages/contracts/src/generated/'+stem+'.ts')
assert changed == allowed, (changed-allowed, allowed-changed)
added = {'crates/mo-raster/src/image/precision.rs', 'docs/implementation/image-precision.md'}
for suffix in ['parity.mjs', 'reference.py', 'regressions.mjs', 'evidence.py']:
    added.add('tools/verification/image-precision-'+suffix)
sources = {r['path'] for r in old['sourceFiles']} | added
for name in sources:
    p = Path(name)
    if p.suffix in ['.rs', '.cpp', '.h', '.ts', '.py', '.mjs']:
        assert len(p.read_text().splitlines()) <= 2000, name
for name in ['Cargo.lock', 'pnpm-lock.yaml', 'components/skia/lock.json']:
    assert entry(name) == next(r for r in old['sourceFiles'] if r['path'] == name)
logs = ['tests', 'build-workspace', 'clippy', 'fmt', 'rust-wasm', 'schema-check', 'types-check', 'contracts',
        'precision', 'precision-reference', 'precision-regressions']
for name in logs:
    value = (ROOT/(name+'.log')).read_text()
    assert not any(v in value for v in ['error:', 'FAILED', 'Traceback', 'AssertionError', '\nDiff in ']), name
    if name in ['build-workspace', 'clippy', 'rust-wasm']:
        assert 'Finished' in value, name
value = (ROOT/'tests.log').read_text()
assert 'Doc-tests mo_xml' in value
rust_tests = re.findall(r'^test (.+) \.\.\. ok$', value, re.M)
new_tests = [n for n in rust_tests if n not in old['rustTestNames']]
assert (len(rust_tests), len(new_tests)) == (556, 4)
assert len(re.findall(r'^check contracts/generated/', (ROOT/'schema-check.log').read_text(), re.M)) == 78
assert len(re.findall(r'^check .+\.ts$', (ROOT/'types-check.log').read_text(), re.M)) == 78
contract = json.loads((ROOT/'contracts.log').read_text())
assert (contract['schemas'], contract['negativeMutations']) == (78, 9)
reports = {n: json.loads((ROOT/(n+'.json')).read_text()) for n in ['precision', 'precision-reference', 'precision-regressions']}
p = reports['precision']
assert (len(p['cases']), p['pairedCalls'], len(p['failures'])) == (288, 610, 34)
r = reports['precision-reference']
assert (r['cases'], r['comparisons']) == (192, 941760)
r = reports['precision-regressions']
assert (r['pairedCalls'], r['updatedErrorReports']) == (650, 0)
assert 'repeated_affine_quantization_covers_the_viewport_not_just_one_tile ... FAILED' in (ROOT/'reproduction.log').read_text()
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
# This stage reuses all fixed C++ sources, codec dependencies and artifacts.
for record in old['componentBuilds'].values():
    audit(record)
    build = json.loads(Path(record['path']).read_text())
    for key in ['componentSources', 'artifacts', 'imageCodecs']:
        audit(build[key])
for key in ['previousReleaseArtifactsVerifiedUnchanged', 'standardInputs', 'codecInputLock']:
    audit(old[key])
schema_checks = 0
for c in p['cases']:
    for scene in [False, True]:
        stem = 'image-scene' if scene else 'image-raster'
        for suffix, key in [('request', 'sceneRequest' if scene else 'request'), ('response', 'sceneResponse' if scene else 'response')]:
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
probe = ROOT/'probe'
comparisons = json.loads((probe/'pixel-comparison.json').read_text())
assert len(comparisons) == 12 and all(r['rotWithShapeFalseEqualsTrue'] and r['pixelHashes'][0] == r['pixelHashes'][1] for r in comparisons)
assert len(list(probe.glob('*.pptx'))) == 24 and len(list(probe.glob('*.pdf'))) == 24
report = dict(
    format='musteroffice.image-precision-verification/1', previousEvidence=entry(PREVIOUS),
    scope='Shared image input uncertainty and repeat/mirror parameter bounds. Not complete source image pages or Office/WPS/Musterwork acceptance.',
    sourceFiles=[entry(n) for n in sorted(sources)], changedPreviousSources=sorted(changed), addedSources=sorted(added),
    rustTestNames=rust_tests, newTestNames=new_tests,
    checks=dict(rustTests=556, newRustTests=4, strictClippy=True, rustfmt=True, schemas=78,
                generatedTypeScript=True, runtimeSchemaChecks=schema_checks, genericNegativeMutations=9,
                newCases=288, newPairedCalls=610, newPreflightFailures=34,
                exactRationalCases=192, exactRationalComparisons=941760,
                oldRegressionPairedCalls=650, oldMetadataPixelsFramesUnchanged=True,
                fixedComponentsAndThirdPartyResolutionUnchanged=True, markdown=markdown, localLinks=links),
    regressionReproduction=entry(ROOT/'reproduction.log'),
    reports={n: entry(ROOT/(n+'.json')) for n in reports},
    validationLogs={n: entry(ROOT/(n+'.log')) for n in logs},
    artifacts={n: entry(path) for n, path in {
        'nativeWorker': 'target/debug/mo-raster-worker', 'nativeCli': 'target/debug/mo-cli',
        'nativeTextWorker': 'target/debug/mo-text-worker', 'rustWasm': str(ROOT/'wasm-node/mo_wasm_bg.wasm'),
        'rustWasmGlue': str(ROOT/'wasm-node/mo_wasm.js'), 'typescriptAdapter': '.codex-work/clips/ts-raster/index.js',
        'cppWasm': '.codex-work/clips/component/mo-skia.wasm', 'cppWasmGlue': '.codex-work/clips/component/mo-skia.mjs',
    }.items()},
    rotationExperiment=dict(application='LibreOffice 26.2.0.3 afbbd0df0edb6d40b450b0337ac646b0913a760c',
        inputs=24, switchPairs=12, allPairsPixelIdentical=True,
        conclusion='This corpus does not establish rotWithShape=false target behavior. WPS not exercised: GUI permissions unavailable.',
        files=[entry(p) for p in sorted(probe.glob('*')) if p.is_file()]+[entry(ROOT/'probe.py'), entry(ROOT/'lo-export.log')]),
    previousReleaseArtifactsVerifiedUnchanged=old['previousReleaseArtifactsVerifiedUnchanged'],
    standardInputs=old['standardInputs'], codecInputLock=old['codecInputLock'],
    limitations=['Parameter bounds exclude shader inverse arithmetic, filter colors and AA coverage.',
                 'No new C++ sampler or sanitizer run; component artifacts verified unchanged.',
                 'Source paint orientation, world composition, ordered image/text page rendering remain unfinished.',
                 'No new installer, latency/RSS or Office/WPS editing acceptance; full goal remains open.'],
)
if '--seal' in sys.argv:
    assert not OUTPUT.exists(), 'evidence is immutable after sealing'
    OUTPUT.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
else:
    assert json.loads(OUTPUT.read_text()) == report, 'completed evidence differs from current closure'
print(json.dumps(dict(evidence=entry(OUTPUT), sources=len(sources), auditedRecords=records)))
