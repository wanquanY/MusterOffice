"""Seal delivered-byte admission evidence without replacing earlier records."""
import gzip
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/delivery-receive')
output = Path('docs/reviews/evidence/2026-09-26-delivery-receive-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-operation-client-verification.json')
def entry(path):
    path = Path(path); raw = path.read_bytes()
    return dict(path=str(path), byteLength=len(raw), sha256=hashlib.sha256(raw).hexdigest())
def read(path): return json.loads(Path(path).read_text())
assert entry(parent_path)['sha256'] == 'acc996d6cf6d1a30c1ca615b57f663b6c057515668a4cbea0e07d25156bddc09'
parent = read(parent_path)
prior = {e['path']: e for e in parent['sourceFiles']}
changed = {p for p, e in prior.items() if entry(p) != e}
assert changed == {'Cargo.lock', 'crates/mo-kernel-api/Cargo.toml', 'crates/mo-kernel-api/src/lib.rs',
    'crates/mo-presentation-delivery/Cargo.toml', 'crates/mo-presentation-delivery/src/lib.rs',
    'crates/mo-wasm/src/lib.rs', 'tools/mo-cli/Cargo.toml', 'tools/mo-cli/src/main.rs',
    'tools/mo-contract-codegen/src/main.rs', 'docs/implementation/progress.md'}, changed
sources = set(prior) | {'crates/mo-presentation-delivery/src/receive.rs',
    'crates/mo-kernel-api/src/delivery.rs', 'docs/implementation/delivery-receive.md'}
for directory in ['crates/mo-presentation-delivery/src/receive', 'fixtures/presentations/delivery-receive']:
    sources |= {str(p) for p in Path(directory).rglob('*') if p.is_file()}
sources |= {str(p) for p in Path('tools/verification').glob('delivery-receive-*') if p.is_file()}
sources |= {str(p) for p in Path('contracts/generated').glob('delivery-inspect-*.schema.json')}
sources |= {str(p) for p in Path('packages/contracts/src/generated').glob('delivery-inspect-*.ts')}
links = 0
for name in sorted(sources):
    path = Path(name)
    if path.suffix in ['.rs', '.py', '.ts', '.mjs', '.cpp', '.h']:
        assert len(path.read_text().splitlines()) <= 2000, name
    if path.suffix != '.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', path.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'): continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (path.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

workspace = read(root/'workspace.json')
assert [r['name'] for r in workspace] == ['fmt', 'workspace-tests', 'clippy', 'native', 'schemas', 'types', 'wasm', 'bindgen']
for record in workspace:
    assert record['exitCode'] == 0
    text = Path(record['log']).read_text()
    assert not any(s in text for s in ['error:', 'FAILED', 'Traceback', 'error TS', 'ERR_PNPM']), record
workspace_tests = re.findall(r'^test (.+) \.\.\. ok$', (root/'workspace-tests-1.log').read_text(), re.M)
assert len(workspace_tests) == 855
latest_tests = re.findall(r'^test (.+) \.\.\. ok$', (root/'tests-3.log').read_text(), re.M)
assert len(latest_tests) == len(set(latest_tests)) == 19
assert '19 passed; 0 failed' in (root/'tests-3.log').read_text()
assert not (root/'fmt-final.log').read_bytes()
assert len(list(Path('contracts/generated').glob('*.schema.json'))) == 111
checks = [dict(name=r['name'], exitCode=r['exitCode'], log=entry(r['log'])) for r in workspace]
for name in ['tests-3', 'parity-2', 'reference-2', 'regression', 'fmt-final']:
    log = root/f'{name}.log'; text = log.read_text()
    assert not any(s in text for s in ['FAILED', 'Traceback', 'AssertionError']), name
    checks.append(dict(name=name, exitCode=0, log=entry(log)))

parity_path = root/'parity-2/report.json'; parity = read(parity_path)
assert len(parity['cases']) == 36 and sum(c['expected']=='inspected' for c in parity['cases']) == 3
for key in ['native', 'wasm', 'wasmGlue']:
    assert entry(parity[key]['path']) == parity[key]
for case in parity['cases']:
    for key in ['input', 'bytes', 'response']: assert entry(case[key]['path']) == case[key]
    response = read(case['response']['path'])
    assert response['status'] == ('inspected' if case['expected']=='inspected' else 'error')
    if case['expected'] != 'inspected': assert response['error']['code'] == case['expected']
