"""Seal native text declarations, real edits and strict prior-result comparison."""
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path('.codex-work/source-text')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-source-page-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-source-text-verification.json')


def read(path):
    return json.loads(Path(path).read_text())


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return {'path': str(path), 'byteLength': len(data),
            'sha256': hashlib.sha256(data).hexdigest()}


def bound_files(value):
    if isinstance(value, list):
        for child in value:
            bound_files(child)
    elif isinstance(value, dict):
        if 'path' in value and 'sha256' in value:
            assert entry(value['path'])['sha256'] == value['sha256'], value['path']
        for key, child in value.items():
            if key.endswith('Path') and key[:-4] + 'Sha256' in value:
                assert entry(child)['sha256'] == value[key[:-4] + 'Sha256'], child
            if isinstance(child, (dict, list)):
                bound_files(child)


assert entry(PREVIOUS)['sha256'] == 'da4b661645b8a4dbd5f6614de8be3c5755754f65f1cdc8f53b160f453823ca72'
old = read(PREVIOUS)
artifacts = {key: entry(value['path']) for key, value in old['artifacts'].items()}
for key in ['cppWasm', 'cppWasmGlue', 'typescriptAdapter',
            'rasterWasm', 'rasterWasmGlue', 'rasterAdapter', 'rustWasmGlue']:
    assert artifacts[key] == old['artifacts'][key], key
assert artifacts['nativeWorker']['byteLength'] == old['artifacts']['nativeWorker']['byteLength']
for path in ['Cargo.lock', 'pnpm-lock.yaml']:
    assert entry(path) == next(value for value in old['sourceFiles'] if value['path'] == path)


def current_artifacts(report, raster=False):
    aliases = {
        'nativeSha256': 'nativeCli', 'nativeCliSha256': 'nativeCli',
        'wasmSha256': 'rustWasm', 'rustWasmSha256': 'rustWasm',
        'wasmKernelSha256': 'rustWasm',
        'nativeWorkerSha256': 'nativeRasterWorker' if raster else 'nativeWorker',
        'nativeRasterWorkerSha256': 'nativeRasterWorker',
        'componentSha256': 'rasterWasm' if raster else 'cppWasm',
        'componentWasmSha256': 'cppWasm', 'adapterSha256': 'rasterAdapter',
    }
    for field, artifact in aliases.items():
        if field in report:
            assert report[field] == artifacts[artifact]['sha256'], (field, artifact)


audit = read(ROOT / 'regression-audit.json')
assert audit['previousEvidenceSha256'] == entry(PREVIOUS)['sha256']
assert (audit['unchangedBatches'], audit['migratedBatches']) == (7854, 1647)
assert audit['additions'] == {'text': 8067}
regressions = {}
raster_reports = {'placement', 'groups', 'pathRaster', 'sceneRaster', 'pageRaster',
                  'gradientRaster', 'presetExpansion', 'sourcePlacement', 'sourcePage'}
for name, info in audit['reports'].items():
    snapshot = ROOT / 'previous' / (name + '.json')
    assert entry(snapshot)['sha256'] == old['regressionReports'][name]['report']['sha256']
    report = read(info['reportPath'])
    current_artifacts(report, name in raster_reports)
    field = 'batches' if name == 'bidiConformance' else 'cases'
    bound_files(report[field])
    count = len(report[field])
    assert count == info['casesUnchanged'] + info['additiveTextCatalogOnly']
    regressions[name] = {'report': entry(info['reportPath']), 'batches': count,
                         'migrationAudit': info}
assert len(regressions) == 44
assert sum(item['batches'] for item in regressions.values()) == 9501

parity = read(ROOT / 'parity.json')
current_artifacts(parity)
bound_files(parity)
assert parity['exactIndexAndCandidateBytes']
assert len(parity['cases']) == 269
assert {status: sum(case['status'] == status for case in parity['cases'])
        for status in ['inspected', 'edited', 'INPUT_INVALID']} == {
            'inspected': 124, 'edited': 124, 'INPUT_INVALID': 21}
assert all(case['textDeclarationsPreserved'] for case in parity['cases'] if case['status'] == 'edited')
manifest = read(ROOT / 'manifest.json')
bound_files(manifest)
assert len(manifest['cases']) == 145
assert sum(case['error'] for case in manifest['cases']) == 21
reference = read(ROOT / 'reference.json')
bound_files(reference)
assert reference['counts'] == {
    'catalogs': 620, 'nodes': 8437, 'attributes': 4119, 'retainedLocations': 3,
    'textLeaves': 498, 'editedPackages': 124, 'unchangedCompressedParts': 2480,
    'validModifiedParts': 246, 'invalidModifiedParts': 23, 'validModifiedStyleParts': 6,
}
by_name = {case['name']: case for case in parity['cases']}
assert len(reference['cases']) == 124
for case in reference['cases']:
    assert case['responseSha256'] == by_name['inspect-' + case['name']]['responseSha256']
enums = read(ROOT / 'enums.json')
bound_files(enums)
assert (enums['enumTypes'], enums['values']) == (13, 103)
contracts = read(ROOT / 'contracts-combined.log')
assert (contracts['schemas'], contracts['sourceTextResponses'],
        contracts['sourceTextEditRequests']) == (66, 269, 124)
assert (contracts['sourcePageResponses'], contracts['sourcePageRequests'],
        contracts['sourceResponses'], contracts['sourceRequests']) == (100, 66, 73, 95)
