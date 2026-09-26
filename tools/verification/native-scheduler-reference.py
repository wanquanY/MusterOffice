"""Real persistent CLI scheduling, current exports, old v4 assets and schemas."""
from contextlib import closing
import hashlib
import json
from pathlib import Path
import selectors
import shutil
import sqlite3
import subprocess
import sys
import time
import jsonschema

root = Path(sys.argv[1]); root.mkdir()
stage = Path('.codex-work/native-scheduler')
cli = Path('target/release/mo-host')
worker = Path('target/release/mo-raster-worker')
parent = json.loads(Path('docs/reviews/evidence/2026-09-26-operation-discovery-verification.json').read_text())

def entry(path):
    p = Path(path); data = p.read_bytes()
    return dict(path=str(p), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

cli_id, worker_id = entry(cli), entry(worker)
# Reuse the actual file/resource reference with only the newly expected schema
# version changed. Its original source and every original artifact stay intact.
original_driver = Path('tools/verification/export-host-reference.py')
assert entry(original_driver) == next(r for r in parent['sourceFiles'] if r['path'] == str(original_driver))
source = original_driver.read_text()
assert source.count('== 4\n') == 2
replay = root/'export-v5-reference.py'
replay.write_text(source.replace('== 4\n', '== 5\n'))
with (root/'export.log').open('x') as log:
    run = subprocess.run([sys.executable, str(replay), str(root/'export')], stdout=log, stderr=subprocess.STDOUT)
assert run.returncode == 0
export = json.loads((root/'export/reference.json').read_text())
schemas = {kind: jsonschema.Draft202012Validator(json.loads(Path(f'contracts/generated/{kind}.schema.json').read_text()))
           for kind in ['host-request', 'host-response']}
calls, sessions = [], []

def record(payload, output, mode):
    index = len(calls); paths = {}
    for kind, data in [('stdin', payload), ('stdout', output)]:
        path = root/f'call-{index:03}.{kind}'; path.write_bytes(data); paths[kind] = entry(path)
    calls.append(dict(mode=mode, **paths))
    response = json.loads(output); schemas['host-response'].validate(response)
    return response

def one(query, database):
    schemas['host-request'].validate(query)
    payload = (json.dumps(query)+'\n').encode()
    run = subprocess.run([str(cli), str(database), 'verifier', 'fixture'], input=payload, capture_output=True, timeout=30)
    assert run.returncode == 0 and not run.stderr, run.stderr
    return record(payload, run.stdout, 'single')

class Session:
    def __init__(self, database):
        self.index = len(sessions)
        self.error = root/f'session-{self.index}.stderr'
        self.log = self.error.open('xb')
        self.argv = [str(cli), str(database), 'verifier', 'fixture', '--scheduled', '2',
                     '--preview-worker', str(worker.resolve()), worker_id['sha256']]
        self.process = subprocess.Popen(self.argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log)
        self.selector = selectors.DefaultSelector(); self.selector.register(self.process.stdout, selectors.EVENT_READ)
    def request(self, query):
        schemas['host-request'].validate(query)
        payload = (json.dumps(query)+'\n').encode()
        self.process.stdin.write(payload); self.process.stdin.flush()
        assert self.selector.select(30), 'persistent control response timed out'
        response = self.process.stdout.readline()
        assert response, self.error.read_text()
        return record(payload, response, f'persistent-{self.index}')
    def __enter__(self): return self
    def __exit__(self, kind, *_):
        self.process.stdin.close()
        try:
            if kind: self.process.kill()
            code = self.process.wait(timeout=60)
        except subprocess.TimeoutExpired:
            self.process.kill(); self.process.wait(); raise
        finally:
            self.selector.close(); self.process.stdout.close(); self.log.close()
        sessions.append(dict(argv=self.argv, exitCode=code, stderr=entry(self.error)))
        if not kind: assert code == 0 and not self.error.read_bytes()

def job(response):
    return response['result']['job'] if response['outcome'] == 'succeeded' else response['job']

def wait(session, identifier):
    deadline = time.monotonic()+30
    while True:
        info = job(session.request(dict(operation='getJob', jobId=identifier)))
        if info['state'] in ['succeeded', 'failed', 'cancelled']: return info
        assert time.monotonic() < deadline
        time.sleep(0.02)

database = root/'scheduled.sqlite'
shutil.copy2(root/'export/host.sqlite', database)
queries = [json.loads(Path(c['stdin']['path']).read_text()) for c in export['calls'] if 'stdin' in c and not c['arguments']]
query = next(q for q in queries if q.get('request', {}).get('requestId') == 'export')
old_completed = job(one(dict(operation='getJob', jobId=job(one(query, database))['id']), database))
prestart = json.loads(json.dumps(query)); prestart['request']['requestId'] = 'before-scheduler-start'
queued = job(one(prestart, database)); assert queued['state'] == 'queued'
with Session(database) as session:
    caps = session.request(dict(operation='capabilities'))['result']['capabilities']
    assert caps['queuedExecution'] == 'hostScheduled'
    assert caps['executorDigest'] == cli_id['sha256']
    assert caps['renderer']['implementationSha256'] == worker_id['sha256']
    assert not caps['fullPresentationAcceptance']
    recovered = wait(session, queued['id'])
    assert recovered['state'] == 'succeeded' and recovered['result'] == old_completed['result']
    accepted_ids = []
    for mode in ['job', 'sync', 'auto']:
        q = json.loads(json.dumps(query)); q['request']['requestId'] = f'scheduled-{mode}'; q['request']['outputMode'] = mode
        accepted_ids.append(job(session.request(q))['id'])
    for identifier in accepted_ids:
        completed = wait(session, identifier)
        assert completed['state'] == 'succeeded' and completed['result'] == old_completed['result']
    assert job(session.request(query)) == old_completed
    documents = []
    for identifier in caps['schemas']:
        document = session.request(dict(operation='getSchema', id=identifier))['result']['document']
        assert document['schema'] == json.loads(Path(f'contracts/generated/{identifier}.schema.json').read_text())
        documents.append(document)
    (root/'schemas.json').write_text(json.dumps(documents, indent=2)+'\n')
with Session(database) as session:
    for identifier in accepted_ids:
        assert job(session.request(dict(operation='getJob', jobId=identifier)))['state'] == 'succeeded'
with closing(sqlite3.connect(database)) as db:
    assert db.execute('SELECT count(*) FROM result_assets').fetchone()[0] == 12
    assert db.execute('SELECT count(*) FROM revisions').fetchone()[0] == 1
    assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    assert not db.execute('PRAGMA foreign_key_check').fetchall()

def rows(path):
    with closing(sqlite3.connect(path)) as db:
        names = [r[0] for r in db.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
        return {n: db.execute('SELECT * FROM "'+n+'" ORDER BY rowid').fetchall() for n in names}

original = Path('.codex-work/export-host/reference-4/host.sqlite')
original_id = entry(original)
export_parent_path = Path('docs/reviews/evidence/2026-09-26-export-host-verification.json')
assert entry(export_parent_path)['sha256'] == '913785c32342065aa09315263061dd74f781e1489517408790aedf0a5c50b181'
export_parent = json.loads(export_parent_path.read_text())
assert original_id == next(r for r in export_parent['artifacts'] if r['path'] == str(original))
copy = root/'old-v4.sqlite'; shutil.copy2(original, copy)
before = rows(copy)
with closing(sqlite3.connect(copy)) as db:
    assert db.execute('PRAGMA user_version').fetchone()[0] == 4
    old_job = json.loads(db.execute("SELECT info FROM jobs WHERE operation='presentations.export' AND json_extract(info,'$.state')='succeeded'").fetchone()[0])
assert job(one(dict(operation='getJob', jobId=old_job['id']), copy)) == old_job
assets = old_job['result']['receipt']['bundle']['assets']
old_reads = []
for asset in assets:
    response = one(dict(operation='readAsset', assetId=asset['id']), copy)
    assert response['result']['asset']['descriptor'] == {k: asset[k] for k in ['sha256', 'byteLength', 'mediaType']}
    run = subprocess.run([str(cli), str(copy), 'verifier', 'fixture', 'read-asset', asset['id'], '0', asset['byteLength']], capture_output=True, timeout=30)
    assert run.returncode == 0 and not run.stderr
    assert len(run.stdout) == int(asset['byteLength']) and hashlib.sha256(run.stdout).hexdigest() == asset['sha256']
    path = root/f'old-asset-{len(old_reads):02}.bin'; path.write_bytes(run.stdout)
    old_reads.append(dict(asset=asset, bytes=entry(path)))
assert rows(copy) == before and entry(original) == original_id
with closing(sqlite3.connect(copy)) as db:
    assert db.execute('PRAGMA user_version').fetchone()[0] == 5
    assert db.execute("SELECT count(*) FROM sqlite_schema WHERE name IN ('jobs_queued','jobs_expiring')").fetchone()[0] == 2

# Use the frozen parity program against the actual new module and new schemas.
program = Path('.codex-work/operation-discovery/reference-1/schema-parity.mjs')
assert entry(program) == next(r for r in parent['artifacts'] if r['path'] == str(program))
with (root/'schema-parity.log').open('x') as log:
    run = subprocess.run(['node', str(program), str((stage/'wasm-node/mo_wasm.js').resolve()), str(root/'schemas.json')], stdout=log, stderr=subprocess.STDOUT)
assert run.returncode == 0
assert entry(cli) == cli_id and entry(worker) == worker_id
report = dict(format='musteroffice.native-scheduler-reference/1', cli=cli_id, worker=worker_id,
    export=entry(root/'export/reference.json'), exportReplay=entry(replay), calls=calls, sessions=sessions,
    automaticExports=4, modes=['job','sync','auto'], priorQueueExecuted=True, completedJobsUnchanged=True,
    currentDatabase=entry(database), oldDatabase=original_id, migrated=entry(copy), oldAssets=len(assets), oldAssetReads=old_reads, allOldRowsUnchanged=True,
    schemas=entry(root/'schemas.json'), schemaPairs=len(documents), schemaProgram=entry(program), schemaLog=entry(root/'schema-parity.log'),
    limitation='Native scheduler with owned supported-content export. Full PPT, MCP, browser owner, Office/WPS and Musterwork acceptance remain open.')
(root/'reference.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(calls=len(calls), automaticExports=4, migrations=3, schemaPairs=len(documents))))
