"""Reuse the frozen workspace suite in a fresh stage and build its actual CLI."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path('.codex-work/operation-discovery')
root.mkdir(exist_ok=True)
parent = json.loads(Path('docs/reviews/evidence/2026-09-26-export-host-verification.json').read_text())
source_path = Path('tools/verification/export-host-workspace.py')
record = next(r for r in parent['sourceFiles'] if r['path'] == str(source_path))
assert hashlib.sha256(source_path.read_bytes()).hexdigest() == record['sha256']
source = source_path.read_text()
old = "root = Path('.codex-work/export-host')"
assert source.count(old) == 1
source = source.replace(old, "root = Path('.codex-work/operation-discovery')")
replay = root/'workspace-replay.py'
if replay.exists(): assert replay.read_text() == source
else:
    with replay.open('x') as f: f.write(source)
start = sys.argv[1] if len(sys.argv) > 1 else 'fmt'
if start != 'release-build':
    run = subprocess.run([sys.executable,str(replay),start])
    if run.returncode: raise SystemExit(run.returncode)
env = dict(os.environ,CARGO_BUILD_JOBS='2',MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
attempt = 1
while (root/f'release-build-{attempt}.log').exists(): attempt += 1
log = root/f'release-build-{attempt}.log'
argv = ['cargo','build','--release','--locked','--offline','-p','mo-host','-p','mo-raster-worker']
with log.open('x') as f:
    run = subprocess.run(argv,env=env,stdout=f,stderr=subprocess.STDOUT)
records = json.loads((root/'workspace.json').read_text())
record = dict(name='release-build',argv=argv,exitCode=run.returncode,log=str(log))
records.append(record)
(root/'workspace.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps(record),flush=True)
raise SystemExit(run.returncode)
