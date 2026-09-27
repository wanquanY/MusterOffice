"""Real thin MCP computation over both pinned protocol profiles and caller files.

Not Office/WPS, complete presentation coverage, release size or throughput evidence.
"""
import base64
import copy
import json
from pathlib import Path
import sys

from compute_support import Files, Session, create, export, read, sha, write

root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary, worker = map(Path, sys.argv[2:4])
fixture = read('fixtures/presentations/delivery/input.json')
observations = []
for era in ('2025-11-25', '2026-07-28'):
    files = Files(root / era, worker)
    with Session(binary, files, era, 'protocol') as client:
        caps = client.tool('mo_capabilities', {})
        assert caps['businessJobs'] is False and caps['completePresentationCapability'] is False
        assert caps['exportRenderer']['implementationSha256'] == sha(worker.read_bytes())
        for name in caps['schemas']:
            schema = client.tool('mo_schema', {'id': name})
            assert schema['schema'] == read('contracts/generated/' + name + '.schema.json')
        assert client.call('prompts/list')['error']['code'] == -32601
        assert client.call('tasks/list')['error']['code'] == -32601
        assert client.call('tools/call', dict(name='mo_jobs_get', arguments={}))['error']['code'] == -32602
        assert client.call('resources/list')['result']['resources'] == []
        missing=client.call('resources/read', {'uri':'musteroffice://output/missing/file.bin'})
        assert missing['error']['code']==(-32002 if era=='2025-11-25' else -32602)
        templates = client.call('resources/templates/list')['result']['resourceTemplates']
        assert len(templates) == 1
        args = files.invocation('created', create(fixture['document']))
        original = client.computed(args)['result']['snapshot']
        protected = (files.outputs / 'created/result.json').read_bytes()
        assert client.tool('mo_presentations_compute', args)['outcome'] == 'failed'
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
        requests = []
        for name, snapshot in [('exported', original), ('exported-edited', edited)]:
            invocation, bindings = export(files, snapshot, caps['exportRenderer'])
            args = files.invocation(name, invocation, bindings)
            result = client.computed(args)['result']['receipt']
            folder = files.outputs / name
            index = read(folder / 'files.json'); assert len(index) == 12
            for asset in index:
                data = (folder / asset['file']).read_bytes()
                assert sha(data) == asset['asset']['sha256'] and len(data) == int(asset['asset']['byteLength'])
                uri = f"musteroffice://output/{name}/{asset['file']}"
                resource = client.call('resources/read', {'uri': uri})['result']['contents'][0]
                actual = base64.b64decode(resource['blob'], validate=True)
                assert actual == data
                assert resource['_meta']['io.musteroffice/range']['sha256'] == sha(actual)
            pptx = next(asset for asset in index if asset['asset']['role'] == 'pptx')
            data = (folder / pptx['file']).read_bytes()
            (files.inputs / 'source.pptx').write_bytes(data)
            binding = dict(file='source.pptx', info=dict(id='source', descriptor=dict(sha256=sha(data),
                           byteLength=str(len(data)), mediaType=pptx['asset']['mediaType']), verification='bytesSha256'))
            imp = dict(request=dict(contractVersion=caps['contractVersion'], requestId='import:' + name,
                       profileId='presentations-author-model-v01-draft', action=dict(kind='import',
                       documentId='import:' + name, source=dict(resourceId='source', assetId='source'))))
            imported = client.computed(files.invocation('imported-' + name, imp, [binding]))['result']['snapshot']
            assert imported['document']['sourceBindings'] and len(imported['document']['slideOrder']) == 2
            requests.append(dict(name=name, pptxSha256=sha(data), assets=len(index)))
        # Metadata resource is the actual caller-owned JSON, not a hidden store.
        result_uri = 'musteroffice://output/created/result.json'
        resource = client.call('resources/read', {'uri': result_uri})['result']['contents'][0]
        assert resource['text'].encode() == protected
        for bad in ['../config.json', '/etc/passwd', 'a/b', 'a\\b', 'NUL.txt']:
            args = dict(invocationFile=bad, inputsFile='created-inputs.json', outputDirectory='invalid')
            assert client.tool('mo_presentations_compute', args)['outcome'] == 'failed'
            assert not (files.outputs / 'invalid').exists()
        (files.root / 'outside.pptx').write_bytes(data)
        binding['file'] = '../outside.pptx'
        assert client.tool('mo_presentations_compute', files.invocation('escape', imp, [binding]))['outcome'] == 'failed'
        assert not (files.outputs / 'escape').exists()
        if sys.platform != 'win32':
            (files.inputs / 'linked.json').symlink_to(files.inputs / 'created.json')
            args = dict(invocationFile='linked.json', inputsFile='created-inputs.json', outputDirectory='linked')
            assert client.tool('mo_presentations_compute', args)['outcome'] == 'failed'
            (files.outputs / 'linked-directory').symlink_to(files.inputs, target_is_directory=True)
            assert 'error' in client.call('resources/read', {'uri': 'musteroffice://output/linked-directory/created.json'})
        large = files.outputs / 'large'; large.mkdir()
        body = bytes(range(256)) * 4096; (large / 'body.bin').write_bytes(body)
        assert 'error' in client.call('resources/read', {'uri': 'musteroffice://output/large/body.bin'})
        rebuilt = bytearray()
        for offset in range(0, len(body), 262144):
            resource = client.call('resources/read', {'uri': f'musteroffice://output/large/body.bin?offset={offset}&length=262144'})['result']['contents'][0]
            chunk = base64.b64decode(resource['blob'], validate=True)
            meta = resource['_meta']['io.musteroffice/range']
            assert meta['offset'] == str(offset) and meta['totalByteLength'] == str(len(body)) and meta['sha256'] == sha(chunk)
            rebuilt.extend(chunk)
        assert rebuilt == body
        for suffix in ['?offset=00&length=1', '?offset=0&length=262145', '?offset=0&length=0', '?offset=18446744073709551616&length=1', '?offset=0&length=1&x=y']:
            assert 'error' in client.call('resources/read', {'uri': 'musteroffice://output/large/body.bin' + suffix})
        files.clean()
        observations.append(dict(protocol=era, calls=client.seq, exports=requests, largeBytes=len(rebuilt)))
    # Restart does not need a database or reconstruct a job. The caller still
    # owns its files and may read them through the same explicitly configured root.
    with Session(binary, files, era, 'reopened') as client:
        resource = client.call('resources/read', {'uri': result_uri})['result']['contents'][0]
        assert resource['text'].encode() == protected
    files.clean()
