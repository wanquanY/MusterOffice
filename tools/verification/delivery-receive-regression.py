"""Existing public API regressions and exact comparison to the frozen baseline."""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path('.codex-work/delivery-receive')
def entry(path):
    path = Path(path); data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())
def read(path): return json.loads(Path(path).read_text())
baseline_path = Path('docs/reviews/evidence/2026-09-26-native-scheduler-verification.json')
assert entry(baseline_path)['sha256'] == '6b464c30f4d184bfc9261000c9555a556a7c0972321d42a594c5e86ec75c51f1'
baseline = read(baseline_path)
record = baseline['reports']['parity']; assert entry(record['path']) == record
record = read(record['path'])['current']; assert entry(record['path']) == record
previous = read(record['path'])
reports = {}; artifacts = []
for name, script, extra in [
    ('authored', 'tools/verification/pptx-parity.mjs', []),
    ('source', 'tools/verification/pptx-source-parity.mjs', ['.codex-work/sealed-export/source-manifest.json']),
    ('editor', 'tools/verification/native-wasm-parity.mjs', []),
]:
    assert entry(script) == next(e for e in baseline['sourceFiles'] if e['path'] == script)
    path = root/f'regression-{name}.json'; err = root/f'regression-{name}.stderr'
    with path.open('x') as stdout, err.open('x') as stderr:
        run = subprocess.run(['node', script, str(root/'frozen/mo-cli'), str(root/'wasm-node/mo_wasm.js'), *extra],
            stdout=stdout, stderr=stderr, timeout=180)
    assert run.returncode == 0 and not err.read_bytes()
    current = read(path)
    old = previous['reports'][name]; assert entry(old['path']) == old
    old = read(old['path'])
    def normalize(cases):
        return [{k: v for k, v in case.items() if k not in ['request', 'output']} for case in cases]
    assert normalize(current['cases']) == normalize(old['cases']), name
    if name == 'source': assert normalize(current['outputs']) == normalize(old['outputs'])
    if 'artifactDirectory' in current:
        directory = Path(current['artifactDirectory'])
        assert directory.is_relative_to('.codex-work')
        artifacts += [entry(p) for p in sorted(directory.rglob('*')) if p.is_file()]
    reports[name] = dict(cases=current['passed'], report=entry(path), allPreviousResultsIdentical=True)
    print(json.dumps(dict(name=name, cases=current['passed'])), flush=True)
with (root/'regression.json').open('x') as f:
    json.dump(dict(baseline=entry(baseline_path), reports=reports, generatedArtifacts=artifacts,
        limitation='Previous file export, source edit and document APIs only; not new rendering or complete product acceptance.'), f, indent=2)
    f.write('\n')
