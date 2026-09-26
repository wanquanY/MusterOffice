"""Independent normalized-image reference shared by raster and scene tests."""
from fractions import Fraction as F
import hashlib

U = 1 << 32
def value(s): return F(int(s), U)
def point(p): return [value(p[k]) for k in ['x', 'y']]
def tile(v, n, mode):
    if mode == 'clamp': return max(0, min(n - 1, v))
    if mode == 'repeat': return v % n
    if mode == 'mirror':
        v %= 2*n
        return v if v < n else 2*n - v - 1
    assert mode == 'decal'
    return v if 0 <= v < n else None
def premul(p, alpha):
    return [F(v) if alpha == 'premultiplied' or k == 3 else F(v*p[3], 255) for k, v in enumerate(p)]
def sample(image, data, brush, x, y):
    w, h = image['width'], image['height']
    def at(i, j):
        i, j = tile(i, w, brush['tileX']), tile(j, h, brush['tileY'])
        if i is None or j is None: return [F(0)] * 4
        offset = 4*(j*w + i)
        return premul(data[offset:offset+4], image['alpha'])
    if brush['sampling'] == 'nearest': return at(x.numerator // x.denominator, y.numerator // y.denominator)
    assert brush['sampling'] == 'linear'
    x -= F(1, 2); y -= F(1, 2)
    i, j = x.numerator // x.denominator, y.numerator // y.denominator
    dx, dy = x - i, y - j
    result = [F(0)] * 4
    for px, wx in [(i, 1-dx), (i+1, dx)]:
        for py, wy in [(j, 1-dy), (j+1, dy)]:
            for k, v in enumerate(at(px, py)): result[k] += v*wx*wy
    return result
def entry(path):
    b = path.read_bytes()
    return dict(path=str(path), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())

