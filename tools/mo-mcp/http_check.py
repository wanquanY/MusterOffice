"""Real stateless HTTP export and transport checks, not Office/WPS acceptance."""
import base64
import copy
from concurrent.futures import ThreadPoolExecutor
import http.client
import json
from pathlib import Path
import sys

from compute_support import Files, create, export, read, sha, write
from http_support import Client, Server, decode, rpc

root = Path(sys.argv[1]).resolve(); root.mkdir(parents=True, exist_ok=False)
binary, worker = map(Path, sys.argv[2:4])
files = Files(root / 'caller', worker)
fixture = read('fixtures/presentations/delivery/input.json')
observations = []
with Server(binary, files) as server, Client(server) as client:
    caps = client.tool('mo_capabilities', {})
    assert caps['transport']['protocols'] == ['2026-07-28'] and caps['transport']['sessions'] is False
    assert caps['businessJobs'] is False and caps['completePresentationCapability'] is False
    assert caps['exportRenderer']['implementationSha256'] == sha(worker.read_bytes())
    for name in caps['schemas']:
        assert client.tool('mo_schema', {'id': name})['schema'] == read('contracts/generated/' + name + '.schema.json')
    original = client.computed(files.invocation('created', create(fixture['document'])))['result']['snapshot']
    protected = (files.outputs / 'created/result.json').read_bytes()
    assert client.tool('mo_presentations_compute', files.invocation('created', create(fixture['document'])))['outcome'] == 'failed'
    assert (files.outputs / 'created/result.json').read_bytes() == protected
    text = copy.deepcopy(original['document']['objects']['title:1']['content']['text'])
    text['paragraphs'][0]['runs'][0]['content']['text'] = 'A A A'
    edit = dict(request=dict(contractVersion=caps['contractVersion'], requestId='edited',
                profileId='presentations-author-model-v01-draft', action=dict(kind='apply',
                documentId=original['document']['id'], baseRevision=original['revision'],
                operations=[dict(operationId='title', operation=dict(kind='setText', object='title:1', text=text))])),
                snapshot=original)
    edited = client.computed(files.invocation('edited', edit))['result']['snapshot']
    assert edited['revision'] != original['revision']
    assert edited['document']['objects']['title:1']['content']['text'] == text
    for name, snapshot in [('exported', original), ('exported-edited', edited)]:
        invocation, bindings = export(files, snapshot, caps['exportRenderer'])
        client.computed(files.invocation(name, invocation, bindings))
        folder = files.outputs / name
        index = read(folder / 'files.json'); assert len(index) == 12
        for asset in index:
            data = (folder / asset['file']).read_bytes()
            assert sha(data) == asset['asset']['sha256'] and len(data) == int(asset['asset']['byteLength'])
            resource = client.call('resources/read', {'uri': f"musteroffice://output/{name}/{asset['file']}"})['result']['contents'][0]
            assert base64.b64decode(resource['blob'], validate=True) == data
        pptx = next(a for a in index if a['asset']['role'] == 'pptx')
        data = (folder / pptx['file']).read_bytes(); (files.inputs / 'source.pptx').write_bytes(data)
        binding = dict(file='source.pptx', info=dict(id='source', descriptor=dict(sha256=sha(data),
                       byteLength=str(len(data)), mediaType=pptx['asset']['mediaType']), verification='bytesSha256'))
        imp = dict(request=dict(contractVersion=caps['contractVersion'], requestId='import:' + name,
                   profileId='presentations-author-model-v01-draft', action=dict(kind='import',
                   documentId='import:' + name, source=dict(resourceId='source', assetId='source'))))
        imported = client.computed(files.invocation('imported-' + name, imp, [binding]))['result']['snapshot']
        assert imported['document']['sourceBindings'] and len(imported['document']['slideOrder']) == 2
        observations.append(dict(export=name, assets=len(index), pptxSha256=sha(data)))
    large = files.outputs / 'large'; large.mkdir()
    body = bytes(range(256)) * 4096; (large / 'body.bin').write_bytes(body)
    assert 'error' in client.call('resources/read', {'uri': 'musteroffice://output/large/body.bin'})
    rebuilt = bytearray()
    for offset in range(0, len(body), 262144):
        value = client.call('resources/read', {'uri': f'musteroffice://output/large/body.bin?offset={offset}&length=262144'})['result']['contents'][0]
        chunk = base64.b64decode(value['blob'], validate=True)
        meta = value['_meta']['io.musteroffice/range']
        assert meta['offset'] == str(offset) and meta['sha256'] == sha(chunk)
        rebuilt.extend(chunk)
    assert rebuilt == body
    failures = []

    def check(label, message, expected_status, code=None, **kwargs):
        status, headers, data = server.request(message, **kwargs)
        write(server.directory / (label + '.json'), dict(status=status, headers=headers, body=data.decode()))
        assert status == expected_status, (label, status, data)
        if code is not None:
            assert decode(headers, data)['error']['code'] == code, (label, data)
        failures.append(label)

    for verb in ('GET', 'DELETE', 'PUT'):
        check('method-' + verb, rpc('tools/list'), 405, verb=verb)
    for method in ('tasks/list', 'prompts/list', 'ping'):
        check(method.replace('/', '-'), rpc(method), 404, -32601)
    check('unknown-tool', rpc('tools/call', {'name': 'unknown', 'arguments': {}}), 400, -32602)
    check('bad-host', rpc('tools/list'), 403, override={'Host': 'evil.example'})
    check('bad-origin', rpc('tools/list'), 403, override={'Origin': 'https://evil.example'})
    check('null-origin', rpc('tools/list'), 403, override={'Origin': 'null'})
    check('missing-version', rpc('tools/list'), 400, override={'MCP-Protocol-Version': None})
    check('method-mismatch', rpc('tools/list'), 400, -32020, override={'Mcp-Method': 'tools/call'})
    check('name-mismatch', rpc('tools/call', {'name': 'mo_capabilities', 'arguments': {}}), 400, -32020,
          override={'Mcp-Name': 'mo_presentations_compute'})
    unknown = rpc('tools/list'); unknown['params']['_meta']['io.modelcontextprotocol/protocolVersion'] = '2099-01-01'
    check('unknown-version', unknown, 400, -32022, override={'MCP-Protocol-Version': '2099-01-01'})
    old = rpc('initialize', dict(protocolVersion='2025-11-25', capabilities={}, clientInfo=dict(name='legacy', version='1')))
    old['params'].pop('_meta')
    # The SDK uses legacy HTTP status semantics for a legacy initialize request:
    # a JSON-RPC error in HTTP 200. No initialize result/session may be returned.
    check('legacy-http', old, 200, -32022, override={'MCP-Protocol-Version': '2025-11-25'})
    old_request = rpc('tools/list')
    old_request['params']['_meta']['io.modelcontextprotocol/protocolVersion'] = '2025-11-25'
    check('legacy-inline', old_request, 400, -32022, override={'MCP-Protocol-Version': '2025-11-25'})
    check('duplicate-json', rpc('tools/list'), 400, raw=b'{"jsonrpc":"2.0","id":1,"id":2,"method":"tools/list"}')
    check('body-limit', rpc('tools/list'), 413, raw=b' ' * (4 * 1024 * 1024 + 1))
    check('content-type', rpc('tools/list'), 415, override={'Content-Type': 'text/plain'})
    check('accept', rpc('tools/list'), 406, override={'Accept': 'application/json'})
    check('header-budget', rpc('tools/list'), 431, override={'X-Oversized': 'a' * 17000})
    for header, expected in [('Mcp-Method', 400), ('Origin', 403)]:
        value = 'tools/list' if header == 'Mcp-Method' else 'https://evil.example'
        extra = [(header, value)] * (1 if header == 'Mcp-Method' else 2)
        with server.pending(rpc('tools/list'), extra_headers=extra) as sock:
            response = http.client.HTTPResponse(sock); response.begin(); data = response.read()
            assert response.status == expected, (header, response.status, data)
        failures.append('duplicate-' + header)
    # A stale legacy session hint cannot bind two stateless HTTP requests.
    status, headers, data = server.request(rpc('tools/list'), {'Mcp-Session-Id': 'invented', 'Last-Event-ID': 'old'})
    assert status == 200 and 'result' in decode(headers, data) and 'mcp-session-id' not in headers
    with Server(binary, Files(root / 'other')) as other, Client(other) as other_client:
        assert 'error' in other_client.call('resources/read', {'uri': 'musteroffice://output/created/result.json'})
    assert client.call('resources/read', {'uri': 'musteroffice://output/created/result.json'})['result']['contents'][0]['text'].encode() == protected
    files.clean()
    calls = client.seq
