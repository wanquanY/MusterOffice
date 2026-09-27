"""Independent JSON-RPC client and caller-owned file fixtures for thin MCP."""
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import time

import jsonschema


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def read(path):
    return json.loads(Path(path).read_text())


class Files:
    def __init__(self, root, worker=None):
        self.root = Path(root).resolve()
        self.root.mkdir(parents=True, exist_ok=False)
        for name in ('input', 'output', 'temporary'):
            (self.root / name).mkdir()
        self.config = self.root / 'config.json'
        value = dict(inputDirectory=str(self.root / 'input'), outputDirectory=str(self.root / 'output'),
                     temporaryDirectory=str(self.root / 'temporary'), computationSlots=1, controlSlots=1)
        if worker:
            value['exportWorker'] = dict(path=str(Path(worker).resolve()), sha256=sha(Path(worker).read_bytes()))
        write(self.config, value)
        self.inputs = self.root / 'input'
        self.outputs = self.root / 'output'

    def invocation(self, name, value, inputs=None):
        write(self.inputs / (name + '.json'), value)
        write(self.inputs / (name + '-inputs.json'), [] if inputs is None else inputs)
        return dict(invocationFile=name + '.json', inputsFile=name + '-inputs.json', outputDirectory=name)

    def clean(self):
        temp = self.root / 'temporary'
        assert sorted(p.name for p in temp.iterdir()) in [[], ['.mo-executions-v1']]
        if (temp / '.mo-executions-v1').exists():
            assert sorted(p.name for p in (temp / '.mo-executions-v1').iterdir()) == ['registry.lock']
        assert not list(self.root.rglob('*.sqlite')) and not list(self.root.rglob('*.db'))