reference_path = root/'reference-2.json'; reference = read(reference_path)
assert reference['cases'] == reference['responseSchemasChecked'] == 36
assert reference['fixtureAssetsUnchangedFromNativeSdk'] == 12
assert reference['canonicalBundleDigestsChecked'] == 3 and len(reference['independentPngPages']) == 2
assert reference['pptxSha256'] == '49213f1e4cf95b4c60010f210e5e27445edf1e7ecc431573faa0d88cb5f6d7f8'
assert reference['requestSchemaFailures'] == ['unknown-field', 'numeric-length', 'noncanonical-length']
regression_path = root/'regression.json'; regression = read(regression_path)
assert {name:r['cases'] for name,r in regression['reports'].items()} == {'authored':17, 'source':33, 'editor':22}
for name, record in regression['reports'].items():
    assert record['allPreviousResultsIdentical'] and entry(record['report']['path']) == record['report']
    assert not (root/f'regression-{name}.stderr').read_bytes()
for artifact in regression['generatedArtifacts']: assert entry(artifact['path']) == artifact

previous_wasm = Path('.codex-work/native-scheduler/wasm-node/mo_wasm_bg.wasm')
assert entry(previous_wasm)['sha256'] == '7a97e4077b4f45da133fc46abac0f4d2a4df6b53f7ad13948b9a3cff520a0b83'
def size(path):
    return dict(module=entry(path), gzip9Bytes=len(gzip.compress(Path(path).read_bytes(), compresslevel=9, mtime=0)))
old, new = size(previous_wasm), size(parity['wasm']['path'])
assert (new['module']['byteLength'], new['gzip9Bytes']) == (10049547, 2722193)
assert (new['module']['byteLength']-old['module']['byteLength'],new['gzip9Bytes']-old['gzip9Bytes']) == (1285370,296898)
observation = read(root/'musterwork-interface-observation.json')
assert observation['sourceCommit'] == '5750dd6378dc941dc8fc0948d54d4269eb154256' and len(observation['paths']) == 4
report = dict(format='musteroffice.delivery-receive-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(sources-set(prior)), localLinksChecked=links,
    checks=checks, workspaceTests=workspace_tests, finalAffectedLibraryTests=latest_tests,
    workspaceScope='855 workspace tests before the final PNG error classification addition; all 19 affected library tests then passed. Strict all-target Clippy and final builds include that addition.',
    schemaCount=111, actualParity=entry(parity_path), independentChecks=entry(reference_path),
    publicApiRegression=entry(regression_path), sourceObservation=entry(root/'musterwork-interface-observation.json'),
    wasmModuleSize=dict(previous=old, current=new, rawDelta=1285370, gzip9Delta=296898,
        scope='Locked release Rust module only; no JS, draw/shape components, fonts/media, product packaging or benchmark.'),
    artifacts=[entry(p) for p in sorted(root.rglob('*')) if p.is_file()],
    externalGeneratedArtifacts=regression['generatedArtifacts'],
    limitations=[
        'Portable receipt/byte/reference inspection, not a finished Musterwork successor parser or Artifact adapter.',
        'Host must supply accepted pins, trusted execution binding, immutable retained authorized bytes and atomic commit checks.',
        'Reports and producer claims are not transferable authority, renderer attestation or source-to-output semantic/visual equivalence.',
        'PNG reader accepts the current owned writer profile, not arbitrary imported PNG formats.',
        'Native CLI uses a streamed bounded diagnostic bundle; WASM inline input is bounded at 128 MiB. Core supports per-asset readers.',
        'No full XSD, new renderer, Office/WPS, browser owner, product performance/RSS/install-size or complete capability acceptance.',
        'Complete advanced contents, Viewer/Player, Skill/Plugin, product history and Musterwork replacement gates remain open.'])
encoded = json.dumps(report, ensure_ascii=False, indent=2)+'\n'
if '--check' in sys.argv: assert output.read_text() == encoded
elif '--dry-run' not in sys.argv:
    assert not output.exists(), 'never overwrite historical evidence'
    output.write_text(encoded)
print(json.dumps(dict(sourceFiles=len(sources), localLinks=links, workspaceTests=855,
    finalAffectedTests=19, newParityCalls=36, priorParityCalls=72, schemas=111,
    output=str(output), check='--check' in sys.argv)))
