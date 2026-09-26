"""Actual CLI/WASM discovery, existing v4 public data, and new export jobs."""
from contextlib import closing
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import jsonschema

root=Path(sys.argv[1]); root.mkdir()
cli=Path('target/release/mo-host'); worker=Path('target/release/mo-raster-worker')
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-export-host-verification.json').read_text())

def entry(path):
    p=Path(path); b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())

cli_id=entry(cli); worker_id=entry(worker)
schemas={name:jsonschema.Draft202012Validator(json.loads(Path(f'contracts/generated/{name}.schema.json').read_text()))
         for name in ['host-request','host-response']}
calls=[]
def invoke(query=None,*,database=None,arguments=(),renderer=False,binary=False):
    if database is None: database=root/'discovery.sqlite'
    payload=b'' if query is None else (json.dumps(query)+'\n').encode()
    if query is not None: schemas['host-request'].validate(query)
    argv=[str(cli),str(database),'verifier','fixture']
    if renderer: argv+=['--preview-worker',str(worker.resolve()),worker_id['sha256']]
    result=subprocess.run(argv+list(arguments),input=payload,capture_output=True,timeout=120)
    index=len(calls); paths={}
    for name,data in [('stdin',payload),('stdout',result.stdout),('stderr',result.stderr)]:
        p=root/f'call-{index:03}.{name}'; p.write_bytes(data); paths[name]=entry(p)
    calls.append(dict(argv=argv,exitCode=result.returncode,**paths))
    assert result.returncode==0 and not result.stderr,result.stderr.decode(errors='replace')
    if binary: return result.stdout
    response=json.loads(result.stdout); schemas['host-response'].validate(response); return response

caps=invoke(dict(operation='capabilities'))['result']['capabilities']
configured=invoke(dict(operation='capabilities'),renderer=True)['result']['capabilities']
assert len(caps['operations'])==15 and not caps['completeFeatureCatalogue'] and not caps['fullPresentationAcceptance']
catalogue_schema=json.loads(Path('contracts/generated/host-capabilities.schema.json').read_text())
assert {op['operation'] for op in caps['operations']}==set(catalogue_schema['$defs']['ServiceOperation']['enum'])
assert set(caps['schemas'])==set(catalogue_schema['$defs']['SchemaId']['enum'])
assert caps['executorDigest']==cli_id['sha256'] and caps['queuedExecution']=='explicitRun'
assert caps['renderer'] is None and configured['renderer']['implementationSha256']==worker_id['sha256']
for a,b in zip(caps['operations'],configured['operations'],strict=True):
    if a['operation']=='presentations.export':
        assert not a['available'] and a['unavailableReason']=='previewRendererNotConfigured'
        assert b['available'] and b['unavailableReason'] is None
        assert a['revisionPolicy']=='immutableHistorical'
        assert set(a['requiredPermissions'])=={'export','readDocument','readAssets'}
    else: assert a==b
assert caps['limits']==configured['limits']
documents=[]
for identifier in caps['schemas']:
    response=invoke(dict(operation='getSchema',id=identifier))
    document=response['result']['document']
    file=Path(f'contracts/generated/{identifier}.schema.json')
    assert document['schema']==json.loads(file.read_text())
    assert document['id']==identifier
    jsonschema.Draft202012Validator.check_schema(document['schema'])
    documents.append(document)
(root/'schemas.json').write_text(json.dumps(documents,indent=2)+'\n')
# The same schema documents are read in the actual generated WASM module.
program=root/'schema-parity.mjs'
program.write_text('''import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const wasm=require(process.argv[2]);
const docs=JSON.parse(fs.readFileSync(process.argv[3]));
for(const d of docs) assert.deepEqual(JSON.parse(wasm.operation_schema_json(JSON.stringify(d.id))),d);
assert.throws(()=>wasm.operation_schema_json('"private-file"'),e=>String(e).startsWith('InputInvalid:'));
assert.throws(()=>wasm.operation_schema_json(' '.repeat(129)),e=>String(e).startsWith('LimitExceeded:'));
console.log(JSON.stringify({schemaPairs:docs.length,invalidInputs:2}));
''')
with (root/'schema-parity.log').open('x') as f:
    run=subprocess.run(['node',str(program),str(Path('.codex-work/operation-discovery/wasm-node/mo_wasm.js').resolve()),str(root/'schemas.json')],stdout=f,stderr=subprocess.STDOUT)
assert run.returncode==0

original=Path('.codex-work/export-host/reference-4/host.sqlite')
original_id=entry(original)
assert original_id==next(r for r in parent['artifacts'] if r['path']==str(original))
copy=root/'old-v4.sqlite'; shutil.copy2(original,copy)
def rows(path):
    with closing(sqlite3.connect(path)) as db:
        names=[r[0] for r in db.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
        return {name:db.execute('SELECT * FROM "'+name+'" ORDER BY rowid').fetchall() for name in names}
before=rows(copy)
with closing(sqlite3.connect(copy)) as db:
    assert db.execute('PRAGMA user_version').fetchone()[0]==4
    raw=db.execute("SELECT info FROM jobs WHERE operation='presentations.export' AND json_extract(info,'$.state')='succeeded'").fetchone()[0]
old_job=json.loads(raw)
response=invoke(dict(operation='getJob',jobId=old_job['id']),database=copy)
assert response['result']['job']==old_job
assets=old_job['result']['receipt']['bundle']['assets']
for asset in assets:
    response=invoke(dict(operation='readAsset',assetId=asset['id']),database=copy)
    assert response['result']['asset']['descriptor']=={k:asset[k] for k in ['sha256','byteLength','mediaType']}
    data=invoke(database=copy,arguments=['read-asset',asset['id'],'0',asset['byteLength']],binary=True)
    assert len(data)==int(asset['byteLength']) and hashlib.sha256(data).hexdigest()==asset['sha256']
assert rows(copy)==before and entry(original)==original_id

# Reuse the frozen actual export driver without modifying its source.
script=Path('tools/verification/export-host-reference.py')
assert entry(script)==next(r for r in parent['sourceFiles'] if r['path']==str(script))
with (root/'export.log').open('x') as f:
    run=subprocess.run([sys.executable,str(script),str(root/'export')],stdout=f,stderr=subprocess.STDOUT)
assert run.returncode==0
assert entry(cli)==cli_id and entry(worker)==worker_id
report=dict(format='musteroffice.operation-discovery-reference/1',cli=cli_id,worker=worker_id,
    calls=calls,schemas=entry(root/'schemas.json'),schemaPairs=len(documents),invalidWasmInputs=2,
    schemaParityProgram=entry(program),schemaParityLog=entry(root/'schema-parity.log'),
    oldDatabase=original_id,copy=entry(copy),oldJobAndAssetsUnchanged=True,oldAssets=len(assets),
    export=entry(root/'export/reference.json'),limits=caps['limits'],
    limitation='Draft standard owner and schema discovery. No complete EmbeddedHost, MCP adapter, full WASM owner or Musterwork acceptance.')
with (root/'reference.json').open('x') as f: json.dump(report,f,indent=2); f.write('\n')
print(json.dumps(dict(calls=len(calls),schemaPairs=len(documents),oldAssets=len(assets),newExportCalls=31)))
