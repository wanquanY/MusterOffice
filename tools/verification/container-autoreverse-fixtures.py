"""Native two-leg activation and pixel controls, including nested and clipped turns."""
from fractions import Fraction as F
import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location('rate', Path(__file__).with_name('container-rate-fixtures.py'))
rate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rate)
NAMES = ['auto-parallel', 'auto-nested', 'auto-easing', 'auto-cut-turn', 'auto-cut-late']
REPEATS = [1, 2, 4, 1, 2, 4]
TIMES = ['0','0.5','1','1.25','1.5','2','2.5','2.75','3','3.25','3.5','4','4.5',
         '5','5.5','6','6.5','7','7.5','8','8.5','9','10','1/7','7/3','1.5']


def timeline(name):
    nodes = []
    for i, count in enumerate(REPEATS):
        node = dict(id='spin-' + str(i), start=rate.at(0), duration=rate.f.time(4//count),
                    repeatMilli=count*1000, fill='hold', effect=dict(kind='rotation',
                    target='shape:' + str(i), composition='absolute',
                    **{'from':20*60000, 'to':(40+20*i)*60000}))
        if i >= 3:
            node['timeTransform'] = dict(speedMilliPercent=-100000, autoReverse=False,
                                         accelerationMilliPercent=0, decelerationMilliPercent=0)
        nodes.append(node)
    outer = rate.group('outer', 200000, [n['id'] for n in nodes], 1)
    outer['timeTransform']['autoReverse'] = True
    containers, roots = [outer], ['outer']
    if name == 'auto-nested':
        inner = rate.group('inner', 100000, outer['children'])
        inner['timeTransform']['autoReverse'] = True
        outer['children'] = ['inner']
        outer['timeTransform']['speedMilliPercent'] = -200000
        containers.append(inner)
    elif name == 'auto-easing':
        outer['timeTransform']['accelerationMilliPercent'] = 100000
    elif name.startswith('auto-cut-'):
        clip = rate.group('clip', 100000, ['outer'])
        clip['duration'] = dict(kind='fixed', duration=rate.f.time(3 if name == 'auto-cut-turn' else 4))
        containers.append(clip)
        roots = ['clip']
    return dict(format='musteroffice.timeline/0.2-draft', nodes=nodes,
                tree=dict(roots=roots, containers=containers))


def clock(name, time):
    end = F({'auto-nested':9, 'auto-cut-turn':3, 'auto-cut-late':4}.get(name, 5))
    elapsed = max(F(0), (min(time, end)-1)/2)
    leg = int(elapsed)
    if time >= end and elapsed.denominator == 1 and leg > 0:
        leg -= 1
    progress = elapsed-leg if leg%2 == 0 else 1-(elapsed-leg)
    if name == 'auto-easing':
        progress *= progress
    return end, leg, progress


def expected(name, index, time):
    if time < 1:
        return None, F(20)
    end, leg, q = clock(name, time)
    backwards = (leg%2 == 1) != (index >= 3)
    if index >= 3:
        q = 1-q
    position = q*REPEATS[index]
    p = position%1
    entering = time == 1+2*leg
    if not p and position > 0 and ((backwards and entering) or (not backwards and time >= end)):
        p = F(1)
    return p, 20+(20+20*index)*p


def check_state(name, time, state):
    end, leg, _ = clock(name, time)
    start = F(1+2*leg)
    endpoint = min(end, start+2)
    assert len(state['nodes']) == 6
    for node in state['nodes']:
        assert node['start'] == rate.f.exact(start), (name,time,node)
        assert node['end'] == rate.f.exact(endpoint), (name,time,node)
        assert node['phase'] == ('scheduled' if time < 1 else 'frozen' if time >= end else 'active'), (name,time,node)


if __name__ == '__main__':
    rate.main(NAMES, timeline, expected, [Path(__file__)], TIMES, check_state)
