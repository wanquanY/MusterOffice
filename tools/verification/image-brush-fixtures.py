"""Original normalized pixels and exact affine requests, with no external assets."""
from copy import deepcopy
import hashlib
import json
from pathlib import Path

ROOT = Path('.codex-work/image-brush/cases')
ROOT.mkdir(parents=True, exist_ok=True)
U = 1 << 32
def fixed(v):
    return str(round(v * U))
def point(x, y):
    return dict(x=fixed(x), y=fixed(y))
def request(matrix=(4, 0, 4, 0, 4, 4), alpha='premultiplied', sampling='nearest', tiles=('clamp', 'clamp')):
    a, b, tx, c, d, ty = matrix
    path = dict(fillRule='nonzero', commands=[dict(kind=k, **({} if p is None else dict(to=point(*p))))
        for k, p in [('move', (0, 0)), ('line', (16, 0)), ('line', (16, 16)), ('line', (0, 16)), ('close', None)]])
    brush = dict(kind='image', image=dict(resource=0, origin=point(tx, ty), xStep=point(a, c),
        yStep=point(b, d), tileX=tiles[0], tileY=tiles[1], sampling=sampling))
    pixels = bytes([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 128, 64, 32, 128])
    if alpha == 'straight':
        pixels = bytes([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 128, 64, 128])
    q = dict(raster=dict(viewport=dict(width=16, height=16, origin=point(0, 0),
        scale=dict(numerator=1, denominator=1), coordinateTolerance=str(1 << 24), background=[0, 0, 0, 0]),
        paths=[path], draws=[dict(path=0, origin=point(0, 0), brush=brush)]),
        images=[dict(width=2, height=2, alpha=alpha, sha256=hashlib.sha256(pixels).hexdigest())])
    return q, pixels

def save(name, q, pixels, oracle=True):
    (ROOT / (name + '.json')).write_text(json.dumps(q))
    (ROOT / (name + '.bin')).write_bytes(pixels)
    return dict(name=name, oracle=oracle)

cases = []
for alpha in ['premultiplied', 'straight']:
    for sampling in ['nearest', 'linear']:
        for tile in ['clamp', 'repeat', 'mirror', 'decal']:
            q, b = request(alpha=alpha, sampling=sampling, tiles=(tile, tile))
            cases.append(save(f'{alpha}-{sampling}-{tile}', q, b))
for name, matrix in [('rotated', (0, -4, 12, 4, 0, 4)), ('flipped', (-4, 0, 12, 0, 4, 4)),
    ('sheared', (4, 2, 2, 0, 4, 4)), ('fractional', (2.5, 0.75, 2.25, -0.5, 3.5, 3.25)),
    ('quantized', (1/3, 0.7, 1/7, -0.1, 1.7, 3.1)),
    ('min-scale', (1/16384, 0, 0, 0, 1/16384, 0))]:
    q, b = request(matrix=matrix, tiles=('mirror', 'repeat'))
    cases.append(save(name, q, b))
q, b = request(); shift = 1 << 80
q['raster']['viewport']['origin'] = point(shift, -shift)
q['raster']['draws'][0]['origin'] = point(shift, -shift)
q['raster']['draws'][0]['brush']['image']['origin'] = point(shift+4, -shift+4)
cases.append(save('huge-origin', q, b))
q, b = request(); q['raster']['viewport']['background'] = [20, 80, 160, 255]
cases.append(save('composite', q, b))
q, b = request(); q['raster']['draws'][0]['origin'] = point(4, 4)
cases.append(save('placed-path', q, b))
q, b = request(); q['raster']['draws'].append(deepcopy(q['raster']['draws'][0]))
cases.append(save('reused', q, b))
q, b = request(); q['raster']['draws'][0]['brush']['image']['resource'] = 1
q['images'].append(deepcopy(q['images'][0])); b += b
cases.append(save('second-resource', q, b))
q, b = request(); q['raster']['draws'].append(dict(path=0, origin=point(0, 0), brush=dict(kind='gradient',
    gradient=dict(geometry=dict(kind='linear', start=point(0, 0), end=point(16, 0)),
      stops=[dict(position=0, srgb=[0, 0, 0, 0]), dict(position=1, srgb=[0, 0, 0, 0])],
      tile='clamp', interpolation='srgb', alpha='straight'))))
cases.append(save('mixed-gradient', q, b))
for alpha in ['straight', 'premultiplied']:
    q, b = request(alpha=alpha, sampling='linear')
    b = bytes([255, 0, 0, 255, 0, 0, 255 if alpha == 'straight' else 0, 0] * 2)
    q['images'][0]['sha256'] = hashlib.sha256(b).hexdigest()
    cases.append(save(alpha + '-hidden-color', q, b))
negative = []
for name in ['digest', 'length', 'premul', 'reference', 'singular', 'small-scale', 'corner-range', 'precision', 'dimensions']:
    q, b = request(); image = q['raster']['draws'][0]['brush']['image']
    if name == 'digest': q['images'][0]['sha256'] = '0' * 64
    if name == 'length': b = b[:-1]
    if name == 'premul': b = bytes([255, 0, 0, 0]) + b[4:]; q['images'][0]['sha256'] = hashlib.sha256(b).hexdigest()
    if name == 'reference': image['resource'] = 1
    if name == 'singular': image['yStep'] = image['xStep']
    if name == 'small-scale': image['xStep'] = point(1/32768, 0)
    if name == 'corner-range': image['xStep'] = point(20000, 0)
    if name == 'precision':
        image['origin'] = point(16000 + 1/4096, 0); q['raster']['viewport']['coordinateTolerance'] = '256'
    if name == 'dimensions': q['images'][0]['width'] = 8193
    negative.append(save('invalid-' + name, q, b, False))
(ROOT / 'manifest.json').write_text(json.dumps(dict(cases=cases, negative=negative), indent=2))
print(json.dumps(dict(cases=len(cases), negative=len(negative))))