class Session:
    def __init__(self, binary, files, era, name):
        self.files, self.era, self.seq = files, era, 0
        self.directory = files.root / name
        self.directory.mkdir()
        self.stderr = self.directory / 'stderr.log'
        self.log = self.stderr.open('xb')
        self.p = subprocess.Popen([str(Path(binary).resolve()), str(files.config)], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=self.log)
        self.pending = bytearray()
        self.tools = {}

    def __enter__(self):
        if self.era == '2025-11-25':
            value = self.call('initialize', dict(protocolVersion=self.era, capabilities={},
                             clientInfo=dict(name='thin-computation-check', version='1')))['result']
            assert value['protocolVersion'] == self.era
            self.send('notifications/initialized', notification=True)
        else:
            value = self.call('server/discover')['result']
        self.info = value
        tools = self.call('tools/list')['result']['tools']
        self.tools = {t['name']: t for t in tools}
        assert list(self.tools) == ['mo_capabilities', 'mo_presentations_compute', 'mo_schema']
        for tool in tools:
            jsonschema.Draft202012Validator.check_schema(tool['inputSchema'])
            if 'outputSchema' in tool:
                jsonschema.Draft202012Validator.check_schema(tool['outputSchema'])
        return self

    def __exit__(self, kind, *_):
        if not self.p.stdin.closed:
            self.p.stdin.close()
        try:
            code = self.p.wait(timeout=30)
        finally:
            if self.p.poll() is None:
                self.p.kill(); self.p.wait()
            self.p.stdout.close(); self.log.close()
        if not kind:
            assert code == 0 and not self.stderr.read_bytes(), self.stderr.read_text()

    def send(self, method, params=None, notification=False):
        self.seq += 1
        params = {} if params is None else params.copy()
        if self.era == '2026-07-28':
            params['_meta'] = {'io.modelcontextprotocol/protocolVersion': self.era,
                              'io.modelcontextprotocol/clientCapabilities': {},
                              'io.modelcontextprotocol/clientInfo': {'name': 'thin-computation-check', 'version': '1'}}
        message = dict(jsonrpc='2.0', method=method, params=params)
        if not notification:
            message['id'] = self.seq
        line = (json.dumps(message, separators=(',', ':')) + '\n').encode()
        (self.directory / f'{self.seq:03}.request.json').write_bytes(line)
        self.p.stdin.write(line); self.p.stdin.flush()
        return self.seq

    def receive(self, seconds=30):
        deadline = time.monotonic() + seconds
        while b'\n' not in self.pending:
            remaining = deadline - time.monotonic()
            assert remaining > 0 and select.select([self.p.stdout], [], [], remaining)[0], 'response timeout'
            data = os.read(self.p.stdout.fileno(), 65536)
            assert data, f'connection ended: {self.stderr.read_text()}'
            self.pending.extend(data)
            assert len(self.pending) <= 16 * 1024 * 1024 + 1
        line, _, rest = self.pending.partition(b'\n'); self.pending[:] = rest
        response = json.loads(line)
        (self.directory / f"{response.get('id', 'none')}.response.json").write_bytes(line + b'\n')
        return response

    def call(self, method, params=None):
        request_id = self.send(method, params)
        response = self.receive()
        assert response['id'] == request_id, response
        if method in ('tools/list', 'resources/list', 'resources/templates/list', 'resources/read') and 'result' in response:
            if self.era == '2026-07-28':
                assert response['result']['ttlMs'] == 0 and response['result']['cacheScope'] == 'private'
            else:
                assert 'ttlMs' not in response['result'] and 'cacheScope' not in response['result']
        return response

    def tool(self, name, arguments):
        jsonschema.Draft202012Validator(self.tools[name]['inputSchema']).validate(arguments)
        response = self.call('tools/call', dict(name=name, arguments=arguments))['result']
        value = response['structuredContent']
        assert json.loads(response['content'][0]['text']) == value
        if 'outputSchema' in self.tools[name]:
            jsonschema.Draft202012Validator(self.tools[name]['outputSchema']).validate(value)
        if name == 'mo_presentations_compute':
            assert response['isError'] == (value['outcome'] == 'failed')
        return value

    def computed(self, arguments):
        value = self.tool('mo_presentations_compute', arguments)
        assert value['outcome'] == 'computed', value
        result = value['result']
        path = self.files.outputs / result['resultFile']
        assert result['resultFile'] == arguments['outputDirectory'] + '/result.json'
        assert result['resultSha256'] == sha(path.read_bytes())
        assert int(result['resultByteLength']) == path.stat().st_size
        assert result['productCommitted'] is False
        self.files.clean()
        receipt = read(path)
        schema = read('contracts/generated/computation-receipt.schema.json')
        jsonschema.Draft202012Validator(schema).validate(receipt)
        return receipt


def create(document, request_id='create'):
    return dict(request=dict(contractVersion='musteroffice.computation/1-draft', requestId=request_id,
                             profileId='presentations-author-model-v01-draft', action=dict(kind='create', document=document)))


def export(files, snapshot, renderer):
    fixture = read('fixtures/presentations/delivery/input.json')
    bindings = []
    for name, path, media in [('image', 'fixtures/presentations/native-export/resources.bin', 'image/png'),
                               ('font', 'fixtures/fonts/owned.ttf', 'font/ttf')]:
        data = Path(path).read_bytes(); (files.inputs / (name + '.bin')).write_bytes(data)
        bindings.append(dict(file=name + '.bin', info=dict(id=name + ':1', descriptor=dict(sha256=sha(data),
                        byteLength=str(len(data)), mediaType=media), verification='bytesSha256')))
    request = dict(contractVersion='musteroffice.computation/1-draft', requestId='export:embedded-test',
                   profileId='presentations-pptx-resource-delivery-v1-draft', action=dict(kind='export',
                   documentId=snapshot['document']['id'], baseRevision=snapshot['revision'], settings=dict(
                   delivery=fixture['settings'], resources=[dict(resourceId='resource:checker', assetId='image:1')],
                   fontAssetId='font:1', renderer=renderer)))
    return dict(request=request, snapshot=snapshot), bindings
