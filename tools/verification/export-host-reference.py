"""Actual release CLI export, persisted public bytes and legacy database migration."""
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import jsonschema
from contextlib import closing

root = Path(sys.argv[1])
root.mkdir()
cli = Path('target/release/mo-host')
worker = Path('target/release/mo-raster-worker')
database = root/'host.sqlite'
calls = []
validated_requests = 0
validated_responses = 0
schemas = {}
for kind in ['host-request', 'host-response']:
    schema = json.loads(Path(f'contracts/generated/{kind}.schema.json').read_text())
    jsonschema.Draft202012Validator.check_schema(schema)
    schemas[kind] = jsonschema.Draft202012Validator(schema)


def entry(path):
    p = Path(path)
    data = p.read_bytes()
    return dict(path=str(p), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


identity = entry(worker)
cli_identity = entry(cli)


def invoke(arguments=(), payload=b'', *, db=database, principal='verifier', scope='fixture', binary=False, renderer=False):
    global validated_requests, validated_responses
    if not arguments and payload:
        schemas['host-request'].validate(json.loads(payload))
        validated_requests += 1
    prefix = [str(cli), str(db), principal, scope]
    if renderer:
        prefix += ['--preview-worker', str(worker.resolve()), identity['sha256']]
    process = subprocess.run(prefix+list(arguments), input=payload, capture_output=True, timeout=120)
    index = len(calls)
    stdout = root/f'call-{index:03}.stdout'
    stderr = root/f'call-{index:03}.stderr'
    stdout.write_bytes(process.stdout)
    stderr.write_bytes(process.stderr)
    record = dict(arguments=list(arguments), database=str(db), principal=principal, scope=scope,
                      renderer=renderer, requestSha256=hashlib.sha256(payload).hexdigest(), exitCode=process.returncode,
                      stdout=entry(stdout), stderr=entry(stderr))
    if payload:
        stdin = root/f'call-{index:03}.stdin'
        stdin.write_bytes(payload)
        record['stdin'] = entry(stdin)
    calls.append(record)
    assert process.returncode == 0 and not process.stderr, process.stderr.decode(errors='replace')
    if binary:
        return process.stdout
    response = json.loads(process.stdout)
    schemas['host-response'].validate(response)
    validated_responses += 1
    return response


def request(body, **kwargs):
    return invoke(payload=(json.dumps(body)+'\n').encode(), **kwargs)


def submit(action, name, profile='presentations-author-model-v01-draft', mode='sync'):
    return dict(operation='submit', request=dict(contractVersion='musteroffice.operations/1-draft',
        requestId=name, profileId=profile, outputMode=mode, action=action))


def upload(name, path, mime):
    data = Path(path).read_bytes()
    response = request(dict(operation='beginUpload', request=dict(requestId=name,
        descriptor=dict(sha256=hashlib.sha256(data).hexdigest(), byteLength=str(len(data)), mediaType=mime))))
    assert response['outcome'] == 'succeeded'
    identifier = response['result']['upload']['id']
    response = invoke(['append', identifier, '0'], data)
    assert response['outcome'] == 'succeeded'
    response = request(dict(operation='sealUpload', uploadId=identifier))
    assert response['outcome'] == 'succeeded'
    return response['result']['upload']['asset']['id']


fixture = json.loads(Path('fixtures/presentations/delivery/input.json').read_text())
created = request(submit(dict(kind='create', document=fixture['document']), 'create'))
assert created['outcome'] == 'succeeded'
initial = created['result']['job']['result']['receipt']
image = upload('image', 'fixtures/presentations/native-export/resources.bin', 'image/png')
font = upload('font', 'fixtures/fonts/owned.ttf', 'application/octet-stream')
query = submit(dict(kind='export', documentId=initial['documentId'], baseRevision=initial['revision'],
    settings=dict(delivery=fixture['settings'], resources=[dict(resourceId='resource:checker', assetId=image)],
        fontAssetId=font, renderer=dict(implementationSha256=identity['sha256'], profile='drawingml-resource-page-q32-v1-draft'))),
    'export', 'presentations-pptx-resource-delivery-v1-draft', 'job')
accepted = request(query)
assert accepted['outcome'] == 'accepted' and accepted['job']['state'] == 'queued'
assert accepted['job']['result'] is None
with closing(sqlite3.connect(database)) as db:
    assert db.execute('SELECT count(*) FROM result_assets').fetchone()[0] == 0
finished = invoke(['run', accepted['job']['id']], renderer=True)
assert finished['outcome'] == 'succeeded'
job = finished['result']['job']
receipt = job['result']['receipt']
assert receipt['documentId'] == initial['documentId'] and receipt['revision'] == initial['revision']
assert receipt['semanticDigest'] == initial['semanticDigest']
bundle = receipt['bundle']
assert len(bundle['assets']) == 12 and len(bundle['previews']) == 2
candidate = root/'candidate'
candidate.mkdir()
files = []
with closing(sqlite3.connect(database)) as db:
    for i, asset in enumerate(bundle['assets']):
        raw = invoke(['read-asset', asset['id'], '0', asset['byteLength']], binary=True)
        assert hashlib.sha256(raw).hexdigest() == asset['sha256'] and len(raw) == int(asset['byteLength'])
        ext = '.json' if asset['mediaType'].endswith('json') else '.png' if asset['mediaType'] == 'image/png' else '.pptx' if asset['role'] == 'pptx' else '.bin'
        file = f'{i:03}{ext}'
        (candidate/file).write_bytes(raw)
        row = db.execute('SELECT principal,job_id,fence,name,info FROM result_assets WHERE scope=? AND id=?', ('fixture',asset['id'])).fetchone()
        principal, identifier, fence, name, stored_info = row
        info = json.loads(stored_info)
        assert info['descriptor'] == {k:asset[k] for k in ['sha256','byteLength','mediaType']}
        chunks = db.execute('SELECT data FROM result_chunks WHERE scope=? AND principal=? AND job_id=? AND fence=? AND name=? ORDER BY chunk_index',('fixture',principal,identifier,fence,name)).fetchall()
        assert b''.join(bytes(c[0]) for c in chunks) == raw
        files.append(dict(name=name, asset=asset, file=file))
    assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    assert not db.execute('PRAGMA foreign_key_check').fetchall()
    assert db.execute('PRAGMA user_version').fetchone()[0] == 4
    assert db.execute('SELECT count(*) FROM revisions').fetchone()[0] == 1
    assert db.execute('SELECT revision FROM heads').fetchone()[0] == initial['revision']
    assert db.execute("SELECT count(*) FROM result_spools WHERE json_extract(info,'$.state')='Published' AND expires_at IS NULL").fetchone()[0] == 12
    assert db.execute('SELECT sum(reserved_bytes) FROM result_spools').fetchone()[0] == sum(int(a['byteLength']) for a in bundle['assets'])
(candidate/'files.json').write_text(json.dumps(files,indent=2)+'\n')
(candidate/'bundle.json').write_text(json.dumps(bundle,indent=2)+'\n')
retry = request(query)
assert retry['result']['job'] == job
cancelled = request(dict(operation='cancelJob',jobId=job['id']))
assert cancelled['result']['job'] == job
pptx = next(a for a in bundle['assets'] if a['id'] == bundle['pptxAssetId'])
assert hashlib.sha256(invoke(['read-asset',pptx['id'],'0',pptx['byteLength']],binary=True)).hexdigest() == pptx['sha256']
changed = json.loads(json.dumps(query))
changed['request']['action']['settings']['delivery']['previewWidth'] = 320
assert request(changed)['error']['code'] == 'REQUEST_ID_REUSED'
bad = json.loads(json.dumps(query))
bad['request']['requestId'] = 'wrong-renderer'
bad['request']['action']['settings']['renderer']['implementationSha256'] = '0'*64
bad = request(bad)
assert bad['outcome'] == 'accepted'
rejected = invoke(['run',bad['job']['id']],renderer=True)
assert rejected['outcome'] == 'failed' and rejected['error']['code'] == 'EXECUTOR_MISMATCH'
with closing(sqlite3.connect(database)) as db:
    assert db.execute('SELECT count(*) FROM result_assets').fetchone()[0] == 12

# Read historic resource bytes through the new CLI after migration. These
# resource-owner fixtures have jobs and assets, but no committed documents.
def rows(path):
    with closing(sqlite3.connect(path)) as db:
        names = [r[0] for r in db.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
        return {name:db.execute('SELECT * FROM "'+name.replace('"','""')+'" ORDER BY rowid').fetchall() for name in names}


migrations = []
for name in ['candidate', 'host']:
    original = Path(f'.codex-work/job-results/pptx/{name}.sqlite')
    before_identity = entry(original)
    copy = root/f'migration-{name}.sqlite'
    shutil.copy2(original, copy)
    before = rows(copy)
    with closing(sqlite3.connect(copy)) as db:
        assert db.execute('PRAGMA user_version').fetchone()[0] == 3
        scope, principal, identifier, info = db.execute('SELECT scope,principal,id,info FROM assets LIMIT 1').fetchone()
        stored_asset = json.loads(info)
    response = request(dict(operation='readAsset', assetId=identifier),db=copy,scope=scope,principal=principal)
    assert response['outcome'] == 'succeeded' and response['result']['asset'] == stored_asset
    descriptor = stored_asset['descriptor']
    raw = invoke(['read-asset',identifier,'0',descriptor['byteLength']],db=copy,scope=scope,principal=principal,binary=True)
    assert len(raw) == int(descriptor['byteLength']) and hashlib.sha256(raw).hexdigest() == descriptor['sha256']
    after = rows(copy)
    assert all(after[table] == values for table, values in before.items())
    assert set(after)-set(before) == {'result_assets'} and not after['result_assets']
    with closing(sqlite3.connect(copy)) as db:
        assert db.execute('PRAGMA user_version').fetchone()[0] == 4
        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        assert not db.execute('PRAGMA foreign_key_check').fetchall()
    assert entry(original) == before_identity
    # sqlite3.Connection's own context manager commits/rolls back but does not
    # close. Final hashes must follow close/checkpoint, not precede it.
    migrations.append(dict(before=before_identity, after=entry(copy), existingTables=len(before), existingRows=sum(map(len,before.values())), allRowsUnchanged=True, resource=stored_asset))

with (root/'independent.log').open('x') as log:
    independent = subprocess.run([sys.executable,'tools/verification/delivery-reference.py',str(candidate)],stdout=log,stderr=subprocess.STDOUT)
assert independent.returncode == 0
assert entry(cli) == cli_identity and entry(worker) == identity
report = dict(format='musteroffice.export-host-reference/1', cli=cli_identity, worker=identity,
    calls=calls, migrations=migrations, publicAssets=12, pages=2, actualChunkAndPublicBytesEqual=True,
    acceptedBeforeExecution=True, atomicPublishedState=True, publishedBytesSurviveCancelAndReopen=True,
    idempotentResult=True, oldReceiptsUnchanged=True, reference=entry(candidate/'reference.json'),
    validatedRequests=validated_requests, validatedResponses=validated_responses,
    schemas=[entry(f'contracts/generated/{kind}.schema.json') for kind in schemas],
    database=entry(database), limitation='Actual release CLI plus owned subset; no full feature, external application, MCP or Musterwork acceptance.')
with (root/'reference.json').open('x') as out:
    json.dump(report,out,indent=2)
    out.write('\n')
print(json.dumps(dict(calls=len(calls),migrations=len(migrations),publicAssets=12,pages=2)))