splitting = read(ROOT / 'type-splitting.log')
assert splitting == {'compiler': '7.0.2', 'declarations': 382, 'modules': 4,
                     'cyclicImportsAndHeritageChecked': True, 'publicEntryChecked': True,
                     'negativeTypeAssignmentChecked': True, 'deterministic': True,
                     'oversizeAndUnexpectedDeclarationsRejected': True}
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
assert len(tests) == 392
logs = ['workspace-tests.log', 'clippy.log', 'fmt.log', 'build-native.log',
        'build-wasm.log', 'wasm-bindgen.log', 'schema-check.log', 'types-check.log',
        'types-write.log', 'type-splitting.log', 'fixtures.log', 'parity.log',
        'reference.log', 'enums.log', 'regressions.log', 'regression-audit.log',
        'contracts-combined.log']
logs += [path.name for path in sorted(ROOT.glob('*-regression.log'))]
for name in logs:
    text = (ROOT / name).read_text()
    assert not any(word in text for word in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
changed = [item['path'] for item in old['sourceFiles']
           if item['path'].startswith('contracts/generated/') and entry(item['path']) != item]
assert changed == ['contracts/generated/pptx-source-response.schema.json']
assert len(list(Path('contracts/generated').glob('*.schema.json'))) == 66

sources = {item['path'] for item in old['sourceFiles']}
for base in ['components', 'crates', 'tools', 'contracts', 'packages', 'fixtures', 'docs/implementation']:
    for path in Path(base).rglob('*'):
        if path.is_file() and '__pycache__' not in path.parts and path.suffix in [
                '.rs', '.toml', '.json', '.ts', '.mjs', '.py', '.md', '.h', '.cpp',
                '.bin', '.xml', '.ttf', '.otf', '.ttc', '.patch', '.txt']:
            sources.add(str(path))
for path in sources:
    if Path(path).suffix in ['.rs', '.ts', '.mjs', '.py', '.cpp', '.h']:
        assert len(Path(path).read_text().splitlines()) <= 2000, path
environment = json.loads(subprocess.check_output([
    'node', '--input-type=module', '-e',
    'import os from "node:os";console.log(JSON.stringify({platform:os.platform(),'
    'release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,'
    'memoryBytes:os.totalmem(),node:process.version}));'], text=True))
assert environment == old['environment']
regressions['sourceText'] = {'report': entry(ROOT / 'parity.json'), 'batches': 269}
result = {
    'format': 'musteroffice.source-text-verification/1', 'previousEvidence': entry(PREVIOUS),
    'checks': {'rustTests': 392, 'newRustTests': 8, 'nativeWasmLogicalBatches': 9770,
               'previousLogicalBatchesRerun': 9501, 'newSourceTextBatches': 269,
               'nativeInputPackages': 145, 'nativeEditedPackages': 124,
               'runtimeSchemas': 66, 'changedExistingSchemas': 1, 'newSchemas': 0,
               'strictClippy': True, 'rustfmt': True, 'typescript': True},
    'scope': 'Native source text declarations and source-preserving leaf edits. '
             'Inheritance, font binding, source text layout/rendering and Musterwork integration remain incomplete.',
    'artifacts': artifacts,
    'artifactByteDeltas': {key: value['byteLength'] - old['artifacts'][key]['byteLength']
                          for key, value in artifacts.items()},
    'artifactSizeScope': 'Uncompressed incomplete development artifacts, not the complete kernel or '
                         'Musterwork installer. No new product latency/RSS measurements.',
    'environment': environment, 'regressionReports': regressions, 'migrationAudit': audit,
    'textReport': parity, 'independentReference': reference, 'enumReference': enums,
    'contracts': contracts, 'typescriptDeclarationSplitting': splitting,
    'fixtureManifest': entry(ROOT / 'manifest.json'),
    'rustTestNames': tests, 'sourceFiles': [entry(path) for path in sorted(sources)],
    'validationLogs': [entry(ROOT / name) for name in logs], 'changedSchemas': changed,
    'dependencyChanges': {'externalRuntimeVersions': [], 'pnpmLockUnchanged': True,
                          'cargoLockUnchanged': True},
    'limitations': [
        'Declarations preserve explicit absence/false/zero and source identity; they do not resolve effective defaults, inheritance or theme fonts.',
        'Autofit values are recorded, not applied. Source page rendering still rejects visible text bodies.',
        'Field identifiers and hyperlink actions are data only. Relationships, field computation and interaction are not evaluated.',
        'WordArt, 3D, image bullets, extensions and unknown attributes/subtrees remain uninterpreted. xml:space is retained; text is not normalized.',
        'The XSD checks cover changed slide/presentation/master parts, not entire package conformance. The retained unknown attribute fixture and its edited form are intentionally invalid.',
        'No source style/tree edits, complete advanced text playback or Office/WPS open/edit/save acceptance were performed.',
        'The TypeScript 7.0.2 unstable AST API is pinned and tested as a development-only generator dependency.',
        'Native validation is macOS arm64 and WASM uses Node. Browser/other-platform workers, production latency/memory/installer gates, full Agent services and Musterwork E0-E3 remain pending.',
    ],
}
raw = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
assert '/Users/' not in raw
summary = {'checks': result['checks'], 'sourceFiles': len(sources),
           'artifactByteDeltas': result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as handle:
        handle.write(raw)
    summary['evidence'] = entry(OUTPUT)
print(json.dumps(summary, indent=2))
