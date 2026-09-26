"""Actual release-process failure exits while the client keeps stdin OPEN."""
import hashlib
import json
from pathlib import Path
import platform
import sys
import time
from wire_check import Client

root = Path(sys.argv[1]); root.mkdir(parents=True, exist_ok=False)
binary = Path(sys.argv[2]).resolve()
config = root/'operator.json'
config.write_text(json.dumps(dict(database=str((root/'host.sqlite').resolve()), principal='lifecycle',
    scope='lifecycle', permissions=['create', 'edit', 'export', 'readDocument', 'readJob',
    'cancelJob', 'writeAssets', 'readAssets']))+'\n')
records = []
for era in ['legacy', 'modern']:
    for mode in ['closed', 'unread']:
        name = f'{era}-{mode}'
        with Client(root, binary, config, name, era, expected_exit=1) as client:
            client.start()  # Synchronize on a real response; no startup sleeps.
            if mode == 'closed': client.p.stdout.close()
            started = time.monotonic()
            client.send('tools/list')  # Full actual catalogue is larger than a pipe buffer.
            code = client.p.wait(timeout=16)
            elapsed = time.monotonic()-started
            assert code == 1 and not client.p.stdin.closed
            if mode == 'unread':
                assert 9 <= elapsed < 16, elapsed
                remaining = client.p.stdout.read()
                (root/f'{name}.partial-output.bin').write_bytes(remaining)
                assert remaining and b'\n' not in remaining
            records.append(dict(era=era, mode=mode, inputKeptOpen=True, exitCode=code, elapsedSeconds=elapsed))
        assert b'response write failed or timed out' in (root/f'{name}.stderr').read_bytes()
report = dict(binary={'path':str(binary), 'sha256':hashlib.sha256(binary.read_bytes()).hexdigest()},
    platform=platform.platform(), records=records)
(root/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report))