# Multiple admitted exports share only the physical registry. Temporary lock
# contention must wait within its bound, not become a random failed computation.
files = Files(root / 'concurrent', worker)
config = read(files.config); config['computationSlots'] = 2; write(files.config, config)
with Session(binary, files, '2025-11-25', 'protocol') as client:
    caps = client.tool('mo_capabilities', {})
    snapshot = client.computed(files.invocation('created', create(fixture['document'])))['result']['snapshot']
    for batch in range(4):
        invocation, bindings = export(files, snapshot, caps['exportRenderer'])
        pending = {}
        for side in ('left', 'right'):
            name = f'{side}-{batch}'
            args = files.invocation(name, invocation, bindings)
            request_id = client.send('tools/call', dict(name='mo_presentations_compute', arguments=args))
            pending[request_id] = name
        for _ in range(2):
            response = client.receive(); name = pending.pop(response['id'])
            value = response['result']['structuredContent']
            assert value['outcome'] == 'computed', value
            folder = files.outputs / name
            assert value['result']['resultSha256'] == sha((folder / 'result.json').read_bytes())
            index = read(folder / 'files.json'); assert len(index) == 12
            for asset in index:
                assert sha((folder / asset['file']).read_bytes()) == asset['asset']['sha256']
        files.clean()
    observations.append(dict(protocol='2025-11-25', concurrentBatches=4, concurrentExports=8))
# Protocol era must not change document computation or actual output bytes.
a = root / '2025-11-25/output/exported'
b = root / '2026-07-28/output/exported'
assert read(a / 'files.json') == read(b / 'files.json')
for f in read(a / 'files.json'):
    assert (a / f['file']).read_bytes() == (b / f['file']).read_bytes()
write(root / 'report.json', dict(format='musteroffice.thin-mcp-files-check/1', status='passed',
      binarySha256=sha(binary.read_bytes()), workerSha256=sha(worker.read_bytes()), observations=observations,
      protocolOutputsByteIdentical=True, productCommitted=False, scope='Local caller-file bridge only; no remote gateway, Office/WPS or full presentation acceptance.'))
print(json.dumps(observations))
