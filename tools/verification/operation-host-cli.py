"""Exercise the actual release host and inspect its committed database independently."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess

root = Path('.codex-work/operation-host')
database = root/'release-operations.sqlite'
assert not database.exists(), 'refuse to reuse or overwrite a previous run'
binary = Path('target/release/mo-host')
cases = []
def call(name, request=None, arguments=()):
    run = subprocess.run([str(binary), str(database), 'agent:release-fixture', 'scope:release-fixture', *arguments], input='' if request is None else json.dumps(request)+'\n', capture_output=True, text=True, timeout=30)
    assert run.returncode == 0 and not run.stderr, run.stderr
    response = json.loads(run.stdout)
    cases.append(dict(name=name, request=request, arguments=list(arguments), response=response))
    return response
document = json.loads(Path('fixtures/presentations/playback/page.json').read_text())['page']['document']
create = dict(operation='submit', request=dict(contractVersion='musteroffice.operations/1-draft', requestId='create', profileId='presentations-author-model-v01-draft', outputMode='job', action=dict(kind='create', document=document)))
accepted = call('create', create)
assert accepted['outcome'] == 'accepted' and accepted['job']['state'] == 'queued'
completed = call('run-create', arguments=('run', accepted['job']['id']))
assert completed['outcome'] == 'succeeded'
read = dict(operation='readDocument', documentId=document['id'])
original = call('read-original', read)['result']['snapshot']
assert original['document'] == document
operations = [dict(operationId='title', operation=dict(kind='setTitle', title='持久化 Agent 操作验收样本'))]
edit = dict(operation='submit', request=dict(contractVersion='musteroffice.operations/1-draft', requestId='edit', profileId='presentations-author-model-v01-draft', outputMode='sync', action=dict(kind='apply', documentId=document['id'], baseRevision=original['revision'], operations=operations)))
edited = call('edit', edit)
assert edited['outcome'] == 'succeeded'
assert call('retry-edit', edit) == edited
assert call('retry-create', create) == completed
current = call('read-current', read)['result']['snapshot']
expected = dict(document, title=operations[0]['operation']['title'])
assert current['document'] == expected and current['revision'] != original['revision']
assert call('read-historical', dict(read, revision=original['revision']))['result']['snapshot'] == original
assert call('cancel-committed', dict(operation='cancelJob', jobId=edited['result']['job']['id'])) == edited
assert completed['result']['job']['executorDigest'] == hashlib.sha256(binary.read_bytes()).hexdigest()
with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as db:
    assert db.execute('PRAGMA integrity_check').fetchall() == [('ok',)]
    counts = {table:db.execute(f'SELECT count(*) FROM {table}').fetchone()[0] for table in ['jobs','revisions','heads']}
    assert counts == dict(jobs=2, revisions=2, heads=1)
    stored = db.execute('SELECT snapshot FROM revisions ORDER BY revision').fetchall()
    assert {json.loads(row[0])['revision'] for row in stored} == {original['revision'], current['revision']}
    assert all(json.loads(row[0])['state'] == 'succeeded' for row in db.execute('SELECT info FROM jobs'))
def entry(path):
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())
report = dict(format='musteroffice.operation-host-release-cli/1', binary=entry(binary), database=entry(database), counts=counts, processInvocations=len(cases), sqliteInspectorVersion=sqlite3.sqlite_version, cases=cases, limitations=['Self-owned author-model fixture only; no PPTX or resource-byte export, MCP or product integration.'])
(root/'release-cli.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
print(json.dumps(dict(processInvocations=len(cases), counts=counts)))
