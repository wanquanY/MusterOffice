"""Independent Fraction affine inversion, tiling, sampling and SrcOver oracle.

Reads authored requests, not compiled matrices. Image planes are original test
assets. Integer rectangular path coverage is exact; this is not an AA-edge test.
"""
from fractions import Fraction as F
import hashlib
import json
import struct
from pathlib import Path

ROOT = Path('.codex-work/image-brush')
from image_reference import U, value, point, tile, premul, sample, entry

results = []; affine_corners = 0
manifest = json.loads((ROOT / 'cases/manifest.json').read_text())
for case in manifest['cases']:
    name = case['name']; prefix = ROOT / 'cases' / name
    q = json.loads(prefix.with_suffix('.json').read_text())
    pixels = prefix.with_suffix('.rgba').read_bytes()
    data = prefix.with_suffix('.bin').read_bytes()
    resources = []; offset = 0
    for image in q['images']:
        size = image['width']*image['height']*4
        resources.append(data[offset:offset+size]); offset += size
    viewport = q['raster']['viewport']; errors = []; compared = 0
    vx, vy = point(viewport['origin']); scale = F(viewport['scale']['numerator'], viewport['scale']['denominator'])
    for py in range(viewport['height']):
        for px in range(viewport['width']):
            dest = premul(viewport['background'], 'straight')
            x, y = vx + F(2*px+1, 2)/scale, vy + F(2*py+1, 2)/scale
            for draw in q['raster']['draws']:
                if draw['brush']['kind'] == 'gradient':
                    assert all(s['srgb'] == [0, 0, 0, 0] for s in draw['brush']['gradient']['stops'])
                    continue
                dx, dy = point(draw['origin'])
                if not (dx <= x < dx+16 and dy <= y < dy+16): continue
                b = draw['brush']['image']; a, c = point(b['xStep']); b1, d = point(b['yStep']); tx, ty = point(b['origin'])
                det = a*d - b1*c
                sx, sy = ((x-tx)*d - (y-ty)*b1)/det, (a*(y-ty) - c*(x-tx))/det
                source = sample(q['images'][b['resource']], resources[b['resource']], b, sx, sy)
                dest = [source[k] + dest[k]*(1-source[3]/255) for k in range(4)]
            offset = 4*(py*viewport['width'] + px)
            expected = [max(0, min(255, round(v))) for v in dest]
            actual = pixels[offset:offset+4]; delta = max(abs(a-b) for a, b in zip(expected, actual))
            # RGBA8 rounding and Skia's scalar filter/compositor can differ by
            # one unit. No geometric edge pixels are excluded or screenshot read.
            if delta > 1: errors.append(dict(x=px, y=py, actual=list(actual), expected=expected, delta=delta))
            compared += 1
    assert not errors, (name, errors[:8], len(errors))
    compiled = json.loads(prefix.with_suffix('.result.json').read_text())
    frame = compiled['frame']; cursor = 12
    for _ in range(frame[5]): cursor += 2 + 7*frame[cursor+1]
    cursor += 4*frame[8]
    for _ in range(frame[9]): cursor += 9 + 5*frame[cursor+4]
    cursor += 4*frame[10]
    brushes = [frame[cursor+10*i:cursor+10*(i+1)] for i in range(frame[11])]
    draw_start = cursor + 10*frame[11]
    bound = F(int(compiled['images']['coordinateErrorBound']), U)
    for i, draw in enumerate(q['raster']['draws']):
        if draw['brush']['kind'] != 'image': continue
        b = draw['brush']['image']; resource = q['images'][b['resource']]
        index = frame[draw_start+6*i+5] - frame[9] - 1
        words = brushes[index]; matrix = [F(struct.unpack('<f', struct.pack('<I', v))[0]) for v in words[4:]]
        a, c = point(b['xStep']); bb, d = point(b['yStep']); tx, ty = point(b['origin'])
        for x in [0, resource['width']]:
            for y in [0, resource['height']]:
                exact = [(a*x+bb*y+tx-vx)*scale, (c*x+d*y+ty-vy)*scale]
                actual = [matrix[0]*x+matrix[1]*y+matrix[2], matrix[3]*x+matrix[4]*y+matrix[5]]
                assert all(abs(a-b) <= bound for a, b in zip(exact, actual)), name
                affine_corners += 1
    results.append(dict(name=name, comparedPixels=compared, request=entry(prefix.with_suffix('.json')),
        pixels=entry(prefix.with_suffix('.rgba'))))
out = dict(format='musteroffice.image-brush-reference/1', method='exact Fraction affine, per-axis tile, premultiplied sampling, SrcOver',
    tolerancePerChannel=1, cases=results, affineCorners=affine_corners, comparedPixels=sum(r['comparedPixels'] for r in results))
(ROOT / 'reference.json').write_text(json.dumps(out, indent=2))
print(json.dumps(dict(cases=len(results), comparedPixels=out['comparedPixels'], affineCorners=affine_corners)))
