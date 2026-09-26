"""Independent real-process MCP + binary channel + SQLite/export checks.

Run from the repository root. Each run requires a new output directory.
No Office/WPS acceptance or full MCP implementation is implied.
"""
import base64
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import select
import sqlite3
import subprocess
import sys
import time
import zipfile
import io
import jsonschema

root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve(); worker = Path(sys.argv[3]).resolve()
meta = {'io.modelcontextprotocol/protocolVersion': '2026-07-28',
        'io.modelcontextprotocol/clientCapabilities': {},
        'io.modelcontextprotocol/clientInfo': {'name': 'independent-office-check', 'version': '1'}}
permissions = ['create', 'edit', 'export', 'readDocument', 'readJob', 'cancelJob', 'writeAssets', 'readAssets']
host_schema = json.loads(Path('contracts/generated/host-response.schema.json').read_text())
validator = jsonschema.Draft202012Validator(host_schema)
fixture = json.loads(Path('fixtures/presentations/delivery/input.json').read_text())
records = []

def sha(data): return hashlib.sha256(data).hexdigest()
def artifact(path):
    path = Path(path); data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=sha(data))
def config(name, **changes):
    path = root/f'{name}.config.json'
    value = dict(database=str((root/f'{name}.sqlite').resolve()), principal='mcp-verifier', scope='mcp-fixture',
                 permissions=permissions, previewWorker=dict(path=str(worker), sha256=sha(worker.read_bytes())))
    value.update(changes); path.write_text(json.dumps(value, indent=2)+'\n')
    return path

class Session:
    def __init__(self, era, cfg, name):
        self.era = era; self.cfg = cfg; self.name = name; self.seq = 0; self.pending = bytearray(); self.tools = {}
        self.stderr = root/f'{name}.stderr'; self.log = self.stderr.open('xb')
        self.p = subprocess.Popen([binary, cfg], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log)
    def __enter__(self):
        if self.era == 'legacy':
            result = self.call('initialize', dict(protocolVersion='2025-11-25', capabilities={},
                clientInfo={'name': 'independent-office-check', 'version': '1'}))['result']
            assert result['protocolVersion'] == '2025-11-25'
            self.call('notifications/initialized', notification=True)
        else:
            discovery = self.call('server/discover')['result']
            assert discovery['ttlMs'] >= 0 and discovery['cacheScope'] in ['private', 'public']
        tools = self.call('tools/list')['result']['tools']
        assert [t['name'] for t in tools] == sorted(t['name'] for t in tools)
        self.tools = {t['name']: t for t in tools}
        assert self.call('prompts/list')['error']['code'] == -32601
        for tool in tools:
            jsonschema.Draft202012Validator.check_schema(tool['inputSchema'])
            assert tool['outputSchema'] == host_schema
        return self
    def __exit__(self, kind, *_):
        self.p.stdin.close()
        try:
            if kind: self.p.kill()
            code = self.p.wait(timeout=90)
        finally:
            if self.p.poll() is None: self.p.kill(); self.p.wait()
            self.p.stdout.close(); self.log.close()
        if not kind: assert code == 0 and not self.stderr.read_bytes(), self.stderr.read_text()
    def call(self, method, params=None, *, notification=False):
        self.seq += 1
        params = {} if params is None else params.copy()
        if self.era == 'modern': params['_meta'] = meta
        request = dict(jsonrpc='2.0', method=method, params=params)
        if not notification: request['id'] = self.seq
        raw = (json.dumps(request, separators=(',', ':'))+'\n').encode()
        self.p.stdin.write(raw); self.p.stdin.flush()
        prefix = root/f'{self.name}-{self.seq:03}'
        prefix.with_suffix('.request.json').write_bytes(raw)
        if notification: return
        deadline = time.monotonic()+90
        while b'\n' not in self.pending:
            remaining = deadline-time.monotonic()
            assert remaining > 0 and select.select([self.p.stdout], [], [], remaining)[0], 'MCP response timed out'
            part = os.read(self.p.stdout.fileno(), 65536)
            assert part, f'empty MCP response: {self.stderr.read_text()}'
            self.pending.extend(part); assert len(self.pending) <= 16*1024*1024+1
        line, rest = self.pending.split(b'\n', 1); self.pending[:] = rest
        prefix.with_suffix('.response.json').write_bytes(line+b'\n')
        response = json.loads(line); assert response['id'] == self.seq
        if method in ['tools/list', 'resources/list', 'resources/templates/list', 'resources/read'] and 'result' in response:
            if self.era == 'modern':
                assert response['result']['ttlMs'] == 0 and response['result']['cacheScope'] == 'private'
            else:
                assert 'ttlMs' not in response['result'] and 'cacheScope' not in response['result']
        records.append(dict(session=self.name, id=self.seq, method=method))
        return response
    def tool(self, name, arguments, *, valid=True):
        if valid: jsonschema.Draft202012Validator(self.tools[name]['inputSchema']).validate(arguments)
        wire = self.call('tools/call', dict(name=name, arguments=arguments))
        assert 'result' in wire, wire
        result = wire['result']; value = result['structuredContent']
        validator.validate(value); assert json.loads(result['content'][0]['text']) == value
        assert result['isError'] == (value['outcome'] == 'failed')
        assert ('resultType' in result) == (self.era == 'modern')
        return value, result
    def wait(self, value):
        deadline = time.monotonic()+90
        while value['outcome'] == 'accepted':
            assert time.monotonic() < deadline
            time.sleep(.02)
            value, wire = self.tool('mo_jobs_get', dict(jobId=value['job']['id']))
        assert value['outcome'] == 'succeeded', value
        return value
    def resource(self, uri):
        wire = self.call('resources/read', dict(uri=uri)); assert 'result' in wire, wire
        result = wire['result']; assert ('resultType' in result) == (self.era == 'modern')
        if self.era == 'modern': assert result['ttlMs'] == 0 and result['cacheScope'] == 'private'
        contents = result['contents']; assert len(contents) == 1 and contents[0]['uri'] == uri
        return contents[0]

