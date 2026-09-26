"""Inspect real persisted bytes independently and upgrade frozen v2 copies."""
from contextlib import closing
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import xml.etree.ElementTree as ET
import zipfile

root = Path('.codex-work/job-results')
parent_path = Path('docs/reviews/evidence/2026-09-26-resource-host-verification.json')
parent = json.loads(parent_path.read_text())
assert hashlib.sha256(parent_path.read_bytes()).hexdigest() == 'b654adc6fbdd035ebb6f55e46460295d5d4855ae188e177c17c0b79d7d5fea66'
frozen = {r['path']: r for r in parent['artifacts']}

def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())

def snapshot(path):
    with closing(sqlite3.connect(f'file:{path}?mode=ro&immutable=1', uri=True)) as db:
        assert db.execute('PRAGMA integrity_check').fetchall() == [('ok',)]
        assert db.execute('PRAGMA foreign_key_check').fetchall() == []
        return {t: db.execute(f'SELECT * FROM {t} ORDER BY 1,2,3').fetchall()
                for t in ['jobs', 'revisions', 'heads', 'asset_uploads', 'asset_chunks', 'assets']}

calls = []
def run(name, argv, data=None):
    result = subprocess.run(argv, input=data, capture_output=True, timeout=60)
    (root/(name+'.stdout')).write_bytes(result.stdout)
    (root/(name+'.stderr')).write_bytes(result.stderr)
    calls.append(dict(name=name, argv=argv, exitCode=result.returncode,
                      stdout=entry(root/(name+'.stdout')), stderr=entry(root/(name+'.stderr'))))
    assert result.returncode == 0 and not result.stderr, (name, result.stderr)
    return result.stdout

out = root/'pptx'
out.mkdir()
prior_db = Path('.codex-work/resource-host/pptx/host.sqlite')
prior_report = Path('.codex-work/resource-host/pptx/result.json')
assert entry(prior_db) == frozen[str(prior_db)]
assert entry(prior_report) == frozen[str(prior_report)]
before = snapshot(prior_db)
shutil.copyfile(prior_db, out/'host.sqlite')
shutil.copyfile(prior_report, out/'input-bindings.json')
run('private-output', ['target/release/examples/job_output', str(out)])
report = json.loads((out/'result.json').read_text())
after = snapshot(out/'host.sqlite')
assert all(set(before[t]) <= set(after[t]) for t in before)
assert all(before[t] == after[t] for t in before if t != 'jobs')
data = (out/'diagnostic-copy.pptx').read_bytes()
baseline = Path('.codex-work/resource-host/pptx/resource-backed.pptx')
assert data == baseline.read_bytes()
assert report['storedSha256'] == hashlib.sha256(data).hexdigest()
assert report['storedBytes'] == len(data)
assert report['writeReceipt'] == dict(sha256=report['storedSha256'], byteLength=len(data))
with closing(sqlite3.connect(out/'host.sqlite')) as db:
    assert db.execute('PRAGMA user_version').fetchone() == (3,)
    rows = db.execute('SELECT chunk_index,data,sha256 FROM result_chunks ORDER BY chunk_index').fetchall()
    assert [r[0] for r in rows] == list(range(len(rows)))
    assert b''.join(r[1] for r in rows) == data
    assert all(hashlib.sha256(r[1]).hexdigest() == r[2] and 0 < len(r[1]) <= 262144 for r in rows)
    scope, principal, job_id, fence, name, request, executor, info, reserved, expiry = db.execute('SELECT * FROM result_spools').fetchone()
    info = json.loads(info)
    assert info['state'] == 'Sealed' and info['digest'] == report['storedSha256']
    assert info['received'] == reserved == len(data) and info['spec']['max_bytes'] == 1_000_000
    assert job_id == report['job']['id'] and request == report['job']['requestDigest']
    assert executor == report['job']['executorDigest'] and fence == int(report['job']['fence'])
    assert report['job']['state'] == 'running' and report['job']['result'] is None
    assert db.execute('SELECT count(*) FROM assets').fetchone()[0] == len(before['assets'])
    assert db.execute('SELECT count(*) FROM heads').fetchone() == (0,)
