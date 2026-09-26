"""Independent rational layout for cluster spacing, baseline and line-spacing requests."""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
from line_geometry_math import reference, nearest

ROOT = Path('.codex-work/character-spacing')
report = json.loads((ROOT / 'geometry.json').read_text())
coordinates = layouts = lines = glyphs = 0
maximum = F(0)


def read(record):
    b = Path(record['path']).read_bytes()
    assert len(b) == record['byteLength'] and hashlib.sha256(b).hexdigest() == record['sha256']
    return json.loads(b)


def verify(actual, fixed, exact, bound):
    global coordinates, maximum
    if isinstance(actual, dict):
        assert actual.keys() == fixed.keys() == exact.keys()
        for key in actual:
            verify(actual[key], fixed[key], exact[key], bound)
    elif isinstance(actual, list):
        assert len(actual) == len(fixed) == len(exact)
        for a, f, e in zip(actual, fixed, exact):
            verify(a, f, e, bound)
    elif isinstance(actual, str):
        assert int(actual) == nearest(F(fixed))
        error = abs(F(int(actual)) - exact)
        assert error <= bound
        maximum = max(maximum, error)
        coordinates += 1
    else:
        assert actual == fixed == exact


for c in report['cases']:
    if not c['success']:
        continue
    q, r = read(c['request']), read(c['response'])['result']
    layout = r['layout']
    count = len(layout['lines']) + sum(len(line['visualFragments']) for line in layout['lines'])
    verify(layout, reference(q, r, True), reference(q, r, False), F(1, 2) + F(5 * (count + 1), 1 << 32))
    layouts += 1
    lines += len(layout['lines'])
    glyphs += sum(len(line['glyphs']) for line in layout['lines'])
assert (layouts, lines, glyphs, coordinates) == (25, 26, 44, 321)
result = dict(format='musteroffice.character-spacing-numerics/1', layouts=layouts, lines=lines,
              glyphs=glyphs, coordinates=coordinates, maximumWireErrorEmu=str(maximum),
              q32ProfileExact=True, rationalErrorBoundVerified=True)
(ROOT / 'numerics.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
