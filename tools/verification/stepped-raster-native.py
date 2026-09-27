"""Full byte/status comparison against pinned old component material."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--manifest', type=Path, required=True)
p.add_argument('--probe', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
p.add_argument('--budgets', default='4096,1,7')
p.add_argument('--cancel-at', default='0')
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
inputs = {}


def load(record):
    data = Path(record['path']).read_bytes()
    assert len(data) == record['byteLength'] and hashlib.sha256(data).hexdigest() == record['sha256']
    inputs[record['path']] = record['sha256']
    return data


cases = json.loads(a.manifest.read_text())['cases']
results = []
for case in cases:
    frame = load(case.get('frame', case.get('base')))
    if mutation := case.get('mutation'):
        if mutation['name'].startswith('truncated-'):
            frame = frame[:int(mutation['name'].split('-')[1]) * 4]
        elif 'index' in mutation:
            frame = bytearray(frame)
            struct.pack_into('<I', frame, mutation['index'] * 4, mutation['value'])
    pixels = load(case['currentPixels'])
    images = load(case['images']) if 'images' in case else None
    incoming = struct.pack('<I', len(frame)//4) + frame
    if images is not None:
        incoming += struct.pack('<I', len(images)) + images
    for budget in a.budgets.split(','):
        for cancel_at in a.cancel_at.split(','):
            result = subprocess.run([str(a.probe), str(int(images is not None)), budget, cancel_at], input=incoming,
                                    capture_output=True, timeout=60)
            stem = a.output / f'{len(results):04}'
            stem.with_suffix('.out').write_bytes(result.stdout)
            stem.with_suffix('.log').write_bytes(result.stderr)
            assert result.returncode == 0, (case['name'], result.returncode, result.stderr)
            status, count = struct.unpack('<II', result.stdout[:8])
            assert len(result.stdout) == count + 8
            assert status == case['status'], (case['name'], budget, status, case['status'])
            assert result.stdout[8:] == pixels, (case['name'], budget, cancel_at, 'pixel mismatch')
            metrics = json.loads(result.stderr)
            results.append(dict(name=case['name'], budget=int(budget), cancelAt=int(cancel_at), status=status,
                                pixelSha256=hashlib.sha256(pixels).hexdigest(), byteLength=len(pixels), metrics=metrics))
            print(json.dumps(dict(index=len(results), name=case['name'], budget=budget)), flush=True)
for name, digest in inputs.items():
    assert hashlib.sha256(Path(name).read_bytes()).hexdigest() == digest
report = dict(format='musteroffice.stepped-raster-native/1', status='passed', inputs=inputs, cases=results,
              probeSha256=hashlib.sha256(a.probe.read_bytes()).hexdigest(),
              scope='Component status and complete pixel bytes; existing owned fixtures, not full PPT/product performance acceptance.')
(a.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
