"""Execute frozen export/file/page parity at this stage's module and outputs."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
root=Path('.codex-work/operation-discovery')
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-export-host-verification.json').read_text())
script=Path('tools/verification/export-host-parity.py')
record=next(r for r in parent['sourceFiles'] if r['path']==str(script))
assert hashlib.sha256(script.read_bytes()).hexdigest()==record['sha256']
source=script.read_text()
assert source.count('.codex-work/export-host')==2
source=source.replace('.codex-work/export-host','.codex-work/operation-discovery')
replay=root/'parity-driver.py'
with replay.open('x') as f: f.write(source)
raise SystemExit(subprocess.run([sys.executable,str(replay),sys.argv[1]]).returncode)
