"""Bind fill declarations, actual edits, artifacts and prior behavior.

Only the declared source-index additions and the replacement of opaque
gradient/pattern line records are normalized when comparing old evidence.
No query, edit candidate, frame or error code is excused from comparison.
One duplicate-color diagnostic wording change is checked explicitly against
both its current full response and its sealed prior response digest.
"""
import copy
import hashlib
import json
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path('.codex-work/source-fills')
OUTPUT = Path('docs/reviews/evidence/2026-09-24-source-fill-verification.json')
def read(p): return json.loads(Path(p).read_text())
def sha(b): return hashlib.sha256(b).hexdigest()
def entry(p):
    p = Path(p); b = p.read_bytes()
    return {'path':str(p), 'byteLength':len(b), 'sha256':sha(b)}
previous = Path('docs/reviews/evidence/2026-09-24-native-path-verification.json')
assert entry(previous)['sha256'] == '647e56df8d91cd5490daa4b938aafbba4308300cfac04f51f1b7810e8219e662'
prior = read(previous)
artifacts = {k:entry(v['path']) for k,v in prior['artifacts'].items()}
for k in ['cppWasm','cppWasmGlue','typescriptAdapter','rasterWasm','rasterWasmGlue','rasterAdapter']:
    assert artifacts[k] == prior['artifacts'][k], k
def files(value):
    if isinstance(value, list):
        for c in value: files(c)
    elif isinstance(value, dict):
        if {'path','sha256'} <= value.keys():
            actual = entry(value['path']); assert actual['sha256'] == value['sha256']
            if 'byteLength' in value: assert actual['byteLength'] == value['byteLength']
        for key,path in value.items():
            if key.endswith('Path') and key[:-4]+'Sha256' in value: assert entry(path)['sha256'] == value[key[:-4]+'Sha256'], path
            if isinstance(path, (dict,list)): files(path)
def artifact_hashes(report, raster=False):
    aliases = {'nativeSha256':'nativeCli', 'nativeCliSha256':'nativeCli', 'nativeWorkerSha256':'nativeRasterWorker' if raster else 'nativeWorker', 'wasmSha256':'rustWasm', 'rustWasmSha256':'rustWasm', 'wasmKernelSha256':'rustWasm', 'componentSha256':'rasterWasm' if raster else 'cppWasm', 'componentWasmSha256':'cppWasm', 'adapterSha256':'rasterAdapter'}
    assert any(k in report for k in ['nativeSha256','nativeCliSha256'])
    assert any(k in report for k in ['wasmSha256','rustWasmSha256','wasmKernelSha256'])
    for key,artifact in aliases.items():
        if key in report: assert report[key] == artifacts[artifact]['sha256'], key
    files(report['cases'] if 'cases' in report else report['batches'])
def previous_line(line):
    if not line or not line.get('fill'): return
    f = line['fill']
    if f['kind'] in ['gradient','pattern']:
        line['fill'] = {'kind':'retained', 'sourceOrdinal':f['sourceOrdinal'], 'nativeKind':f['kind']}
def previous_index(response):
    r = copy.deepcopy(response); index = r.get('index', r)
    for surface in index.get('surfaces', {}).values():
        surface.pop('background', None); surface.pop('rootGroupFill', None)
        for obj in surface['objects']:
            for field in ['fill','fillReference','pictureFill']: obj.pop(field, None)
            previous_line(obj.get('line'))
    for theme in index.get('themes', {}).values():
        scheme = theme.get('formatScheme')
        if not scheme: continue
        for field in ['fills','lines','effects','backgroundFills']:
            for style in scheme[field]: style.pop('fill', None); previous_line(style.get('line'))
    return r
def compare(current, old, report_name):
    assert len(current) == len(old)
    changed = 0; diagnostics = 0
    for a,b in zip(current, old):
        if a == b: continue
        assert a.keys() == b.keys(), a.get('name')
        normalized = copy.deepcopy(a)
        if 'response' in normalized: normalized['response'] = previous_index(normalized['response'])
        if 'responseSha256' in normalized and normalized['responseSha256'] != b['responseSha256']:
            value = previous_index(read(a['responsePath']))
            diagnostic = (report_name, a['name']) in {('sourceLines', 'inspect-reject-color-duplicate'), ('lineStyles', 'declaration-reject-color-duplicate')}
            if diagnostic:
                assert value == {'status':'error', 'error':{'code':'INPUT_INVALID', 'message':'invalid XML: duplicate or out of order fill property'}}
                value['error']['message'] = 'invalid XML: multiple line colors'
                diagnostics += 1
            raw = json.dumps(value, ensure_ascii=False, separators=(',',':')).encode()
            assert sha(raw) == b['responseSha256'], (a['name'], sha(raw), b['responseSha256'])
            normalized['responseSha256'] = b['responseSha256']
        assert normalized == b, a.get('name')
        changed += 1
    return changed - diagnostics, diagnostics
