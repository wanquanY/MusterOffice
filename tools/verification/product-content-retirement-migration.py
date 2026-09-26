"""Upgrade databases produced by the frozen product V183 executable in situ.

Only relative paths, hashes and observations leave the private product repo.
No private executable or database is copied into the kernel repository.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument('--product-repo', type=Path, required=True)
parser.add_argument('--current-executable', type=Path, required=True)
parser.add_argument('--output-dir', type=Path, required=True)
args = parser.parse_args()
product = args.product_repo.resolve()
current = (product / args.current_executable).resolve()
current.relative_to(product)
old = product / '.codex-work/musteroffice-retirement-v183/sqlite-test'
output = args.output_dir
output.mkdir(exist_ok=False)


def entry(path):
    data = path.read_bytes()
    return dict(path=path.relative_to(product).as_posix(), byteLength=len(data),
                sha256=hashlib.sha256(data).hexdigest())


assert entry(old)['sha256'] == '7275b577cab45ef1e3cc561e23f6bccd890fbf46a5961639a5d3fabf1b48fe3f'
observations = []
for tombstoned in [False, True]:
    name = 'tombstoned' if tombstoned else 'published'
    with tempfile.TemporaryDirectory(prefix='musteroffice-retirement-upgrade-', dir=product / '.codex-work') as temporary:
        root = Path(temporary)
        # The child creates a V183 store through the real previous product
        # startup and writes immutable bytes through its real content port.
        with (output / f'{name}-prior.log').open('x') as log:
            import os
            child = subprocess.Popen(
                [str(old), '--exact', 'content::material::tests::crash_writer_process', '--ignored', '--nocapture'],
                env={**os.environ, 'MO_CONTENT_CRASH_ROOT': str(root), 'MO_CONTENT_CRASH_PHASE': 'published'},
                stdout=log, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 120
                while not (root / 'ready').exists():
                    assert child.poll() is None, 'old child exited before published commit'
                    assert time.monotonic() < deadline, 'old child timed out'
                    time.sleep(0.05)
                assert child.poll() is None
                assert (root / 'ready').read_text() == 'published'
                child.kill()
                assert child.wait(timeout=10) < 0
            finally:
                if child.poll() is None:
                    child.kill()
                child.wait(timeout=10)
        database = root / 'state.sqlite3'
        with sqlite3.connect(database) as connection:
            assert connection.execute('SELECT schema_version FROM schema_meta').fetchone() == (183,)
            locator, before_deleted = connection.execute(
                'SELECT locator,deleted_at_ms FROM content_objects').fetchone()
            assert before_deleted is None
            if tombstoned:
                connection.execute('UPDATE content_objects SET deleted_at_ms=1')
                # Isolate tombstone backfill from the independent V183 write
                # recovery path, which could otherwise enqueue this same hash.
                connection.execute('DELETE FROM content_material_writes')
            assert not connection.execute("SELECT name FROM sqlite_master WHERE name='content_material_retirements'").fetchall()
        connection.close()
        blob = root / 'content' / locator
        assert blob.read_bytes() == b'hello'
        with (output / f'{name}-current.log').open('x') as log:
            result = subprocess.run(
                [str(current), '--exact', 'content::material::retirement::tests::upgrades_actual_previous_binary_store', '--ignored', '--nocapture'],
                env={**os.environ, 'MO_CONTENT_V183_ROOT': str(root), 'MO_CONTENT_V183_TOMBSTONE': str(tombstoned).lower()},
                stdout=log, stderr=subprocess.STDOUT, timeout=120, check=False)
        assert result.returncode == 0, name
        with sqlite3.connect(database) as connection:
            assert connection.execute('SELECT schema_version FROM schema_meta').fetchone() == (184,)
            assert connection.execute('SELECT count(*) FROM content_material_retirements').fetchone() == (0,)
            deleted, = connection.execute('SELECT deleted_at_ms FROM content_objects').fetchone()
            assert (deleted is not None) == tombstoned
        connection.close()
        if tombstoned:
            assert not blob.exists()
        else:
            assert blob.read_bytes() == b'hello'
        observations.append(dict(mode=name, fromVersion=183, toVersion=184,
            confirmedLiveBeforeKill=True, actuallyKilled=True, databaseCreatedByPriorBinary=True,
            independentTombstoneBackfill=tombstoned, publishedPreserved=not tombstoned,
            tombstonedBlobReclaimed=tombstoned, durableRetirementsAfter=0))
report = dict(priorExecutable=entry(old), currentExecutable=entry(current), observations=observations)
(output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(previousVersion=183, upgradedVersion=184, actualDatabases=len(observations), hardKills=2)))
