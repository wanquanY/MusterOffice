"""Finite coincident reversal against independent repeated-clock/static geometry.

The shared harness performs native export, independent OPC/XSD checks and real
raster comparisons. WPS negative-speed compatibility is a separate open gate.
"""
from fractions import Fraction
import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location('rate', Path(__file__).with_name('container-rate-fixtures.py'))
rate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rate)
NAMES = ['reverse-coincident', 'reverse-nested', 'reverse-clipped', 'reverse-easing']
REPEATS = [1, 2, 4, 1, 2, 4]


def timeline(name):
    nodes = []
    for i, count in enumerate(REPEATS):
        node = dict(id='spin-' + str(i), start=rate.at(0), duration=rate.f.time(4 // count),
                    repeatMilli=count*1000, fill='hold', effect=dict(kind='rotation',
                    target='shape:' + str(i), composition='absolute',
                    **{'from': 20*60000, 'to': (40+20*i)*60000}))
        if i >= 3:
            node['timeTransform'] = dict(speedMilliPercent=-100000, autoReverse=False,
                                         accelerationMilliPercent=0, decelerationMilliPercent=0)
        nodes.append(node)
    outer = rate.group('outer', -200000, [n['id'] for n in nodes], 1)
    containers, roots = [outer], ['outer']
    if name == 'reverse-nested':
        inner = rate.group('inner', -100000, outer['children'])
        inner['timeTransform']['accelerationMilliPercent'] = 100000
        outer['children'] = ['inner']
        containers.append(inner)
    elif name == 'reverse-clipped':
        clip = rate.group('clip', 100000, ['outer'])
        clip['duration'] = dict(kind='fixed', duration=rate.f.time(2))
        roots = ['clip']
        containers.append(clip)
    elif name == 'reverse-easing':
        outer['timeTransform']['accelerationMilliPercent'] = 50000
    return dict(format='musteroffice.timeline/0.2-draft', nodes=nodes,
                tree=dict(roots=roots, containers=containers))


def expected(name, index, time):
    if time < 1:
        return None, Fraction(20)
    end = Fraction(2 if name == 'reverse-clipped' else 3)
    p = (min(time, end)-1)/2
    q = 1-p
    if name == 'reverse-nested':
        q = p*p
    elif name == 'reverse-easing':
        q = Fraction(4, 3)*q*q if q < Fraction(1, 2) else Fraction(4, 3)*(q-Fraction(1, 4))
    if index >= 3:
        q = 1-q
    position = q*REPEATS[index]
    progress = position % 1
    backwards = (name != 'reverse-nested') != (index >= 3)
    if not progress and position > 0 and ((backwards and time == 1) or (not backwards and time >= end)):
        progress = Fraction(1)
    return progress, 20+(20+20*index)*progress


if __name__ == '__main__':
    rate.main(NAMES, timeline, expected, [Path(__file__)])
