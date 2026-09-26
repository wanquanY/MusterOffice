"""Exact Fraction oracle independent of the Rust integer conversion algorithm.

Find the closest float32 among adjacent candidates using exact rational distance;
verify every local point, translation, and transformed-coordinate error bound.
The raster coverage oracle remains the separately defined component geometry.
"""
import hashlib
import json
import struct
from fractions import Fraction as F
from functools import lru_cache
from pathlib import Path
from stroke_reference import verify_styles

root = Path('.codex-work/path-raster')
U = 1 << 32
counts = dict(cases=0, localCoordinates=0, drawCoordinates=0, transformedCoordinates=0)
maximum = F(0)

def sha(b):
    return hashlib.sha256(b).hexdigest()

@lru_cache(maxsize=131072)
def exact(bits):
    return F(struct.unpack('<f', struct.pack('<I', bits))[0])

@lru_cache(maxsize=131072)
def nearest(value):
    if value == 0:
        return 0
    sign = 0x80000000 if value < 0 else 0
    value = abs(value)
    approx = struct.unpack('<I', struct.pack('<f', float(value)))[0]
    candidates = range(max(0, approx-1), approx+2)
    winner = min(candidates, key=lambda b: (abs(exact(b)-value), b & 1))
    return winner | sign

def coords(command):
    return [int(command[k][axis]) for k in ['control', 'control1', 'control2', 'to']
            if k in command for axis in ['x', 'y']]

def verify(q, r, words):
    global maximum
    v = q['viewport']
    factor = F(v['scale']['numerator'], v['scale']['denominator'] * U)
    paths = q['paths']
    total = sum(len(p['commands']) for p in paths)
    rgba = lambda a: int.from_bytes(bytes(a), 'little')
    assert words[:10] == (0x4d4f534b, 4, v['width'], v['height'], rgba(v['background']),
                          len(paths), len(q['draws']), total, r['info']['work']['strokeStyles'], 0)
    offset = 10
    anchors, compiled = [], []
    for p in paths:
        commands = p['commands']
        anchor = coords(commands[0]) if commands else [0, 0]
        anchors.append(anchor)
        assert words[offset:offset+2] == (int(p['fillRule'] == 'evenodd'), len(commands))
        offset += 2
        rendered = []
        for c in commands:
            values = coords(c)
            expected = [nearest((raw-anchor[k % 2])*factor) for k, raw in enumerate(values)]
            op = {'move': 1, 'line': 2, 'quadratic': 3, 'cubic': 4, 'close': 5}[c['kind']]
            assert words[offset:offset+7] == tuple([op]+expected+[0]*(6-len(expected)))
            offset += 7
            rendered.append(expected)
            counts['localCoordinates'] += len(expected)
        compiled.append(rendered)
    offset, paints, stroke_summary = verify_styles(q['draws'], v['scale'], words, offset, r['info']['work'])
    max_error = F(0)
    # Repeated draws/points retain full work accounting but share exact arithmetic.
    @lru_cache(maxsize=131072)
    def error(raw, local, delta, device):
        return abs(exact(nearest(exact(local)+exact(device)))-(raw+delta)*factor)*U
    for d, paint in zip(q['draws'], paints, strict=True):
        path = d['path']
        commands, anchor = paths[path]['commands'], anchors[path]
        translation = [int(d['origin'][a])+anchor[k]-int(v['origin'][a])
                       if commands else 0 for k, a in enumerate(['x', 'y'])]
        device = [nearest(t*factor) for t in translation]
        assert words[offset:offset+6] == (path, *device, rgba(d['brush']['rgba']), paint, 0)
        offset += 6
        counts['drawCoordinates'] += 2
        for c, rendered in zip(commands, compiled[path]):
            for k, raw in enumerate(coords(c)):
                e = error(raw, rendered[k], translation[k % 2]-anchor[k % 2], device[k % 2])
                max_error = max(max_error, e)
                counts['transformedCoordinates'] += 1
    assert offset == len(words)
    work = r['info']['work']
    assert work['paths'] == len(paths) and work['commands'] == total
    assert work['draws'] == len(q['draws'])
    assert work['drawnCommands'] == sum(len(paths[d['path']]['commands']) for d in q['draws'])
    bound = int(work['coordinateErrorBound'])
    assert max_error <= bound <= max_error+2, (bound, max_error)
    assert bound <= int(v['coordinateTolerance'])
    maximum = max(maximum, max_error)
    counts['cases'] += 1
    return dict(exactMaximumErrorRawQ32=str(max_error), boundRawQ32=str(bound), stroke=stroke_summary)

records = []
for c in json.loads((root/'parity.json').read_text())['cases']:
    if c['status'] != 'rendered':
        continue
    data = {k: Path(c[k+'Path']).read_bytes() for k in ['request', 'response', 'frame', 'pixels']}
    for k, v in data.items():
        assert sha(v) == c[k+'Sha256']
    q, r = json.loads(data['request']), json.loads(data['response'])
    words = struct.unpack('<'+'I'*(len(data['frame'])//4), data['frame'])
    result = verify(q, r, words)
    if c['name'] == 'double-rounding-counterexample':
        assert words[20] == 0x3f800001
        raw = int(q['paths'][0]['commands'][1]['to']['x'])
        approximate = struct.unpack('<I', struct.pack('<f', float(F(raw, 0xffffffff*U))))[0]
        assert approximate == 0x3f800000
    records.append(dict(name=c['name'], frameSha256=c['frameSha256'], **result))
report = {
    'format': 'musteroffice.path-raster-reference/1', 'counts': counts,
    'maximumExactErrorRawQ32': str(maximum), 'doubleRoundingCounterexampleVerified': True,
    'cases': records,
    'scope': 'Exact rational coordinate quantization and transformed control error; not an independent raster coverage or target application visual oracle.'}
(root/'reference.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(counts))