def operation(action, request_id, export=False, mode='job'):
    return dict(request=dict(contractVersion='musteroffice.operations/1-draft', requestId=request_id,
        profileId='presentations-pptx-resource-delivery-v1-draft' if export else 'presentations-author-model-v01-draft',
        outputMode=mode, action=action))
def channel(cfg, args, data=b'', binary_result=False):
    result = subprocess.run([binary, cfg, *args], input=data, capture_output=True, timeout=90)
    assert result.returncode == 0 and not result.stderr, result.stderr
    if binary_result: return result.stdout
    value = json.loads(result.stdout); validator.validate(value)
    assert value['outcome'] == 'succeeded', value
    return value
def upload(session, name, data, mime):
    prepared, _ = session.tool('mo_assets_prepare', dict(request=dict(requestId=name,
        descriptor=dict(sha256=sha(data), byteLength=str(len(data)), mediaType=mime))))
    identifier = prepared['result']['upload']['id']
    for offset in range(0, len(data), 256*1024):
        channel(session.cfg, ['append', identifier, str(offset)], data[offset:offset+256*1024])
    sealed, _ = session.tool('mo_assets_seal', dict(uploadId=identifier))
    assert sealed['outcome'] == 'succeeded'
    checked, _ = session.tool('mo_assets_get_upload', dict(uploadId=identifier))
    assert checked == sealed
    return sealed['result']['upload']['asset']