shutil.copyfile(out/'host.sqlite', out/'candidate.sqlite')
with zipfile.ZipFile(out/'diagnostic-copy.pptx') as archive:
    assert archive.testzip() is None
    xml_parts = [n for n in archive.namelist() if n.endswith(('.xml', '.rels'))]
    for n in xml_parts:
        ET.fromstring(archive.read(n))
    slides = [n for n in archive.namelist() if n.startswith('ppt/slides/slide') and n.endswith('.xml')]
    assert len(slides) == 2
    media = {n: hashlib.sha256(archive.read(n)).hexdigest() for n in archive.namelist() if n.startswith('ppt/media/')}
    assert set(media.values()) == {a['descriptor']['sha256'] for a in json.loads(prior_report.read_text())['assets']}

# Real host process must expire this deliberately old private lease; it must
# never expose the staged output through a public asset or success receipt.
expired = json.loads(run('expired-job', ['target/release/mo-host', str(out/'host.sqlite'), principal, scope], json.dumps(dict(operation='getJob', jobId=job_id)).encode()+b'\n'))
assert expired['outcome'] == 'failed' and expired['job']['state'] == 'failed'
assert expired['job']['id'] == job_id
assert expired['error']['code'] == expired['job']['result']['error']['code'] == 'EXECUTION_INTERRUPTED'
with closing(sqlite3.connect(out/'host.sqlite')) as db:
    assert db.execute('SELECT count(*) FROM result_chunks').fetchone() == (0,)
    assert db.execute('SELECT sum(reserved_bytes) FROM result_spools').fetchone() == (0,)
    assert db.execute('PRAGMA integrity_check').fetchall() == [('ok',)]

# Also migrate a real multi-chunk v2 asset database, including range reads over
# two old chunk boundaries, without rewriting any of its six existing tables.
prior_large = Path('.codex-work/resource-host/resources.sqlite')
assert entry(prior_large) == frozen[str(prior_large)]
large_before = snapshot(prior_large)
upgraded = root/'upgraded-v2.sqlite'
assert not upgraded.exists()
shutil.copyfile(prior_large, upgraded)
with closing(sqlite3.connect(upgraded)) as db:
    scope, asset_id, principal, _, _ = db.execute('SELECT * FROM assets').fetchone()
offset, length = 262144-9, 262144+73
download = run('migrated-range', ['target/release/mo-host', str(upgraded), principal, scope, 'read-asset', asset_id, str(offset), str(length)])
source = Path('.codex-work/resource-host/synthetic-resource.bin')
assert download == source.read_bytes()[offset:offset+length]
assert snapshot(upgraded) == large_before
with closing(sqlite3.connect(upgraded)) as db:
    assert db.execute('PRAGMA user_version').fetchone() == (3,)
assert entry(prior_db) == frozen[str(prior_db)] and entry(prior_large) == frozen[str(prior_large)]

evidence = dict(format='musteroffice.job-output-reference/1', calls=calls,
    inputs=[entry(p) for p in [prior_db, prior_report, prior_large, source, baseline]],
    outputs=[entry(p) for p in [out/'result.json', out/'candidate.sqlite', out/'host.sqlite', out/'diagnostic-copy.pptx', upgraded]],
    programs=[entry(p) for p in ['target/release/examples/job_output', 'target/release/mo-host']],
    privateCandidateMatchesPriorFile=True, persistedChunkBytesMatch=True, noPublicAssetOrJobSuccess=True,
    expiryReleasesCandidate=True, twoV2MigrationsPreserveRows=True, migratedRangeBytes=length,
    slides=len(slides), xmlParts=len(xml_parts), mediaHashes=media, zipCrc=True,
    limitations=['Library candidate under a running mutation lease, not a public export job or bundle.',
                 'Independent SQLite, byte, ZIP and XML checks; no new XSD, pixels, Office/WPS or product acceptance.',
                 'No new performance or installation-size measurement.'])
(root/'reference.json').write_text(json.dumps(evidence, indent=2)+'\n')
print(json.dumps({k:v for k,v in evidence.items() if k not in ['calls','inputs','outputs','programs']}))
