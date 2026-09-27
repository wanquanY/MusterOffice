"""Real TCP disconnect/shutdown cancellation and bounded HTTP admission.

Blocked workers are deliberate fixtures; timing is not renderer performance.
"""
import http.client
import json
import os
from pathlib import Path
import sys
import time

from compute_support import Files, create, export, read, sha, write
from http_support import Client, Server, rpc


def wait_for(predicate, label, seconds=10):
    deadline = time.monotonic() + seconds
    while not predicate():
        assert time.monotonic() < deadline, label
        time.sleep(.01)


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


root = Path(sys.argv[1]).resolve(); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve()
fixture = read('fixtures/presentations/delivery/input.json')
observations = []
for mode in ('disconnect', 'shutdown'):
    ack = root / (mode + '.pid'); worker = root / (mode + '.py')
    worker.write_text('#!' + sys.executable + '\nimport os,sys,time\n'
                      'sys.stdin.buffer.read(1)\n'
                      f'with open({str(ack)!r},"w") as out: out.write(str(os.getpid()))\n'
                      'while True: time.sleep(.1)\n')
    worker.chmod(0o700)
    files = Files(root / mode, worker)
    with Server(binary, files) as server, Client(server) as client:
        caps = client.tool('mo_capabilities', {})
        snapshot = client.computed(files.invocation('created', create(fixture['document'])))['result']['snapshot']
        invocation, bindings = export(files, snapshot, caps['exportRenderer'])
        arguments = files.invocation('blocked', invocation, bindings)
        pending = server.pending(rpc('tools/call', dict(name='mo_presentations_compute', arguments=arguments)))
        try:
            wait_for(lambda: ack.exists() and ack.read_text().isdigit(), 'worker did not start')
            pid = int(ack.read_text()); assert alive(pid)
            busy = files.invocation('busy', create(fixture['document']))
            failed = client.tool('mo_presentations_compute', busy)
            assert failed['outcome'] == 'failed' and failed['error']['code'] == 'LIMIT_EXCEEDED'
            # Same request id on a separate HTTP request must not cancel the worker.
            assert server.request(rpc('tools/list'))[0] == 200 and alive(pid)
            assert 'result' in client.call('resources/read', {'uri': 'musteroffice://output/created/result.json'})
            start = time.monotonic()
            if mode == 'disconnect':
                pending.close()
                wait_for(lambda: not alive(pid), 'disconnected worker still alive')
                for _ in range(100):
                    value = client.tool('mo_presentations_compute', busy)
                    if value['outcome'] == 'computed':
                        break
                    assert value['error']['code'] == 'LIMIT_EXCEEDED', value
                    time.sleep(.01)
                else:
                    raise AssertionError('disconnect did not release computation admission')
            else:
                assert server.stop() == 0
                assert not alive(pid), 'shutdown left a child alive'
            assert not (files.outputs / 'blocked').exists()
            files.clean()
            observations.append(dict(mode=mode, workerStopped=True, privateFilesReleased=True,
                                     elapsedSeconds=time.monotonic() - start))
        finally:
            pending.close()
    files.clean()

acks = root / 'independent-pids'; acks.mkdir()
worker = root / 'independent-worker.py'
worker.write_text('#!' + sys.executable + '\nimport os,sys,time\nfrom pathlib import Path\n'
                  'sys.stdin.buffer.read(1)\n'
                  f'(Path({str(acks)!r}) / str(os.getpid())).touch()\n'
                  'while True: time.sleep(.1)\n')
worker.chmod(0o700)
files = Files(root / 'independent', worker)
config = read(files.config); config['computationSlots'] = 2; write(files.config, config)
with Server(binary, files) as server, Client(server) as client:
    caps = client.tool('mo_capabilities', {})
    snapshot = client.computed(files.invocation('created', create(fixture['document'])))['result']['snapshot']
    invocation, bindings = export(files, snapshot, caps['exportRenderer'])
    sockets = []
    try:
        arguments = files.invocation('left', invocation, bindings)
        sockets.append(server.pending(rpc('tools/call', dict(name='mo_presentations_compute', arguments=arguments))))
        wait_for(lambda: len(list(acks.iterdir())) == 1, 'first worker did not start')
        first = int(next(acks.iterdir()).name)
        arguments = files.invocation('right', invocation, bindings)
        sockets.append(server.pending(rpc('tools/call', dict(name='mo_presentations_compute', arguments=arguments))))
        wait_for(lambda: len(list(acks.iterdir())) == 2, 'second worker did not start')
        second = next(int(p.name) for p in acks.iterdir() if int(p.name) != first)
        assert alive(first) and alive(second)
        sockets[0].close()
        wait_for(lambda: not alive(first), 'first request did not cancel')
        assert alive(second), 'same RPC id coupled independent requests'
        for _ in range(100):
            value = client.tool('mo_presentations_compute', files.invocation('after-first', create(fixture['document'])))
            if value['outcome'] == 'computed':
                break
            assert value['error']['code'] == 'LIMIT_EXCEEDED', value
            time.sleep(.01)
        else:
            raise AssertionError('first request did not release its own slot')
        assert alive(second)
        sockets[1].close()
        wait_for(lambda: not alive(second), 'second request did not cancel')
    finally:
        for sock in sockets:
            sock.close()
assert not (files.outputs / 'left').exists() and not (files.outputs / 'right').exists()
files.clean()
observations.append(dict(mode='independent-requests', sameRpcId=True, activeWorkers=2, isolatedCancellation=True))

files = Files(root / 'admission')
with Server(binary, files) as server:
    sockets = []
    try:
        # Partial bodies keep handler admission alive. Probes establish when all
        # eight requests have been admitted without relying on a fixed sleep.
        for _ in range(8):
            sockets.append(server.pending(rpc('tools/list'), partial=True))
        statuses = []

        def full():
            status = server.request(rpc('tools/list'))[0]
            statuses.append(status)
            assert status in (200, 429), status
            return status == 429

        wait_for(full, 'HTTP admission did not fill', seconds=3)
        assert server.request(rpc('tools/list'))[0] == 429
    finally:
        for sock in sockets:
            sock.close()
    wait_for(lambda: server.request(rpc('tools/list'))[0] == 200, 'HTTP admission leaked')
    start = time.monotonic()
    with server.pending(rpc('tools/list'), partial=True) as sock:
        response = http.client.HTTPResponse(sock); response.begin(); response.read()
        assert response.status == 408, response.status
    elapsed = time.monotonic() - start
    assert 9 <= elapsed < 15, elapsed
    assert server.request(rpc('tools/list'))[0] == 200
    observations.append(dict(mode='admission', capacity=8, overflowStatus=429, slowBodyStatus=408,
                             slowBodyElapsedSeconds=elapsed, disconnectedSlotsRecovered=True))
files.clean()
write(root / 'report.json', dict(format='musteroffice.http-mcp-lifecycle/1', status='passed',
      binarySha256=sha(binary.read_bytes()), observations=observations,
      scope='Loopback TCP on macOS with deliberately blocked child, not WAN or renderer performance.'))
print(json.dumps(observations))