regressions = {}
for name,record in prior['regressionReports'].items():
    snapshot = ROOT/'previous'/(name+'.json'); assert entry(snapshot)['sha256'] == record['report']['sha256']
    path = ROOT/(name+'-regression.json') if name in ['document','opc','source','color','font','export'] else Path(record['report']['path'])
    report = read(path); old = read(snapshot); field = 'batches' if name == 'bidiConformance' else 'cases'
    additions, diagnostics = compare(report[field], old[field], name)
    artifact_hashes(report, name in ['placement','groups','pathRaster','sceneRaster','pageRaster'])
    if name == 'source':
        for c in report['cases']:
            if 'source' in c: assert entry(c['source'])['sha256'] == c['sourceSha256']
        for c in report['outputs']: assert entry(c['output'])['sha256'] == c['sha256']
    regressions[name] = {'report':entry(path), 'batches':len(report[field]), 'previousSemanticsAndArtifactsUnchanged':True, 'sourceFillRepresentationChangesOnly':additions, 'duplicateColorDiagnosticWordingChanges':diagnostics}
assert sum(r['batches'] for r in regressions.values()) == 6236
assert sum(r['duplicateColorDiagnosticWordingChanges'] for r in regressions.values()) == 2
parity = read(ROOT/'parity.json'); independent = read(ROOT/'independent.json'); manifest = read(ROOT/'manifest.json')
artifact_hashes(parity); assert parity['exactIndexAndCandidateBytes']
assert len(parity['cases']) == 287 and len(manifest['cases']) == 162 and len(independent['cases']) == 125
assert [independent[k] for k in ['fillDeclarations','validXsdParts','negativeXsdCases','semanticRangeRejections']] == [3639,999,36,1]
lookup = {c['name']:c for c in parity['cases']}
for c in independent['cases']: assert c['sourceSha256'] == lookup['inspect-'+c['name']]['sourceSha256']
for c in manifest['cases']: assert entry(c['path'])['sha256'] == c['sha256']
files(independent['xsdInputs'])
contracts = read(ROOT/'contracts.json')
assert [contracts[k] for k in ['schemas','sourceFillResponses','sourceFillEditRequests']] == [57,287,125]
assert [contracts[k] for k in ['sourceLineResponses','sourceGeometryResponses','sourceResponses','sourceRequests']] == [136,823,73,95]
line_reference = read('.codex-work/source-lines/independent.json')
assert [line_reference[k] for k in ['lineDeclarations','lineReferences','validXsdParts','negativeXsdCases']] == [1175,10,471,18]
files(line_reference['xsdInputs'])
tests = re.findall(r'^test (.+) \.\.\. ok$', (ROOT/'workspace-tests.log').read_text(), re.M); assert len(tests) == 313
logs = ['workspace-tests.log','clippy.log','fmt.log','schema-check.log','types-check.log','parity.log','independent.log','line-independent.log','regressions.log','native-build.log','wasm-build.log','bindgen.log']
for file in logs:
    raw = (ROOT/file).read_text(); assert not any(s in raw for s in ['error:','FAILED','Traceback']), file
changed_contracts = []
for prefix,suffix in [('contracts/generated/','.schema.json'),('packages/contracts/src/generated/','.ts')]:
    old = [s for s in prior['sourceFiles'] if s['path'].startswith(prefix) and s['path'].endswith(suffix)]; assert len(old) == 57
    for item in old:
        if entry(item['path']) != item: changed_contracts.append(item['path'])
