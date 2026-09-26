"""Real SDK cancellation while a native host call waits for a SQLite writer.

The busy response proves an actual bridge permit is held before cancellation.
No test-only server hooks, fake renderers, or scheduler timing assumptions.
"""
from contextlib import closing
import copy
import hashlib
import json
from pathlib import Path
import sqlite3
import sys
import time
from wire_check import Client

root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve()
fixture = json.loads(Path('fixtures/presentations/delivery/input.json').read_text())
records = []


def arguments(identity):
    return dict(name='mo_presentations_create', arguments=dict(request=dict(
        contractVersion='musteroffice.operations/1-draft', requestId=identity,
        profileId='presentations-author-model-v01-draft', outputMode='sync',
        action=dict(kind='create', document=copy.deepcopy(fixture['document'])))))


def busy(response):
    value = response['result']['structuredContent']
    assert value['outcome'] == 'failed' and value['error']['code'] == 'LIMIT_EXCEEDED', value


def retry(client, identity):
    deadline = time.monotonic()+10
    while True:
        response = client.call('tools/call', arguments(identity))
        value = response['result']['structuredContent']
        if value['outcome'] == 'succeeded': return value['result']['job']
        busy(response)
        assert time.monotonic() < deadline
        time.sleep(.01)


for era in ['legacy', 'modern']:
    cfg = root/f'{era}.json'; database = root/f'{era}.sqlite'
    cfg.write_text(json.dumps(dict(database=str(database.resolve()), principal='cancel-verifier',
        scope='cancel-fixture', permissions=['create', 'readDocument', 'readJob'], computationSlots=1))+'\n')
    with Client(root, binary, cfg, era, era) as client:
        client.start()
        with closing(sqlite3.connect(database)) as conn:
            conn.execute('BEGIN IMMEDIATE')
            try:
                ids = {client.send('tools/call', arguments(identity)): identity for identity in ['blocked-a', 'blocked-b']}
                denied = client.receive(); busy(denied)
                assert denied['id'] in ids
                pending_id = next(i for i in ids if i != denied['id'])
                pending_request = ids[pending_id]
                assert conn.execute('SELECT count(*) FROM jobs').fetchone()[0] == 0
                client.send('notifications/cancelled', {'requestId': pending_id}, notification=True)
                # A protocol error while native computation is blocked must be
                # written intact. A subsequent ordinary response remains usable.
                client.raw(b'{\n'); assert client.receive()['error']['code'] == -32700
                assert 'result' in client.call('resources/list')
                busy(client.call('tools/call', arguments('after-protocol-cancel')))
            finally:
                conn.rollback()  # Release the writer. The native operation can proceed.
            deadline = time.monotonic()+10
            while True:
                rows = conn.execute('SELECT request_id,info FROM jobs').fetchall()
                if rows and json.loads(rows[0][1])['state'] == 'succeeded': break
                assert time.monotonic() < deadline, rows
                time.sleep(.01)
            assert len(rows) == 1 and rows[0][0] == pending_request
            stored = json.loads(rows[0][1]); assert stored['cancelRequested'] is False
            assert conn.execute('SELECT count(*) FROM heads').fetchone()[0] == 1
        # Both stdio versions cancel the waiter without a further response.
        # Neither is a business cancellation or a duplicate document write.
        assert 'result' in client.call('resources/list')
        assert not any(f.get('id') == pending_id for f in client.frames)
        job = retry(client, pending_request)
        assert job['id'] == stored['id'] and job['result'] == stored['result']
        # Repeated cancellations must not leak active slots. Retry the same
        # persisted business operation under the lock: no duplicate mutations.
        for cycle in range(10):
            with closing(sqlite3.connect(database)) as conn:
                conn.execute('BEGIN IMMEDIATE')
                try:
                    ids = [client.send('tools/call', arguments(pending_request)) for _ in range(2)]
                    denied = client.receive(); busy(denied)
                    assert denied['id'] in ids
                    cancelled_id = next(i for i in ids if i != denied['id'])
                    client.send('notifications/cancelled', {'requestId':cancelled_id}, notification=True)
                    assert 'result' in client.call('resources/list')
                    busy(client.call('tools/call', arguments('must-not-run')))
                finally:
                    conn.rollback()
            job = retry(client, pending_request)
            assert job['id'] == stored['id'] and job['result'] == stored['result']
            assert not any(f.get('id') == cancelled_id for f in client.frames)
    with Client(root, binary, cfg, era+'-reconnect', era) as client:
        client.start()
        again = retry(client, pending_request)
        assert again['id'] == stored['id'] and again['result'] == stored['result']
    with closing(sqlite3.connect(database)) as conn:
        assert conn.execute('SELECT count(*) FROM jobs').fetchone()[0] == 1
        assert conn.execute('SELECT count(*) FROM revisions').fetchone()[0] == 1
    for name in [era, era+'-reconnect']: assert not (root/f'{name}.stderr').read_bytes()
    records.append(dict(era=era, pendingProtocolId=pending_id, businessRequestId=pending_request,
        jobId=stored['id'], cancellationRequested=False, documents=1, revisions=1,
        nativePermitHeldAfterProtocolCancellation=True, cancelledProtocolRequests=11,
        retryAndReconnectSameReceipt=True))

report = dict(binary={'path':str(binary), 'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}, records=records)
(root/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report))
