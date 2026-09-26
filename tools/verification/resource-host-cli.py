"""Actual release resource transport, independent byte/SQL checks and v1 upgrade."""
import hashlib
from contextlib import closing
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess

root=Path('.codex-work/resource-host')
binary=Path('target/release/mo-host')
database=root/'resources.sqlite'
upgraded=root/'upgraded-v1.sqlite'
assert not database.exists() and not upgraded.exists(), 'refuse to overwrite a previous database'
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-operation-host-verification.json').read_text())
def entry(path):
    path=Path(path);data=path.read_bytes()
    return dict(path=str(path),byteLength=len(data),sha256=hashlib.sha256(data).hexdigest())
prior_cli=json.loads(Path(parent['reports']['release-cli']['path']).read_text())
assert entry(prior_cli['database']['path'])==prior_cli['database']
shutil.copyfile(prior_cli['database']['path'],upgraded)
def existing_rows(path):
    with closing(sqlite3.connect(f'file:{path}?mode=ro',uri=True)) as db:
        return {table:db.execute(f'SELECT * FROM {table} ORDER BY 1,2,3').fetchall() for table in ['jobs','revisions','heads']}
before=existing_rows(upgraded)
calls=[]
def run(name,db,principal,scope,request=None,args=(),data=None,binary_output=False):
    raw=json.dumps(request).encode()+b'\n' if request is not None else (data or b'')
    result=subprocess.run([str(binary),str(db),principal,scope,*args],input=raw,capture_output=True,timeout=30)
    assert result.returncode==0 and not result.stderr,(name,result.stderr)
    response=json.loads(result.stdout) if not binary_output else None
    calls.append(dict(name=name,request=request,arguments=list(args),inputBytes=len(raw),outputBytes=len(result.stdout),outputSha256=hashlib.sha256(result.stdout).hexdigest(),response=response))
    return result.stdout if binary_output else response
old=run('upgrade-v1-and-read',upgraded,'agent:release-fixture','scope:release-fixture',dict(operation='readDocument',documentId='document:fixture'))
assert old['result']['snapshot']['document']['title']=='持久化 Agent 操作验收样本'
assert existing_rows(upgraded)==before
with closing(sqlite3.connect(upgraded)) as db:
    assert db.execute('PRAGMA user_version').fetchone()==(2,)
    assert db.execute('PRAGMA integrity_check').fetchall()==[('ok',)]
assert entry(prior_cli['database']['path'])==prior_cli['database']

chunk_bytes=256*1024
# Several fixed-size chunks plus a short tail: read ranges deliberately cross
# the storage boundary. Data is synthetic and has no decoding/format claim.
data=bytes((i*17+i//251)%256 for i in range(chunk_bytes*8+731))
source=root/'synthetic-resource.bin';source.write_bytes(data)
q=dict(operation='beginUpload',request=dict(requestId='resource',descriptor=dict(sha256=hashlib.sha256(data).hexdigest(),byteLength=str(len(data)),mediaType='application/octet-stream')))
def call(name,request=None,args=(),data=None,binary_output=False):
    return run(name,database,'agent:resource-fixture','scope:resource-fixture',request,args,data,binary_output)
started=call('begin',q);upload=started['result']['upload'];uid=upload['id']
assert upload['state']=='uploading' and upload['asset'] is None and upload['chunkBytes']==chunk_bytes
for i,offset in enumerate(range(0,len(data),chunk_bytes)):
    chunk=data[offset:offset+chunk_bytes]
    result=call(f'append-{i}',args=('append',uid,str(offset)),data=chunk)
    assert result['result']['upload']['receivedBytes']==str(offset+len(chunk))
    if i==0:assert call('retry-chunk',args=('append',uid,'0'),data=chunk)==result
sealed=call('seal',dict(operation='sealUpload',uploadId=uid))
asset=sealed['result']['upload']['asset'];assert asset['descriptor']==q['request']['descriptor']
assert sealed['result']['upload']['state']=='sealed'
assert call('retry-prepare',q)==sealed
assert call('retry-seal',dict(operation='sealUpload',uploadId=uid))==sealed
assert call('cancel-sealed',dict(operation='cancelUpload',uploadId=uid))==sealed
assert call('asset-info',dict(operation='readAsset',assetId=asset['id']))['result']['asset']==asset
download=call('download',args=('read-asset',asset['id'],'0',str(len(data))),binary_output=True)
assert download==data
target=root/'downloaded-resource.bin';target.write_bytes(download)
for offset,n in [(chunk_bytes-9,73),(len(data)-31,31),(len(data),0)]:
    assert call(f'range-{offset}',args=('read-asset',asset['id'],str(offset),str(n)),binary_output=True)==data[offset:offset+n]
with closing(sqlite3.connect(database)) as db:
    assert db.execute('PRAGMA integrity_check').fetchall()==[('ok',)]
    rows=db.execute('SELECT chunk_index,data,sha256 FROM asset_chunks ORDER BY chunk_index').fetchall()
    assert b''.join(row[1] for row in rows)==data
    assert all(hashlib.sha256(row[1]).hexdigest()==row[2] for row in rows)
    assert max(len(row[1]) for row in rows)==chunk_bytes
    assert db.execute('SELECT sum(reserved_bytes) FROM asset_uploads').fetchone()==(len(data),)
    assert db.execute('SELECT count(*) FROM assets').fetchone()==(1,)
report=dict(format='musteroffice.resource-host-release-cli/1',binary=entry(binary),database=entry(database),source=entry(source),download=entry(target),upgradedDatabase=entry(upgraded),priorDatabase=prior_cli['database'],migrationPreservedRows=True,scopeBytes=len(data),chunks=len(rows),maxChunkBytes=chunk_bytes,processInvocations=len(calls),calls=calls,limitations=['This is binary integrity and host lifecycle evidence, not file-format, Office/WPS or product acceptance.','Declared byte reservations exclude SQLite pages, journals, indexes and metadata overhead.','No throughput or RSS benchmark; per-process startup is included only in functional invocation, not measured as performance.'])
(root/'release-cli.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(dict(processInvocations=len(calls),chunks=len(rows),bytes=len(data),migrationPreservedRows=True)))
