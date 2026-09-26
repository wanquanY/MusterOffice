"""Capture owned requests through the hash-verified historical Native CLI.

Usage: python3 tools/verification/manifest-layout-baseline.py <new-output.json>
The output must not exist. Current example output supplies requests only;
expected results come from the frozen text-body runtime executable.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path('.codex-work/manifest-layout')
assert len(sys.argv) == 2
output = Path(sys.argv[1])
assert not output.exists(), output
seal = json.loads(Path('docs/reviews/evidence/2026-09-25-text-body-verification.json').read_text())
for item in seal['artifacts'].values():
    data = Path(item['path']).read_bytes()
    assert len(data) == item['byteLength'] and hashlib.sha256(data).hexdigest() == item['sha256']
requests = json.loads((root / 'native-paths.json').read_text())['operations']
assert len(requests) == 20
font = Path('fixtures/fonts/owned.ttf')
bindings = json.loads(Path('fixtures/fonts/manifest-paragraph.json').read_text())['manifest']['fonts']
results = []
with tempfile.TemporaryDirectory(prefix='baseline-', dir=root) as temporary:
    path = Path(temporary) / 'input.json'
    for case in requests:
        request = case['request']
        assert request['layout']['paragraph']['fonts'] == bindings
        path.write_text(json.dumps(request))
        r = subprocess.run(['target/release/mo-cli', 'paragraph-paths', str(path), str(font)],
                           capture_output=True, timeout=30)
        assert r.returncode == 0, r.stderr
        results.append(dict(request=request, response=json.loads(r.stdout)))
with output.open('x') as file:
    file.write(json.dumps(results) + '\n')
print(json.dumps(dict(cases=len(results), sha256=hashlib.sha256(output.read_bytes()).hexdigest())))