sessions = []
for era in ['legacy', 'modern']:
    # SEP-2164: current resources use Invalid Params; SDK projects the legacy
    # resource-not-found code for initialized 2025-11-25 sessions.
    missing_resource = -32002 if era == 'legacy' else -32602
    cfg = config(era)
    with Session(era, cfg, era) as session:
        assert len(session.tools) == 13
        resources = session.call('resources/list')['result']['resources']; assert len(resources) == 11
        assert len(session.call('resources/templates/list')['result']['resourceTemplates']) == 1
        limits = json.loads(session.resource('musteroffice://adapter/limits')['text'])
        assert limits['inlineAssetBytes'] == 1024*1024 and limits['inFlightRequests'] == 8
        capabilities, _ = session.tool('mo_capabilities', {})
        assert capabilities['result']['capabilities']['queuedExecution'] == 'hostScheduled'
        for resource in resources:
            if '/schemas/' not in resource['uri']: continue
            identifier = resource['uri'].rsplit('/', 1)[1]
            schema, _ = session.tool('mo_schemas_get', dict(id=identifier))
            assert json.loads(session.resource(resource['uri'])['text']) == schema['result']['document']
        for name, args in [('mo_capabilities', {'principal':'forged'}),
                           ('mo_presentations_create', operation(dict(kind='apply', documentId='wrong', baseRevision='0'*64, operations=[]), 'wrong'))]:
            value, _ = session.tool(name, args, valid=False)
            assert value['outcome'] == 'failed' and value['error']['code'] == 'INPUT_INVALID'
        assert session.call('tools/call', dict(name='missing', arguments={}))['error']['code'] == -32602
        created = session.wait(session.tool('mo_presentations_create', operation(dict(kind='create', document=fixture['document']), 'create'))[0])
        initial = created['result']['job']['result']['receipt']
        # Capture the canonical stored model (the parser materializes declared
        # inherited-style defaults absent from the compact input fixture).
        original, _ = session.tool('mo_presentations_read', dict(documentId=initial['documentId']))
        edited = session.wait(session.tool('mo_presentations_apply', operation(dict(kind='apply', documentId=initial['documentId'],
            baseRevision=initial['revision'], operations=[dict(operationId='title', operation=dict(kind='setTitle', title='MCP real integration'))]), 'edit'))[0])
        revision = edited['result']['job']['result']['receipt']['revision']
        current, _ = session.tool('mo_presentations_read', dict(documentId=initial['documentId']))
        assert current['result']['snapshot']['document'] == dict(original['result']['snapshot']['document'], title='MCP real integration')
        historical, _ = session.tool('mo_presentations_read', dict(documentId=initial['documentId'], revision=initial['revision']))
        assert historical == original
        image = upload(session, 'image', Path('fixtures/presentations/native-export/resources.bin').read_bytes(), 'image/png')
        font = upload(session, 'font', Path('fixtures/fonts/owned.ttf').read_bytes(), 'application/octet-stream')
        export = operation(dict(kind='export', documentId=initial['documentId'], baseRevision=revision,
            settings=dict(delivery=fixture['settings'], resources=[dict(resourceId='resource:checker', assetId=image['id'])],
                fontAssetId=font['id'], renderer=dict(implementationSha256=sha(worker.read_bytes()), profile='drawingml-resource-page-q32-v1-draft'))), 'export', True)
        completed = session.wait(session.tool('mo_presentations_export', export)[0])
        job = completed['result']['job']; bundle = job['result']['receipt']['bundle']
        same, wire = session.tool('mo_jobs_get', dict(jobId=job['id'])); assert same == completed
        assert session.tool('mo_presentations_export', export)[0] == completed
        assert session.tool('mo_jobs_cancel', dict(jobId=job['id']))[0] == completed
        links = {link['name']:link['uri'] for link in wire['content'][1:] if link['type'] == 'resource_link'}
        assert len(links) == len(bundle['assets']) == 12
        destination = root/f'{era}-candidate'; destination.mkdir()
        for index, asset in enumerate(bundle['assets']):
            raw = base64.b64decode(session.resource(links[asset['id']])['blob'], validate=True)
            assert len(raw) == int(asset['byteLength']) and sha(raw) == asset['sha256']
            assert channel(cfg, ['read-asset', asset['id'], '0', asset['byteLength']], binary_result=True) == raw
            (destination/f'{index:03}.bin').write_bytes(raw)
            if asset['role'] == 'pptx':
                with zipfile.ZipFile(io.BytesIO(raw)) as archive:
                    assert archive.testzip() is None
                    assert 'ppt/slides/slide2.xml' in archive.namelist()
                    assert b'<p:sp' in archive.read('ppt/slides/slide1.xml')
        (destination/'bundle.json').write_text(json.dumps(bundle, indent=2)+'\n')
        # A large asset yields a descriptor, never a silently truncated blob.
        large = b'large-asset'*(110*1024)
        info = upload(session, 'large', large, 'application/octet-stream')
        _, link = session.tool('mo_assets_read', dict(assetId=info['id']))
        uri = link['content'][1]['uri']
        descriptor = json.loads(session.resource(uri)['text']); assert descriptor['asset'] == info
        assert channel(cfg, ['read-asset', info['id'], '0', str(len(large))], binary_result=True) == large
        temporary, _ = session.tool('mo_assets_prepare', dict(request=dict(requestId='cancel-upload', descriptor=dict(
            sha256=sha(b'x'), byteLength='1', mediaType='application/octet-stream'))))
        cancelled, _ = session.tool('mo_assets_cancel_upload', dict(uploadId=temporary['result']['upload']['id']))
        assert cancelled['outcome'] == 'failed' and cancelled['error']['code'] == 'CANCELLED'
        detached_document = dict(fixture['document'], id='document:detached-client')
        detached, _ = session.tool('mo_presentations_create', operation(dict(kind='create', document=detached_document), 'disconnect-after-accepted'))
        assert detached['outcome'] == 'accepted'
    # Reconnect to the same owner and read exactly the same committed receipt.
    with Session(era, cfg, f'{era}-reconnect') as session:
        assert session.tool('mo_jobs_get', dict(jobId=job['id']))[0] == completed
        assert base64.b64decode(session.resource(next(iter(links.values())))['blob'])
        resumed = session.wait(session.tool('mo_jobs_get', dict(jobId=detached['job']['id']))[0])
        assert resumed['result']['job']['result']['receipt']['documentId'] == detached_document['id']
    with closing(sqlite3.connect(json.loads(cfg.read_text())['database'])) as database:
        assert database.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        assert not database.execute('PRAGMA foreign_key_check').fetchall()
        assert database.execute('SELECT count(*) FROM revisions').fetchone()[0] == 3
    denied = config(f'{era}-denied', database=json.loads(cfg.read_text())['database'], permissions=[])
    with Session(era, denied, f'{era}-denied') as session:
        assert list(session.tools) == ['mo_capabilities', 'mo_schemas_get']
        assert session.call('resources/templates/list')['result']['resourceTemplates'] == []
        assert session.call('resources/read', dict(uri=next(iter(links.values()))))['error']['code'] == missing_resource
        assert session.call('tools/call', dict(name='mo_jobs_get', arguments={'jobId':job['id']}))['error']['code'] == -32602
    other = config(f'{era}-other-scope', database=json.loads(cfg.read_text())['database'], scope='other-scope')
    with Session(era, other, f'{era}-other-scope') as session:
        assert session.call('resources/read', dict(uri=next(iter(links.values()))))['error']['code'] == missing_resource
        assert session.tool('mo_assets_read', dict(assetId=bundle['assets'][0]['id']))[0]['outcome'] == 'failed'
    sessions.append(dict(era=era, tools=13, schemas=10, exportedAssets=12, jobId=job['id'], candidate=str(destination)))

report = dict(binary=artifact(binary), worker=artifact(worker), sessions=sessions, calls=records,
    limitations=['Development build and self-owned synthetic font fixture; no Office/WPS visual/edit/playback acceptance.',
                 'No Streamable HTTP, Tasks, Skill/Plugin, browser owner or Musterwork integration acceptance.'])
(root/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(calls=len(records), sessions=sessions)))
