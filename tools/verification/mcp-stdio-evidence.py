"""Freeze actual MCP development-adapter evidence without claiming E0-6."""
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote

root = Path('.codex-work/mcp-server')
output = Path('docs/reviews/evidence/2026-09-26-mcp-stdio-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-native-scheduler-verification.json')

def entry(path):
    path = Path(path); data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

assert entry(parent_path)['sha256'] == '6b464c30f4d184bfc9261000c9555a556a7c0972321d42a594c5e86ec75c51f1'
parent = json.loads(parent_path.read_text()); prior = {r['path']:r for r in parent['sourceFiles']}
changed = {name for name, record in prior.items() if entry(name) != record}
allowed = {'README.md', 'docs/README.md', 'docs/implementation/progress.md',
           'docs/implementation/dependencies.md', 'docs/implementation/development.md',
           'docs/design/agent-interfaces.md', 'components/rmcp/component.json',
           'crates/mo-standard-host/src/runtime.rs'}
assert changed == allowed, (changed-allowed, allowed-changed)
assert entry('Cargo.lock') == prior['Cargo.lock']
added = {'docs/implementation/mcp-stdio.md', 'tools/verification/mcp-stdio-evidence.py'}
for directory in ['tools/mo-mcp', 'components/base64']:
    added |= {str(p) for p in Path(directory).rglob('*') if p.is_file() and not {'target', '__pycache__'} & set(p.parts)}
