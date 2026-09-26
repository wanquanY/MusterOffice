"""Preserve the prior stdio record and bind versioned response metadata checks."""
import hashlib
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote

root = Path('.codex-work/mcp-response-metadata')
output = Path('docs/reviews/evidence/2026-09-26-mcp-response-metadata-verification.json')
parent_path = Path('docs/reviews/evidence/2026-09-26-mcp-stdio-verification.json')
def entry(path):
    p = Path(path); raw = p.read_bytes()
    return dict(path=str(p), byteLength=len(raw), sha256=hashlib.sha256(raw).hexdigest())
assert entry(parent_path)['sha256'] == '6e8b1e2d79c541e1247b90df41c433544e2b69808c3a5334d9273ea18392db07'
parent = json.loads(parent_path.read_text()); prior = {r['path']:r for r in parent['sourceFiles']}
changed = {p for p, value in prior.items() if entry(p) != value}
assert changed == {'tools/mo-mcp/src/server.rs', 'tools/mo-mcp/check.py',
                   'docs/implementation/mcp-stdio.md', 'docs/implementation/progress.md'}, changed
added = {'docs/implementation/mcp-response-metadata.md', 'tools/verification/mcp-response-metadata-evidence.py'}
sources = set(prior) | added; links = 0
for name in sorted(sources):
    p = Path(name)
    if p.suffix in ['.rs', '.py', '.ts', '.mjs', '.cpp', '.h']: assert len(p.read_text().splitlines()) <= 2000
    if p.suffix != '.md': continue
    for target in re.findall(r'\]\(([^\s)]+)(?:\s+"[^"\n]*")?\)', p.read_text()):
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('#'): continue
        target = unquote(target.split('#')[0].split('?')[0])
        if target:
            destination = (p.parent/target).resolve()
            assert destination.exists() or destination == output.resolve(), (name, target)
            links += 1

reference_path = root/'reference-1/report.json'; reference = json.loads(reference_path.read_text())
assert len(reference['calls']) == 173
assert [(s['era'], s['exportedAssets']) for s in reference['sessions']] == [('legacy',12), ('modern',12)]
binary = entry(root/'frozen/mo-mcp'); assert binary['sha256'] == reference['binary']['sha256']
assert binary['byteLength'] == reference['binary']['byteLength']
assert entry(reference['worker']['path']) == parent['worker']
methods = {'tools/list', 'resources/list', 'resources/templates/list', 'resources/read'}
cache_counts = dict(legacy=0, modern=0); unsupported_prompts = 0
for call in reference['calls']:
    prefix = reference_path.parent/f"{call['session']}-{call['id']:03}"
    request = json.loads(prefix.with_suffix('.request.json').read_text())
    response = json.loads(prefix.with_suffix('.response.json').read_text())
    assert request['id'] == response['id'] == call['id'] and request['method'] == call['method']
    era = call['session'].split('-')[0]
    if call['method'] in methods and 'result' in response:
        result = response['result']
        if era == 'modern':
            assert result['ttlMs'] == 0 and result['cacheScope'] == 'private' and result['resultType'] == 'complete'
        else:
            assert not {'ttlMs', 'cacheScope', 'resultType'} & set(result)
        cache_counts[era] += 1
    if call['method'] == 'prompts/list':
        assert response['error']['code'] == -32601; unsupported_prompts += 1
    if call['method'] == 'server/discover':
        assert response['result']['ttlMs'] >= 0 and response['result']['cacheScope'] in ['public', 'private']
assert all(n > 20 for n in cache_counts.values()) and unsupported_prompts == 8
for p in reference_path.parent.glob('*.stderr'): assert not p.read_bytes()

# Historical raw messages independently demonstrate the original omission.
old_root = Path(parent['reference']['path']).parent
for call in json.loads(Path(parent['reference']['path']).read_text())['calls']:
    if call['session'] == 'modern' and call['method'] in methods-{'resources/read'}:
        result = json.loads((old_root/f"modern-{call['id']:03}.response.json").read_text())['result']
        assert 'ttlMs' not in result and 'cacheScope' not in result
files_path = root/'file-checks.json'; files = json.loads(files_path.read_text())
old_files = json.loads(Path(parent['downloadedFiles']['path']).read_text())
assert files['files'] == old_files['files']
assert sum(len(f['checkedParts']) for f in files['files']) == 20
tests = re.findall(r'^test (.+) \.\.\. ok$', (root/'tests.log').read_text(), re.M)
assert sorted(tests) == sorted(parent['adapterTestNames'])
checks = []
for name in ['tests', 'clippy', 'fmt', 'release-3', 'reference-1', 'file-checks']:
    log = root/f'{name}.log'; text = log.read_text()
    assert not any(bad in text for bad in ['error:', 'FAILED', 'Traceback', 'AssertionError'])
    if name in ['tests', 'clippy', 'release-3']: assert 'Finished' in text
    checks.append(dict(name=name, exitCode=0, log=entry(log)))
report = dict(format='musteroffice.mcp-response-metadata-verification/1', parent=entry(parent_path),
    sourceFiles=[entry(p) for p in sorted(sources)], changedPriorSources=sorted(changed),
    addedSources=sorted(added), sourceLinksChecked=links, checks=checks, adapterTestNames=tests,
    releaseBinary=binary, reference=entry(reference_path), wireRequests=173,
    cacheResponseCounts=cache_counts, unadvertisedPromptsRejected=unsupported_prompts,
    downloadedFiles=entry(files_path), previousFileAndPixelChecksUnchanged=True,
    artifacts=[entry(p) for p in sorted(root.rglob('*')) if p.is_file()],
    specification='https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/caching',
    limitations=['Protocol metadata correction and repeated native stdio/file checks, not complete MCP conformance.',
        'No new full-workspace, Native/WASM, Office/WPS, performance or Musterwork replacement acceptance.',
        'Previous malformed-frame error recovery, HTTP, SDK/Skill/Plugin and complete presentation gaps remain.'])
encoded = json.dumps(report, indent=2, ensure_ascii=False)+'\n'
if '--check' in sys.argv: assert output.read_text() == encoded
elif '--dry-run' not in sys.argv:
    assert not output.exists(), 'never overwrite historical evidence'
    output.write_text(encoded)
print(json.dumps(dict(sourceFiles=len(sources), wireRequests=173, cacheResponses=cache_counts,
    previousFilesUnchanged=True, output=str(output), check='--check' in sys.argv)))
