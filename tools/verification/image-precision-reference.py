"""Independent exact-rational checks of source/encoded image coordinate drift.

This does not implement the production interval/extent algorithm. It transports
corresponding points between source and encoded texture lattices at viewport
samples, across all corners of each declared upstream uncertainty box.
"""
import hashlib
import itertools
import json
from fractions import Fraction as F
from pathlib import Path
import struct

ROOT = Path('.codex-work/image-paint')
Q = 1 << 32

def entry(p):
    b = Path(p).read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())

def read(r):
    assert entry(r['path']) == r
    return Path(r['path']).read_bytes()

def matrix(b, v, errors=False):
    x, y, o = b['xStep'], b['yStep'], b['origin']
    a = [x['x'], y['x'], o['x'], x['y'], y['y'], o['y']]
    a = [F(int(x), Q) for x in a]
    if not errors:
        a[2] -= F(int(v['origin']['x']), Q)
        a[5] -= F(int(v['origin']['y']), Q)
    return [x * F(v['scale']['numerator'], v['scale']['denominator']) for x in a]

def inverse(m, p):
    det = m[0]*m[4]-m[1]*m[3]
    assert det != 0
    x, y = p[0]-m[2], p[1]-m[5]
    return ((m[4]*x-m[1]*y)/det, (m[0]*y-m[3]*x)/det)

def apply(m, p):
    return (m[0]*p[0]+m[1]*p[1]+m[2], m[3]*p[0]+m[4]*p[1]+m[5])

def corners(values, errors):
    return itertools.product(*[(v-e, v+e) if e else (v,) for v, e in zip(values, errors)])

report = json.loads((ROOT/'precision.json').read_text())
records = []
checks = 0
maximum = F(0)
for c in report['cases']:
    if c['mode'] == 1:
        continue  # all-zero uncertainty is separately byte-equal in runtime test
    q = json.loads(read(c['request']))['raster']
    response = json.loads(read(c['response']))
    bound = F(int(response['info']['images']['coordinateErrorBound']), Q)
    v = q['viewport']
    b = q['draws'][0]['brush']['image']
    raw = read(c['frame'])
    words = struct.unpack('<'+'I'*(len(raw)//4), raw)
    size = 14 if words[1] == 6 else 10
    assert words[1] in [5, 6]
    brush = words[-6-size:-6]
    floats = [F(struct.unpack('<f', struct.pack('<I', w))[0]) for w in brush[4:]]
    encoded = floats[:6]
    domain = floats[6:] if size == 14 else list(map(F, [0, 0, 2, 2]))
    source = matrix(b, v)
    empty = dict(origin=dict(x='0', y='0'), xStep=dict(x='0', y='0'), yStep=dict(x='0', y='0'), sourceDomain=['0']*4)
    e = b.get('uncertainty', empty)
    errors = matrix(e, v, True)
    d = b.get('sourceDomain')
    original = [F(int(d[k]), Q) for k in ['left', 'top', 'right', 'bottom']] if d else list(map(F, [0, 0, 2, 2]))
    de = [F(int(x), Q) for x in e['sourceDomain']]
    # Every uncertainty corner; source-domain corners use paired left/right
    # signs for each axis, plus all mixed signs across the two axes.
    domains = list(corners(original, de))
    points = list(itertools.product([-1, F(v['width'], 2), v['width']+1], [-1, F(v['height'], 2), v['height']+1]))
    local_max = F(0)
    count = 0
    for actual in corners(source, errors):
        for point in points:
            # Both inverse directions check movement at the visible boundary.
            for reverse in [False, True]:
                a, z = (actual, encoded) if reverse else (encoded, actual)
                coords = inverse(a, point)
                for od in domains:
                    da, dz = (od, domain) if reverse else (domain, od)
                    qa, qz = [], []
                    for axis, tile in enumerate([b['tileX'], b['tileY']]):
                        t = (coords[axis]-da[axis])/(da[axis+2]-da[axis])
                        if tile in ['clamp', 'decal']:
                            t = max(F(0), min(F(1), t))
                        qa.append(da[axis]+t*(da[axis+2]-da[axis]))
                        qz.append(dz[axis]+t*(dz[axis+2]-dz[axis]))
                    pa, pz = apply(a, qa), apply(z, qz)
                    delta = max(abs(x-y) for x, y in zip(pa, pz))
                    assert delta <= bound, (c['name'], float(delta), float(bound))
                    local_max = max(local_max, delta)
                    count += 1
    maximum = max(maximum, local_max)
    checks += count
    records.append(dict(name=c['name'], boundRaw=str(bound*Q), maximumDrift=str(local_max), comparisons=count))
result = dict(format='musteroffice.image-precision-reference/1', runtime=entry(ROOT/'precision.json'),
              cases=len(records), comparisons=checks, maximumSampleDrift=str(maximum),
              scope='Exact rational parameter/lattice comparisons, not a proof of filter output or pixel coverage.', records=records)
(ROOT/'precision-reference.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps(dict(cases=len(records), comparisons=checks, maximumSampleDriftPixels=float(maximum))))
