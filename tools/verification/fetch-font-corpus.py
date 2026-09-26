"""Fetch pinned OFL test fonts; never reads system fonts. Assets stay outside runtime packages."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--directory', type=Path, default=Path('.codex-work/font-corpus'))
p.add_argument('--verify-only', action='store_true')
a = p.parse_args()
manifest = json.loads(Path('fixtures/fonts/upstream.json').read_text())
for item in manifest:
    assert Path(item['name']).name == item['name']
    target = a.directory / item['family'] / item['name']
    if not target.exists():
        if a.verify_only:
            raise SystemExit(f'missing {target}')
        response = subprocess.check_output(['curl', '--fail', '--silent', '--show-error',
            '--connect-timeout', '10', '--max-time', '180',
            f"https://api.github.com/repos/google/fonts/git/blobs/{item['gitBlobSha1']}"])
        data = base64.b64decode(json.loads(response)['content'])
    else:
        data = target.read_bytes()
    assert len(data) == item['byteLength'], target
    assert hashlib.sha256(data).hexdigest() == item['sha256'], target
    assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest() == item['gitBlobSha1'], target
    if not target.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as output:
            output.write(data)
    print(f"verified {item['family']}/{item['name']} {len(data)} bytes")
print('All font binaries and accompanying OFL notices verified; no system font fallback.')
