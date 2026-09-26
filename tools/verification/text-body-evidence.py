"""Seal body inheritance, native theme defaults and strict prior-result comparison."""
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path('.codex-work/text-style')
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-source-text-verification.json')
OUTPUT = Path('docs/reviews/evidence/2026-09-25-text-body-verification.json')


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


assert entry(PREVIOUS)['sha256'] == '27c41ce7d6171fadc363e8819daa0fdba28d23f6de895985abf1b49c5f3ba88e'
old = read(PREVIOUS)
artifacts = {key: entry(value['path']) for key, value in old['artifacts'].items()}
for key in ['cppWasm', 'cppWasmGlue', 'typescriptAdapter',
            'rasterWasm', 'rasterWasmGlue', 'rasterAdapter']:
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
assert (audit['unchangedBatches'],audit['migratedBatches']) == (9770,0)
assert set(audit['additions']) == {'textDefaults','removedThemeNotice'}
assert audit['additions']['textDefaults'] == audit['additions']['removedThemeNotice']
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
    assert count == info['casesUnchanged'] + info['themeTextProjectionOnly']
    regressions[name] = {'report': entry(info['reportPath']), 'batches': count,
                         'migrationAudit': info}
assert len(regressions) == 45
assert sum(item['batches'] for item in regressions.values()) == 9770

parity = read(ROOT / 'parity.json')
current_artifacts(parity)
bound_files(parity)
assert parity['exactNativeWasmResponsesAndEdits']
assert len(parity['cases']) == 294
assert sum(c['kind']=='source' for c in parity['cases']) == 99
assert sum(c.get('textBodyPreserved',False) for c in parity['cases']) == 96
manifest = read(ROOT / 'manifest.json')
bound_files(manifest)
assert len(manifest['cases']) == 99
assert sum(c['status']=='resolved' for c in manifest['cases']) == 92
assert sum(c['status']=='unresolved' for c in manifest['cases']) == 4
assert sum(c['status']=='error' for c in manifest['cases']) == 3
reference = read(ROOT / 'reference.json')
bound_files(reference)
assert reference['counts'] == {'bodies':92,'propertyValues':1748,'propertyOrigins':1748,
    'autofitChoices':92,'validParts':390,'invalidParts':6,'unchangedCompressedParts':1920}
by_name = {c['name']:c for c in parity['cases'] if c['kind']=='query'}
assert len(reference['cases']) == 96
for c in reference['cases']:
    assert c['responseSha256'] == by_name[c['name']]['responseSha256']
contracts = read(ROOT / 'contracts.log')
assert (contracts['schemas'],contracts['textBodySources'],contracts['textBodyResponses'],
    contracts['textBodyRequests'],contracts['textBodyEditRequests']) == (68,99,195,195,96)
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT / 'workspace-tests.log').read_text(), re.M)
assert len(tests) == 400
logs = ['workspace-tests.log','clippy.log','fmt.log','build-native.log','build-wasm.log',
    'wasm-bindgen.log','schema-check.log','types-check.log','types-write.log','fixtures.log',
    'parity.log','reference.log','regressions.log','regression-audit.log','contracts.log']
logs += [path.name for path in sorted(ROOT.glob('*-regression.log'))]
for name in logs:
    text = (ROOT / name).read_text()
    assert not any(word in text for word in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
changed = [item['path'] for item in old['sourceFiles']
           if item['path'].startswith('contracts/generated/') and entry(item['path']) != item]
assert changed == ['contracts/generated/pptx-source-response.schema.json']
assert len(list(Path('contracts/generated').glob('*.schema.json'))) == 68

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
regressions['textBodies'] = {'report': entry(ROOT / 'parity.json'), 'batches': 294}
result = {
    'format':'musteroffice.text-body-verification/1','previousEvidence':entry(PREVIOUS),
    'checks':{'rustTests':400,'newRustTests':8,'nativeWasmLogicalBatches':10064,
        'previousLogicalBatchesRerun':9770,'newTextBodyBatches':294,'nativeInputPackages':99,
        'nativeEditedPackages':96,'runtimeSchemas':68,'changedExistingSchemas':1,'newSchemas':2,
        'strictClippy':True,'rustfmt':True,'typescript':True},
    'scope':'Native body property inheritance and theme text defaults, with source-preserving edits. '
        'Draft precedence does not certify Office/WPS. Paragraph/run inheritance, font binding, layout, rendering and Musterwork remain incomplete.',
    'artifacts':artifacts,
    'artifactByteDeltas':{k:v['byteLength']-old['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
    'artifactSizeScope':'Uncompressed incomplete development artifacts, not a complete kernel or Musterwork installer. No new product latency/RSS measurements.',
    'environment':environment,'regressionReports':regressions,'migrationAudit':audit,
    'bodyReport':parity,'independentReference':reference,'contracts':contracts,
    'fixtureManifest':entry(ROOT/'manifest.json'),'rustTestNames':tests,
    'sourceFiles':[entry(p) for p in sorted(sources)],'validationLogs':[entry(ROOT/n) for n in logs],
    'changedSchemas':changed,'newSchemas':['contracts/generated/pptx-text-body-query.schema.json',
        'contracts/generated/pptx-text-body-response.schema.json'],
    'standardInputs':[entry(p) for p in sorted(Path('.codex-work/ecma376/xsd').glob('*.xsd'))]
        +[entry('.codex-work/ecma376/OfficeOpenXML-XMLSchema-Transitional.zip')],
    'dependencyChanges':{'externalRuntimeVersions':[],'pnpmLockUnchanged':True,'cargoLockUnchanged':True},
    'limitations':[
        'The explicit draft order is local/layout placeholder, theme txDef/lnDef/spDef, master placeholder, then declared body defaults. Target application precedence is not certified.',
        'The selected autofit choice is inherited as a whole; missing normal-fit fields receive local defaults. This interpretation still needs target-app edge-case testing.',
        'The query computes 19 body properties and the fitting mode; it does not run automatic fitting, shape resizing, paragraph/run inheritance, theme-font selection or FontManifest binding.',
        'Theme shape-property/style siblings remain outside the text-default projection. Unknown body/default content produces unresolved; their source bytes are retained.',
        'The independent XML oracle validates this draft order and physical provenance. Six intentionally invalid native parts are recorded; 390 valid part checks do not certify full packages.',
        'Source page rendering still rejects visible text. No new Office/WPS opening, editing, saving or visual verification was performed.',
        'Native tests use macOS arm64 and WASM uses Node; browser workers, production resource/performance/installer gates, complete advanced features, Agent services and Musterwork E0-E3 remain pending.'
    ]
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
