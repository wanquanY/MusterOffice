"""Real protocol cancellation/EOF must stop child work and release its slot/files.

A deliberately blocked private worker makes the boundary deterministic. Positive
rendering is tested independently by compute_check.py with the real renderer.
"""
import json
import os
from pathlib import Path
import sys
import time

from compute_support import Files, Session, create, export, read, sha, write

root = Path(sys.argv[1]).resolve(); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve()
fixture = read('fixtures/presentations/delivery/input.json')
observations = []


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


for era in ('2025-11-25', '2026-07-28'):
    for mode in ('cancel', 'eof'):
        name = era + '-' + mode
        ack = root / (name + '.worker-pid')
        worker = root / (name + '.worker.py')
        worker.write_text('#!' + sys.executable + '\nimport os,sys,time\n'
                          'sys.stdin.buffer.read(1)\n'
                          f'with open({str(ack)!r},"w") as out: out.write(str(os.getpid()))\n'
                          'while True: time.sleep(.1)\n')
        worker.chmod(0o700)
        files = Files(root / name, worker)
        with Session(binary, files, era, 'protocol') as client:
            caps = client.tool('mo_capabilities', {})
            snapshot = client.computed(files.invocation('created', create(fixture['document'])))['result']['snapshot']
            invocation, bindings = export(files, snapshot, caps['exportRenderer'])
            args = files.invocation('blocked', invocation, bindings)
            pending = client.send('tools/call', dict(name='mo_presentations_compute', arguments=args))
            wait_for(lambda: ack.exists() and ack.read_text().isdigit(), 'worker did not start')
            pid = int(ack.read_text()); assert alive(pid)
            # The expensive slot remains occupied; control/resource access stays
            # available without requiring a job database or secondary session.
            busy_args = files.invocation('busy', create(fixture['document']))
            failure = client.tool('mo_presentations_compute', busy_args)
            assert failure['outcome'] == 'failed' and failure['error']['code'] == 'LIMIT_EXCEEDED'
            assert not (files.outputs / 'busy').exists()
            ping = client.call('ping')
            assert (ping.get('result') == {}) if era == '2025-11-25' else ping['error']['code'] == -32601
            resource = client.call('resources/read', {'uri': 'musteroffice://output/created/result.json'})
            assert 'result' in resource
            start = time.monotonic()
            if mode == 'cancel':
                client.send('notifications/cancelled', dict(requestId=pending, reason='independent cancellation check'), notification=True)
                wait_for(lambda: not alive(pid), 'cancelled worker still alive')
                # Read actual resource ownership until cleanup completes. Only a
                # busy slot may be retried; no computation error is hidden.
                for attempt in range(100):
                    value = client.tool('mo_presentations_compute', busy_args)
                    if value['outcome'] == 'computed':
                        break
                    assert value['error']['code'] == 'LIMIT_EXCEEDED', value
                    time.sleep(.01)
                else:
                    raise AssertionError('cancelled work did not release its slot')
                assert not (files.outputs / 'blocked').exists()
                files.clean()
            else:
                client.p.stdin.close()
                assert client.p.wait(timeout=10) == 0
                assert not alive(pid), 'EOF left a worker alive'
                assert not (files.outputs / 'blocked').exists()
                files.clean()
            observations.append(dict(protocol=era, mode=mode, childStopped=True, privateFilesReleased=True,
                                     elapsedSeconds=time.monotonic() - start))
        files.clean()
write(root / 'report.json', dict(format='musteroffice.thin-mcp-lifecycle/1', status='passed',
      binarySha256=sha(binary.read_bytes()), observations=observations,
      scope='macOS/Unix stdio test with deliberately blocked private worker, not renderer performance.'))
print(json.dumps(observations))
