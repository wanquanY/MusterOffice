"""Real SDK-backed compact creation and revisioned edits in both MCP eras."""
import copy
import json
from pathlib import Path
import sys

from compute_support import Files, Session, read, sha, write

root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve()
fixture = read('fixtures/presentations/compose/invocation.json')
observations = []
for era in ('2025-11-25', '2026-07-28'):
    files = Files(root / era)
    with Session(binary, files, era, 'protocol') as client:
        caps = client.tool('mo_capabilities', {})
        assert 'compose' in caps['operations'] and not caps['businessJobs']
        created = client.computed(files.invocation('compose', fixture))['result']['snapshot']
        assert len(created['document']['slides']) == 3
        title = created['document']['objects']['object:title']['content']['text']['paragraphs'][0]
        action = dict(kind='apply', documentId=created['document']['id'], baseRevision=created['revision'],
                      operations=[dict(operationId='operation:edit-title', operation=dict(
                          kind='spliceText', object='object:title', paragraph=title['id'], run=title['runs'][0]['id'],
                          start=0, delete=len(title['runs'][0]['content']['text']), insert='MCP edited title'))])
        edit = dict(request=dict(contractVersion='musteroffice.computation/1-draft', requestId='request:edit',
                                profileId='presentations-author-model-v01-draft', action=action), snapshot=created)
        edited = client.computed(files.invocation('edit', edit))['result']['snapshot']
        assert edited['revision'] != created['revision']
        assert edited['document']['objects']['object:title']['content']['text']['paragraphs'][0]['runs'][0]['content']['text'] == 'MCP edited title'
        assert edited['document']['objects']['object:content'] == created['document']['objects']['object:content']
        stale = copy.deepcopy(edit); stale['snapshot'] = edited
        denied = client.tool('mo_presentations_compute', files.invocation('stale', stale))
        assert denied['outcome'] == 'failed' and denied['error']['code'] == 'REVISION_CONFLICT'
        assert not (files.outputs / 'stale').exists()
        duplicate = copy.deepcopy(fixture)
        duplicate['request']['action']['presentation']['slides'].append(copy.deepcopy(duplicate['request']['action']['presentation']['slides'][0]))
        denied = client.tool('mo_presentations_compute', files.invocation('duplicate', duplicate))
        assert denied['outcome'] == 'failed' and denied['error']['code'] == 'INPUT_INVALID'
        assert not (files.outputs / 'duplicate').exists()
        files.clean()
        observations.append(dict(protocol=era, computations=4, created=created['semanticDigest'], edited=edited['semanticDigest']))
assert observations[0]['created'] == observations[1]['created']
assert observations[0]['edited'] == observations[1]['edited']
report = dict(format='musteroffice.compose-mcp/1', status='passed', binarySha256=sha(binary.read_bytes()), observations=observations)
write(root / 'report.json', report)
print(json.dumps(report))
