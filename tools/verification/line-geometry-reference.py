"""Independent Fraction arithmetic and scalar L2 reconstruction for geometry.

Consumes upstream-checked glyphs/metrics, not a second font engine. Verifies the
specified Q32 quantization profile exactly, and bounds its wire error against
unquantized rational geometry. Does not certify Office/WPS layout semantics.
"""
import argparse
import hashlib
import json
from fractions import Fraction as F
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--paragraph-layout', action='store_true')
args = parser.parse_args()
ROOT = Path('.codex-work/paragraph-layout' if args.paragraph_layout else '.codex-work/line-geometry')
UNIT = 1 << 32


from line_geometry_math import nearest, reference


def verify_wire(actual, fixed, exact, limit):
    global coordinates, maximum_error
    if isinstance(actual, dict):
        assert actual.keys() == fixed.keys() == exact.keys()
        for key in actual:
            verify_wire(actual[key], fixed[key], exact[key], limit)
    elif isinstance(actual, list):
        assert len(actual) == len(fixed) == len(exact)
        for values in zip(actual, fixed, exact):
            verify_wire(*values, limit)
    elif isinstance(actual, str):
        assert int(actual) == nearest(F(fixed)), (actual, fixed)
        error = abs(F(int(actual)) - exact)
        assert error <= limit, (error, limit)
        maximum_error = max(maximum_error, error)
        coordinates += 1
    else:
        assert actual == fixed == exact


report = json.loads((ROOT / 'parity.json').read_text())
coordinates, maximum_error, layouts, lines, glyphs = 0, F(0), 0, 0, 0
for case in report['cases']:
    files = {k: Path(case[k + 'Path']).read_bytes() for k in ('request', 'response')}
    for k, data in files.items():
        assert hashlib.sha256(data).hexdigest() == case[k + 'Sha256']
    response = json.loads(files['response'])
    if response['status'] != 'evaluated':
        continue
    q, r = json.loads(files['request']), response['result']
    if args.paragraph_layout:
        r = r['geometry']
        q = {'shaping': {'paragraph': q['paragraph']}, 'styles': q['styles'], 'strutStyle': q['strutStyle'], 'spacing': q['spacing']}
    if r is None or r['layout'] is None:
        continue
    layout = r['layout']
    # Conservative error budget: Q32 metric/extents/half-leading and cumulative
    # line origins, plus every selected fragment contribution and final 0.5 EMU.
    count = len(layout['lines']) + sum(len(l['visualFragments']) for l in layout['lines'])
    limit = F(1, 2) + F(5 * (count + 1), UNIT)
    verify_wire(layout, reference(q, r, True), reference(q, r, False), limit)
    layouts += 1
    lines += len(layout['lines'])
    glyphs += sum(len(l['glyphs']) for l in layout['lines'])
result = {'format': 'musteroffice.line-geometry-reference/1', 'layouts': layouts, 'lines': lines, 'glyphs': glyphs,
          'coordinates': coordinates, 'maximumWireErrorEmu': str(maximum_error),
          'q32ProfileExact': True, 'unquantizedRationalErrorBoundVerified': True,
          'scope': 'Independent Python Fraction geometry and scalar L2 reconstruction, using upstream-checked shaping/metrics; no independent font rasterization or Office/WPS layout comparison.'}
(ROOT / 'reference.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