assert set(changed_contracts) == {'contracts/generated/pptx-source-response.schema.json','packages/contracts/src/generated/pptx-source-response.ts'}
for lock in ['Cargo.lock','pnpm-lock.yaml']: assert entry(lock) == next(s for s in prior['sourceFiles'] if s['path'] == lock)
for reference in prior['reusedGeometryReferences'].values(): files(reference)
component = read('docs/reviews/evidence/2026-09-24-miter-clip-verification.json')
for build in component['componentBuilds'].values():
    assert build['lock'] == read('components/skia/lock.json')
    for record in build['componentSources']+build['artifacts']: assert entry(record['path']) == record
assert read('.codex-work/skia/verification/parity.json') == component['componentReport']
sources = {s['path'] for s in prior['sourceFiles']}
for base in ['components','crates','tools','contracts','packages','fixtures','docs/implementation']:
    for p in Path(base).rglob('*'):
        if p.is_file() and p.suffix in ['.rs','.toml','.json','.ts','.mjs','.py','.md','.h','.cpp','.bin','.ttf','.otf','.ttc','.patch','.txt'] and '__pycache__' not in p.parts: sources.add(str(p))
for p in sources:
    if Path(p).suffix in ['.rs','.ts','.mjs','.py','.cpp','.h']: assert len(Path(p).read_text().splitlines()) <= 2000, p
checks = {'rustTests':313, 'newRustTests':11, 'nativeWasmLogicalBatches':6523, 'previousLogicalBatchesRerun':6236, 'newFillBatches':287, 'ownedPptxInputs':162, 'editedPptxCandidates':125, 'fillDeclarations':3639, 'validXsdParts':999, 'negativeXsdCases':36, 'semanticRangeRejections':1, 'runtimeSchemas':57, 'unchangedSchemas':56, 'strictClippy':True, 'rustfmt':True, 'typescript':True}
regressions['sourceFills'] = {'report':entry(ROOT/'parity.json'), 'batches':287}
result = {'format':'musteroffice.source-fill-verification/1', 'previousEvidence':entry(previous), 'checks':checks,
    'scope':'Shared native fill declarations across shapes, groups, pictures, lines, themes and backgrounds; source binding and real text-edit preservation. No resolved brushes or imported-page rendering.',
    'artifacts':artifacts, 'artifactByteDeltas':{k:v['byteLength']-prior['artifacts'][k]['byteLength'] for k,v in artifacts.items()},
    'environment':{'platform':platform.system()+' '+platform.release(), 'architecture':platform.machine(), 'python':platform.python_version(), 'rust':subprocess.check_output(['rustc','--version'],text=True).strip(), 'node':subprocess.check_output(['node','--version'],text=True).strip()},
    'artifactSizeScope':'Uncompressed incomplete development binaries; not a full kernel, installer, latency or memory acceptance.',
    'regressionReports':regressions, 'fillReport':parity, 'independentReport':independent, 'lineDeclarationReference':entry('.codex-work/source-lines/independent.json'), 'manifest':manifest, 'contractReport':contracts, 'rustTestNames':tests,
    'validationLogs':[entry(ROOT/p) for p in logs], 'sourceFiles':[entry(p) for p in sorted(sources)],
    'reusedGeometryReferences':prior['reusedGeometryReferences'], 'reusedComponentEvidence':{**prior['reusedComponentEvidence'],'rerun':False}, 'dependencyChanges':{'externalRuntimeVersions':[], 'cargoAndPnpmLocksUnchanged':True},
    'limitations':['Fill inheritance, placeholder/group/background selection, image relationship decoding and brush construction remain incomplete.',
                   'Blip effects, background effects and unsupported attributes retain physical bindings; reading is not effect evaluation.',
                   'This development source response adds fill fields and replaces opaque gradient/pattern line records. One duplicate-color error message changes to the shared fill grammar message; its INPUT_INVALID code stays unchanged and both response digests are checked.',
                   'No new Office/WPS/LibreOffice observation, full source-page painting or application edit roundtrip.',
                   'Full text/image/effect import, preset expansion, advanced objects, playback, Agent/Musterwork E0-E3 and complete performance/installer acceptance remain incomplete.']}
raw = json.dumps(result, ensure_ascii=False, indent=2)+'\n'; assert '/Users/' not in raw
summary = {'checks':checks, 'sourceFiles':len(sources), 'artifactByteDeltas':result['artifactByteDeltas']}
if '--seal' in sys.argv:
    with OUTPUT.open('x') as f: f.write(raw)
    summary['evidence'] = entry(OUTPUT)
print(json.dumps(summary, indent=2))