with Server(binary, files, 'reopened') as server, Client(server) as client:
    assert client.call('resources/read', {'uri': 'musteroffice://output/created/result.json'})['result']['contents'][0]['text'].encode() == protected
files.clean()
concurrent = Files(root / 'concurrent', worker)
config = read(concurrent.config); config['computationSlots'] = 2; write(concurrent.config, config)
with Server(binary, concurrent) as server, Client(server) as client:
    caps = client.tool('mo_capabilities', {})
    snapshot = client.computed(concurrent.invocation('created', create(fixture['document'])))['result']['snapshot']
    invocation, bindings = export(concurrent, snapshot, caps['exportRenderer'])
    args = [concurrent.invocation(name, invocation, bindings) for name in ('left', 'right')]

    def render(arguments):
        name = arguments['outputDirectory']
        with Client(server, name) as caller:
            # Do not assert all temporary files are gone until both calls finish.
            result = caller.tool('mo_presentations_compute', arguments)
            assert result['outcome'] == 'computed', result
            folder = concurrent.outputs / name
            assert result['result']['resultSha256'] == sha((folder / 'result.json').read_bytes())
            index = read(folder / 'files.json'); assert len(index) == 12
            for asset in index:
                assert sha((folder / asset['file']).read_bytes()) == asset['asset']['sha256']

    with ThreadPoolExecutor(max_workers=2) as pool:
        list(pool.map(render, args))
    concurrent.clean()
write(root / 'report.json', dict(format='musteroffice.http-mcp-check/1', status='passed', binarySha256=sha(binary.read_bytes()),
      workerSha256=sha(worker.read_bytes()), calls=calls, exports=observations, rejectionChecks=failures,
      reconstructedResourceBytes=len(rebuilt), isolatedFileNamespaces=True, concurrentExports=2, productCommitted=False,
      scope='2026-07-28 loopback HTTP with caller-mounted files; not public gateway, attachments, Office/WPS or full presentation acceptance.'))
print(json.dumps(dict(calls=calls, exports=observations, rejectionChecks=len(failures))))
