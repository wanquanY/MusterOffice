"""Real stdio protocol recovery, bounded fatal inputs and binary authorization."""
from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
from wire_check import Client

root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve()
config = root/'operator.json'
config.write_text(json.dumps(dict(database=str((root/'host.sqlite').resolve()),
    principal='transport-test', scope='transport-test', permissions=[]))+'\n')
faults = [
    ('bad-json', b'{\n', -32700, None),
    ('invalid-utf8', b'\xff\n', -32700, None),
    ('empty-frame', b'\n', -32700, None),
    ('null-id', b'{"jsonrpc":"2.0","id":null,"method":"ping"}\n', -32600, None),
    ('duplicate-id', b'{"jsonrpc":"2.0","id":1,"id":2,"method":"ping"}\n', -32600, None),
    ('batch', b'[]\n', -32600, None),
    ('wrong-version', b'{"jsonrpc":"1.0","id":11,"method":"ping"}\n', -32600, 11),
    ('missing-params', b'{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{}}\n', -32602, 12),
    ('array-params', b'{"jsonrpc":"2.0","id":13,"method":"resources/read","params":[]}\n', -32602, 13),
]
recovery = []
for era in ['legacy', 'modern']:
    cfg = root/f'{era}.json'; db = root/f'{era}.sqlite'
    configuration = json.loads(config.read_text())
    configuration.update(database=str(db.resolve()), permissions=['create', 'readDocument', 'readJob'])
    cfg.write_text(json.dumps(configuration)+'\n')
    with Client(root, binary, cfg, era, era) as client:
        # Exercise recovery both before the SDK handshake and after it.
        client.raw(b'{\n'); assert client.receive()['error']['code'] == -32700
        client.start()
        ping = client.call('ping')
        if era == 'modern': assert ping['error']['code'] == -32601
        else: assert 'result' in ping
        for name, raw, code, identity in faults:
            client.raw(raw)
            value = client.receive()
            assert value['jsonrpc'] == '2.0' and value['error']['code'] == code
            assert value.get('id') == identity and ('id' in value) == (identity is not None)
            assert 'result' in client.call('resources/list')
            recovery.append(dict(era=era, case=name, code=code, identity=identity))
        client.raw(b'{"jsonrpc":"2.0","method":"notifications/cancelled","params":7}\n'
                   b'{"jsonrpc":"2.0","method":"notifications/unknown","params":{}}\n')
        assert 'result' in client.call('resources/list')  # No notification response precedes this.
        for method in ['unknown/test', 'prompts/list', 'logging/setLevel', 'tasks/get']:
            assert client.call(method, {'level': 'info', 'taskId': 'unknown'})['error']['code'] == -32601
        fixture = json.loads(Path('fixtures/presentations/delivery/input.json').read_text())
        request = dict(contractVersion='musteroffice.operations/1-draft', requestId='after-recovery',
            profileId='presentations-author-model-v01-draft', outputMode='sync',
            action=dict(kind='create', document=fixture['document']))
        value = client.call('tools/call', dict(name='mo_presentations_create', arguments={'request': request}))
        result = value['result']['structuredContent']
        assert result['outcome'] == 'succeeded', result
        # Errors never entered the job owner. A legitimate mutation still works.
        with closing(sqlite3.connect(db)) as conn:
            assert conn.execute('SELECT request_id FROM jobs').fetchall() == [('after-recovery',)]
            assert conn.execute('SELECT count(*) FROM heads').fetchone()[0] == 1
    assert not (root/f'{era}.stderr').read_bytes()

cases = {
    'oversize': b'x'*(4*1024*1024+1)+b'\n',
    'unterminated': b'{"jsonrpc":"2.0","id":1,"method":"ping"}',
    'unsolicited-response': b'{"jsonrpc":"2.0","id":1,"result":{}}\n',
}
report = []
for name, data in cases.items():
    result = subprocess.run([binary, config], input=data, capture_output=True, timeout=30)
    (root/f'{name}.stdout').write_bytes(result.stdout)
    (root/f'{name}.stderr').write_bytes(result.stderr)
    assert result.returncode != 0 and not result.stdout, name
    report.append(dict(case=name, exitCode=result.returncode, inputBytes=len(data)))
# The operator channel must not silently grant the default CLI's full rights.
append = subprocess.run([binary, config, 'append', 'not-owned', '0'], input=b'x', capture_output=True, timeout=30)
assert append.returncode == 0 and not append.stderr
assert json.loads(append.stdout)['error']['code'] == 'NOT_AUTHORIZED'
read = subprocess.run([binary, config, 'read-asset', 'not-owned', '0', '1'], capture_output=True, timeout=30)
assert read.returncode != 0 and not read.stdout
(root/'append.stdout').write_bytes(append.stdout)
(root/'read.stderr').write_bytes(read.stderr)
# Invalid operator configuration never starts a server or creates its database.
invalid = root/'invalid.json'
bad_database = root/'invalid.sqlite'
invalid.write_text(json.dumps(dict(database=str(bad_database.resolve()), principal='p', scope='s', permissions=[], computationSlots=0)))
result = subprocess.run([binary, invalid], input=b'', capture_output=True, timeout=30)
assert result.returncode != 0 and not result.stdout and not bad_database.exists()
(root/'invalid-config.stderr').write_bytes(result.stderr)
(root/'report.json').write_text(json.dumps(dict(cases=report, recovery=recovery, realDocumentsCreated=2,
    binary={'path':str(binary), 'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},
    binaryAuthorization=True, invalidConfigRejected=True), indent=2)+'\n')
print(json.dumps(dict(rejectedFrames=len(cases), recoveredFrames=len(recovery), binaryAuthorization=True, invalidConfigRejected=True)))
