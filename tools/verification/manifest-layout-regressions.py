"""Re-execute affected existing text operations using the current Native worker.

Does not rewrite historical reports or claim new WASM parity. Special component
fault-injection scenarios outside each report's ordinary cases are not replayed.
"""
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import argparse

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output-dir', type=Path, default=Path('.codex-work/manifest-layout'))
ROOT = parser.parse_args().output_dir
ROOT.mkdir(parents=True, exist_ok=True)
OUT = ROOT / 'regressions'
OUT.mkdir(exist_ok=True)
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-text-body-verification.json')
WORKER = Path('target/debug/mo-text-worker')
old = json.loads(PREVIOUS.read_text())


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), sha256=hashlib.sha256(data).hexdigest(), byteLength=len(data))


groups = [
    ('itemizationAndParagraph', '--paragraph', 'paragraph-shape'),
    ('mixedFont', '--paragraph', None), ('lineShaping', '--lines', None),
    ('lineGeometry', '--geometry', None), ('paragraphLayout', '--layout', None),
    ('paragraphPaths', '--paths', None),
]
cases, reports, counts = [], [], {}
for name, mode, kind in groups:
    item = old['regressionReports'][name]['report']
    assert entry(item['path']) == item
    reports.append(item)
    report = json.loads(Path(item['path']).read_text())
    counts[name] = 0
    for ordinal, case in enumerate(report['cases']):
        if kind is not None and case['kind'] != kind:
            continue
        request = Path(case['requestPath']).read_bytes()
        bundle = Path(case['bundlePath']).read_bytes()
        response = Path(case['responsePath']).read_bytes()
        for k, data in [('request', request), ('bundle', bundle), ('response', response)]:
            assert hashlib.sha256(data).hexdigest() == case[k + 'Sha256']
        wire = struct.pack('<II', len(request), len(bundle)) + request + bundle
        done = subprocess.run([str(WORKER), mode], input=wire, capture_output=True, timeout=60)
        assert done.returncode == 0, (name, case['name'], done.stderr)
        assert len(done.stdout) >= 4 and struct.unpack_from('<I', done.stdout)[0] == len(done.stdout) - 4
        actual = done.stdout[4:]
        assert actual == response.rstrip(b'\n'), (name, case['name'], actual[:500], response[:500])
        path = OUT / f'{name}-{ordinal:03}.response.json'
        path.write_bytes(actual)
        cases.append(dict(group=name, name=case['name'], mode=mode, request=entry(case['requestPath']),
                          bundle=entry(case['bundlePath']), priorResponse=entry(case['responsePath']), response=entry(path)))
        counts[name] += 1
    print(f'{name}: {counts[name]} exact Native responses', flush=True)
assert sum(counts.values()) == 207, counts
result = dict(format='musteroffice.manifest-layout-native-regressions/1', scope=__doc__,
              worker=entry(WORKER), previousEvidence=entry(PREVIOUS), previousReports=reports,
              counts=counts, total=sum(counts.values()), cases=cases)
(ROOT / 'regressions.json').write_text(json.dumps(result, indent=2) + '\n')