assert not added & set(prior)
sources = set(prior) | added
links = 0
for name in sorted(sources):
    p = Path(name)
    if p.suffix in ['.rs', '.py', '.ts', '.mjs', '.cpp', '.h']:
        assert len(p.read_text().splitlines()) <= 2000, name
    if p.suffix != '.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'): continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (p.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

reference_path = root/'reference-4/report.json'
reference = json.loads(reference_path.read_text())
binary = entry(root/'frozen/mo-mcp')
assert binary['sha256'] == reference['binary']['sha256'] and binary['byteLength'] == reference['binary']['byteLength']
assert entry(reference['worker']['path']) == reference['worker']
assert [(s['era'], s['tools'], s['schemas'], s['exportedAssets']) for s in reference['sessions']] == [
    ('legacy', 13, 10, 12), ('modern', 13, 10, 12)]
assert len(reference['calls']) == 167
for record in reference['calls']:
    prefix = reference_path.parent/f"{record['session']}-{record['id']:03}"
    request = json.loads(prefix.with_suffix('.request.json').read_text())
    response = json.loads(prefix.with_suffix('.response.json').read_text())
    assert request['method'] == record['method'] and request['id'] == response['id'] == record['id']
    result = response.get('result', {})
    if 'structuredContent' in result:
        assert json.loads(result['content'][0]['text']) == result['structuredContent']
for p in reference_path.parent.glob('*.stderr'): assert not p.read_bytes(), p

transport_path = root/'transport-1/report.json'
transport = json.loads(transport_path.read_text())
assert len(transport['cases']) == 5 and all(c['exitCode'] != 0 for c in transport['cases'])
assert transport['binaryAuthorization'] and transport['invalidConfigRejected']
files_path = root/'file-checks-1.json'; files = json.loads(files_path.read_text())
assert len(files['files']) == 2
assert sum(len(r['checkedParts']) for r in files['files']) == 20
assert all(r['nativeObjects'] == 15 and r['textRuns'] == 8 and len(r['pages']) == 2 for r in files['files'])
assert files['files'][0]['pptxSha256'] == files['files'][1]['pptxSha256']
assert files['files'][0]['pages'] == files['files'][1]['pages']

checks = []
for name, log in [('adapter-tests','final-tests.log'), ('adapter-clippy','final-clippy.log'),
                  ('adapter-fmt','final-fmt.log'), ('host-tests','host-tests.log'),
                  ('release-build','release-build.log'), ('protocol','reference-4.log'),
                  ('transport','transport-1.log'), ('downloaded-files','file-checks-1.log')]:
    text = (root/log).read_text()
    assert not any(bad in text for bad in ['error:', 'FAILED', 'Traceback', 'AssertionError']), name
    if name not in ['adapter-fmt', 'protocol', 'transport', 'downloaded-files']: assert 'Finished' in text
    checks.append(dict(name=name, exitCode=0, log=entry(root/log)))
names = lambda log: re.findall(r'^test (.+) \.\.\. ok$', (root/log).read_text(), re.M)
adapter_tests, host_tests = names('final-tests.log'), names('host-tests.log')
assert len(adapter_tests) == 10 and len(host_tests) == 61
assert 'bridge::tests::abandoned_waiter_keeps_execution_permit_and_durable_business_work' in adapter_tests
assert 'transport::tests::late_cancel_cannot_retire_response_before_its_write_future_is_polled' in adapter_tests

metadata = json.loads((root/'metadata.json').read_text())
registry = sorted([dict(name=p['name'], version=p['version'], declaredLicense=p['license'], source=p['source'])
                   for p in metadata['packages'] if p['source']], key=lambda p:(p['name'], p['version']))
assert len(registry) == 106
lock = tomllib.loads(Path('tools/mo-mcp/Cargo.lock').read_text())
sdk = next(p for p in lock['package'] if p['name'] == 'rmcp')
sdk_component = json.loads(Path('components/rmcp/component.json').read_text())
assert sdk['version'] == sdk_component['version'] == '3.4.0'
assert sdk['checksum'] == sdk_component['archive']['sha256']
base64 = next(p for p in lock['package'] if p['name'] == 'base64')
component = json.loads(Path('components/base64/component.json').read_text())
assert base64['checksum'] == component['archiveSha256'] and base64['version'] == component['version']
for license in component['licenses']:
    assert entry(Path('components/base64')/license['path'])['sha256'] == license['sha256']
assert 'release' in (root/'release-build.log').read_text()
assert 'host: aarch64-apple-darwin' in (root/'rustc.txt').read_text()
assert 'rustc 1.92.0' in (root/'rustc.txt').read_text()

artifacts = [entry(p) for p in sorted(root.rglob('*')) if p.is_file()]
report = dict(format='musteroffice.mcp-stdio-verification/1', parent=entry(parent_path),
    scope='Native development stdio adapter over the existing durable host; incomplete protocol/product acceptance.',
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), sourceLinksChecked=links, checks=checks,
    adapterTestNames=adapter_tests, hostTestNames=host_tests, releaseBinary=binary,
    worker=reference['worker'], reference=entry(reference_path), wireRequests=167,
    protocolProfiles=['2025-11-25', '2026-07-28'], toolsPerProfile=13, schemaPairs=20,
    publicAssetsRead=24, transport=entry(transport_path), downloadedFiles=entry(files_path),
    dependencies=registry, dependencyCountScope='Resolved registry graph including target-conditional branches, not a linked-package count or a license audit.',
    artifacts=artifacts, limitations=[
        '10 adapter tests and 61 affected host tests, not a new full-workspace regression or Native/WASM run.',
        'Computational source, generated contracts, root lock and historical worker are unchanged; previous evidence is not relabeled as new testing.',
        'Malformed/ambiguous frames currently close the connection; standard protocol error recovery and the complete SDK cancellation/failure matrix remain open.',
        'No HTTP, Tasks, SDK/Skill/Plugin distribution, browser owner, Musterwork adaptation or replacement acceptance.',
        'Synthetic owned font and generated-subset files; no complete advanced content or Office/WPS visual/edit/playback acceptance.',
        'Native release executable size excludes the external worker, fonts/media and desktop packaging; no new latency/RSS/product benchmark.',
        'Dependency metadata and selected original notices do not constitute full license/distribution clearance.',
    ])
encoded = json.dumps(report, indent=2, ensure_ascii=False)+'\n'
if '--check' in sys.argv:
    assert output.read_text() == encoded, 'frozen evidence differs from current source/artifacts'
elif '--dry-run' not in sys.argv:
    assert not output.exists(), 'never overwrite historical evidence'
    output.write_text(encoded)
print(json.dumps(dict(sourceFiles=len(sources), artifacts=len(artifacts), wireRequests=167,
    adapterTests=10, hostTests=61, publicAssets=24, output=str(output), check='--check' in sys.argv)))
