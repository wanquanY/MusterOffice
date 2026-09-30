"""Independent Decimal/Taylor oracle against original decimal data, not Q32 angles.

Usage: python3 chart-geometry-oracle.py config.json parity-report.json
Sampling is regression evidence, not the proof of the analytic Hermite remainder.
"""
import hashlib
import json
import sys
from decimal import Decimal, localcontext
from fractions import Fraction
from pathlib import Path

D = Decimal
UNIT = D(2) ** 32


def atan_inverse(n):
    x = D(1) / n
    square = -x * x
    term = total = x
    for k in range(1, 180):
        term *= square
        total += term / (2 * k + 1)
    return total


def sin_cos(angle, tau):
    angle %= tau
    if angle > tau / 2:
        angle -= tau
    if angle < -tau / 2:
        angle += tau
    square = -angle * angle
    sine = st = angle
    cosine = ct = D(1)
    for k in range(1, 100):
        st *= square / ((2*k) * (2*k+1))
        ct *= square / ((2*k-1) * (2*k))
        sine += st
        cosine += ct
    return sine, cosine


def point(p):
    return D(p['x']) / UNIT, D(p['y']) / UNIT


def curve(points, t):
    u = 1 - t
    weights = [u**3, 3*u*u*t, 3*u*t*t, t**3]
    return [sum(p[axis]*w for p, w in zip(points, weights)) for axis in range(2)]


def decimal(f):
    return D(f.numerator) / D(f.denominator)


def verify(config, report):
    tau = 2 * (16 * atan_inverse(5) - 4 * atan_inverse(239))
    count = curves = samples = 0
    for item, record in zip(config['cases'], report['cases'], strict=True):
        raw = Path(item['request']).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == record['requestSha256']
        assert item['name'] == record['name']
        response = record['response']
        if response['status'] != 'computed':
            continue
        request = json.loads(raw)
        weights = [abs(Fraction(p['value'].strip())) for p in request['sectors']['weights']]
        total = sum(weights)
        before = Fraction(0)
        start = Fraction(int(request['sectors']['startTurn']), 2**32)
        sign = 1 if request['sectors']['direction'] == 'clockwise' else -1
        center = point(request['center'])
        for weight, definition, output in zip(weights, request['sectors']['weights'], response['geometry']['paths'], strict=True):
            assert definition['pointIndex'] == output['pointIndex']
            count += 1
            if weight == 0:
                assert not output['commands']
                continue
            a = start + sign * before / total
            before += weight
            b = start + sign * before / total
            arcs, current, pen = [], [], None
            for command in output['commands']:
                kind = command['kind']
                if kind == 'move':
                    assert not current
                    pen = point(command['to'])
                elif kind == 'cubic':
                    current.append([pen, point(command['control1']), point(command['control2']), point(command['to'])])
                    pen = current[-1][-1]
                elif kind in ('close', 'line'):
                    if current:
                        arcs.append(current)
                        current = []
                    if kind == 'line':
                        pen = point(command['to'])
                else:
                    raise AssertionError(kind)
            assert not current
            assert len(arcs) == (2 if int(request['innerRadius']) else 1)
            assert sum(map(len, arcs)) == output['arcSegments']
            bound = D(output['coordinateErrorBound']) / UNIT
            assert int(output['coordinateErrorBound']) == sum(int(output[k]) for k in ('numericErrorBound', 'curveErrorBound', 'angularErrorBound'))
            assert int(output['coordinateErrorBound']) <= int(request['coordinateTolerance'])
            for i, arc in enumerate(arcs):
                first, last = (a, b) if i == 0 else (b, a)
                radius = D(request['outerRadius'] if i == 0 else request['innerRadius']) / UNIT
                for j, controls in enumerate(arc):
                    curves += 1
                    for s in range(9):
                        t = D(s) / 8
                        fraction = (D(j) + t) / len(arc)
                        angle = (decimal(first) + decimal(last-first)*fraction)*tau
                        sine, cosine = sin_cos(angle, tau)
                        expected = [center[0]+radius*sine, center[1]-radius*cosine]
                        actual = curve(controls, t)
                        for axis in range(2):
                            assert abs(expected[axis]-actual[axis]) <= bound + D('1e-75'), (item['name'], i, j, s, axis)
                        samples += 1
    return dict(profile='chart-geometry-independent-decimal-oracle/1', sectors=count, cubics=curves, sampledPoints=samples, decimalPrecision=90, originalDecimalAngles=True, allBoundsEncloseSamples=True)


if __name__ == '__main__':
    with localcontext() as context:
        context.prec = 90
        print(json.dumps(verify(json.loads(Path(sys.argv[1]).read_text()), json.loads(Path(sys.argv[2]).read_text()))))
