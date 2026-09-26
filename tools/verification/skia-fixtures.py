"""Owned path probes and exact-origin mapping of existing paragraph scenes.

This is a diagnostic viewport compiler, not the production Draw IR compiler.
It subtracts Q32 origins as integers, then scales with Fraction before the
deliberate float32 device-space boundary. No fonts or browser text are used.
"""
from fractions import Fraction
import copy
import hashlib
import json
from pathlib import Path
import random
import struct

directory = Path('.codex-work/skia/verification')
directory.mkdir(parents=True, exist_ok=True)
cases = []


def bits(x):
    return struct.unpack('<I', struct.pack('<f', float(x)))[0]


def cmd(op, *xy):
    return [op, *map(bits, xy), *([0] * (6 - len(xy)))]


def rect(x, y, w, h, reverse=False):
    points = [(x, y), (x+w, y), (x+w, y+h), (x, y+h)]
    if reverse:
        points.reverse()
    return [cmd(1, *points[0]), *[cmd(2, *v) for v in points[1:]], cmd(5)]


def frame(paths=(), draws=(), width=64, height=64, background=0, styles=()):
    r = [0x4d4f534b, 4, width, height, background, len(paths), len(draws),
         sum(len(p[1]) for p in paths), len(styles), 0]
    for fill, commands in paths:
        r += [fill, len(commands)]
        for c in commands:
            r += c
    for style in styles:
        r += list(style)
    for draw in draws:
        path, x, y, rgba, *paint = draw
        r += [path, bits(x), bits(y), rgba, paint[0] if paint else 0, 0]
    return r


def add(name, words, status=0, **metadata):
    data = struct.pack('<' + 'I' * len(words), *words)
    path = directory / (name + '.request.bin')
    path.write_bytes(data)
    cases.append({'name': name, 'requestPath': str(path), 'requestSha256': hashlib.sha256(data).hexdigest(),
                  'status': status, **metadata})


add('empty', frame(), oracle='empty')
add('background', frame(background=0xff332211), oracle='background')
add('transparent-background', frame(background=0x80332211), oracle='transparent-background')
add('solid', frame([(0, rect(8, 12, 30, 20))], [(0, 0, 0, 0xff0000ff)]), oracle='solid')
add('alpha-overlap', frame([(0, rect(8, 8, 32, 32)), (0, rect(24, 24, 32, 32))],
                          [(0, 0, 0, 0x800000ff), (1, 0, 0, 0x80ff0000)]), oracle='alpha-overlap')
for fill, reverse, name in [(0, False, 'winding-solid'), (0, True, 'winding-hole'), (1, False, 'evenodd-hole')]:
    add(name, frame([(fill, rect(8, 8, 48, 48) + rect(20, 20, 24, 24, reverse))],
                    [(0, 0, 0, 0xff0000ff)]), oracle=name)
add('translated-clip', frame([(0, rect(0, 0, 40, 40))], [(0, -8, -12, 0xff0000ff)]), oracle='translated-clip')
add('half-pixel', frame([(0, rect(8.5, 8, 30, 20))], [(0, 0, 0, 0xff0000ff)]), oracle='half-pixel')
add('empty-path', frame([(0, [])], [(0, 0, 0, 0xff0000ff)]), oracle='empty')
add('move-only', frame([(0, [cmd(1, 3, 5)])], [(0, 0, 0, 0xff0000ff)]), oracle='empty')
add('open-contour', frame([(0, rect(8, 12, 30, 20)[:-1])], [(0, 0, 0, 0xff0000ff)]), oracle='solid')
add('quad', frame([(0, [cmd(1, 4, 32), cmd(3, 32, -20, 60, 32), cmd(3, 32, 84, 4, 32), cmd(5)])], [(0, 0, 0, 0xff774422)]))
add('cubic', frame([(0, [cmd(1, 4, 4), cmd(4, 100, -40, -30, 80, 60, 60), cmd(2, 4, 60), cmd(5)])], [(0, 0, 0, 0xc0885533)]))
rng = random.Random(894523)
for i in range(64):
    commands = [cmd(1, rng.uniform(-8, 72), rng.uniform(-8, 72))]
    commands += [cmd(4, *[rng.uniform(-32, 96) for _ in range(6)]) for _ in range(6)] + [cmd(5)]
    add(f'curve-{i:02}', frame([(i % 2, commands)], [(0, 0, 0, 0xcf397faf)]))

base = frame([(0, rect(8, 12, 30, 20))], [(0, 0, 0, 0xff0000ff)])
for name, index, value, status in [
    ('magic', 0, 0, 1), ('version', 1, 1, 1), ('version-two', 1, 2, 1), ('zero-width', 2, 0, 1),
    ('large-width', 2, 8193, 3), ('pixel-budget', 3, 262145, 3),
    ('paths-budget', 5, 4097, 3), ('draws-budget', 6, 65537, 3),
    ('commands-budget', 7, 262145, 3), ('missing-gradient', 9, 1, 1), ('styles-budget', 8, 4097, 3),
    ('fill', 10, 2, 1), ('opcode', 12, 0, 1), ('missing-move', 12, 2, 1),
    ('nan', 13, 0x7fc00000, 1), ('infinity', 13, 0x7f800000, 1),
    ('large-coordinate', 13, bits(32769), 1), ('unused-field', 15, 1, 1),
    ('count-mismatch', 7, 4, 1), ('bad-path', len(base)-6, 1, 1),
    ('bad-translation', len(base)-5, bits(float('nan')), 1),
    ('translated-range', len(base)-5, bits(32760), 3),
]:
    r = base.copy()
    r[index] = value
    add('reject-' + name, r, status)
add('reject-trailing', base + [0], 1)
for i in range(10):
    add('reject-header-' + str(i), base[:i], 1)
add('reject-truncated-path', base[:-10], 1)
add('reject-truncated-draw', base[:-1], 1)
add('reject-close-twice', frame([(0, rect(8, 8, 10, 10) + [cmd(5)])], []), 1)
add('reject-draw-work', frame([(0, [cmd(1, 0, 0)] * 257)], [(0, 0, 0, 0)] * 4081), 3)
add('maximum-width', frame(width=8192, height=1), oracle='empty')
add('maximum-paths', frame([(0, [])] * 4096, width=1, height=1), oracle='empty')
add('maximum-draws', frame([(0, [])], [(0, 0, 0, 0)] * 65536, width=1, height=1), oracle='empty')
add('maximum-commands', frame([(0, [cmd(1, 0, 0)] * 262144)], width=1, height=1), oracle='empty')
add('coordinate-boundary', frame([(0, [cmd(1, -32768, 32768)])], [(0, 0, 0, 0)], width=1, height=1), oracle='empty')

stroke_base = frame([(0, [cmd(1, 20, 32), cmd(2, 44, 32)])],
                    [(0, 0, 0, 0xff0000ff, 1)], styles=[(bits(8), 0, 1, 0)])
stroke_start = 10 + 2 + 2 * 7
add('stroke-line', stroke_base)
for cap in range(3):
    for join in range(3):
        add(f'stroke-cap-{cap}-join-{join}',
            frame([(0, [cmd(1, 20, 44), cmd(2, 20, 20), cmd(2, 44, 20)])],
                  [(0, 0, 0, 0xff0000ff, 1)], styles=[(bits(16), cap, join, bits(4) if join == 0 else 0)]))
for name, index, value, status in [
    ('width-negative', stroke_start, bits(-1), 1),
    ('width-nan', stroke_start, bits(float('nan')), 1),
    ('width-infinity', stroke_start, bits(float('inf')), 1),
    ('width-range', stroke_start, bits(32769), 1),
    ('cap', stroke_start+1, 3, 1), ('join', stroke_start+2, 4, 1),
    ('unused-miter', stroke_start+3, bits(1), 1),
    ('reference', len(stroke_base)-2, 2, 1),
    ('expanded-range', len(stroke_base)-5, bits(32722), 3),
]:
    bad = stroke_base.copy(); bad[index] = value
    add('reject-stroke-'+name, bad, status)
for value in [-1, 1025, float('nan'), float('inf')]:
    bad = stroke_base.copy(); bad[stroke_start+2] = 0; bad[stroke_start+3] = bits(value)
    add('reject-stroke-miter-'+str(value), bad, 1)
add('maximum-styles', frame(styles=[(bits(i), 0, 1, 0) for i in range(4096)], width=1, height=1), oracle='empty')
add('reject-unused-style', frame(styles=[(bits(-1), 0, 1, 0)]), 1)

for cap in range(3):
    for limit in [1, 1.25, 2, 4, 1024]:
        add(f'clip-cap-{cap}-limit-{limit}',
            frame([(0, [cmd(1, 20, 56), cmd(2, 32, 24), cmd(2, 44, 56)])],
                  [(0, 0, 0, 0xff0000ff, 1)], styles=[(bits(8), cap, 3, bits(limit))]))
for value in [-1, 0, .999, 1025, float('nan'), float('inf')]:
    add('reject-clip-limit-'+str(value), frame(styles=[(bits(8), 0, 3, bits(value))]), 1)
add('reject-clip-hairline-range',
    frame([(0, [cmd(1, 32700, 0), cmd(2, 32701, 0)])],
          [(0, 0, 0, 0xff0000ff, 1)], styles=[(0, 0, 3, bits(1024))]), 3)

mapping = []
parity = json.loads(Path('.codex-work/paragraph-paths/parity.json').read_text())
for case in parity['cases']:
    response_path = Path(case['responsePath'])
    response = json.loads(response_path.read_text())
    scene = response.get('result', {}).get('scene')
    if scene is None:
        continue
    name = 'paragraph-' + case['name']
    bounds = scene['bounds']
    if bounds is None:
        origin = [0, 0]
        scale = Fraction(1, 1 << 32)
    else:
        origin = [int(bounds['min'][d]) for d in ['x', 'y']]
        span = max(int(bounds['max'][d]) - origin[k] for k, d in enumerate(['x', 'y']))
        scale = Fraction(448, max(span, 1))
    # Preserve shared local path resources; translation is rebased in exact Q32.
    paths = []
    for path in scene['paths']:
        commands = []
        for c in path['commands']:
            keys = {'move': ['to'], 'line': ['to'], 'quadratic': ['control', 'to'],
                    'cubic': ['control1', 'control2', 'to'], 'close': []}[c['kind']]
            op = ['move', 'line', 'quadratic', 'cubic', 'close'].index(c['kind']) + 1
            commands.append(cmd(op, *[int(c[key][d]) * scale for key in keys for d in ['x', 'y']]))
        paths.append((0, commands))
    draws = [(g['path'], *[32 + (int(g['origin'][d]) - origin[k]) * scale for k, d in enumerate(['x', 'y'])],
              0xff332211) for g in scene['glyphs']]
    words = frame(paths, draws, 512, 512, 0xffffffff)
    add(name, words, sourceResponseSha256=hashlib.sha256(response_path.read_bytes()).hexdigest())
    # A huge document translation must not enter a float before subtracting the viewport origin.
    shifted = 1 << 110
    shifted_draws = [(g['path'], *[32 + (int(g['origin'][d]) + shifted - (origin[k] + shifted)) * scale
                                  for k, d in enumerate(['x', 'y'])], 0xff332211) for g in scene['glyphs']]
    assert words == frame(paths, shifted_draws, 512, 512, 0xffffffff)
    mapping.append({'name': name, 'glyphs': len(draws), 'paths': len(paths), 'largeOriginInvariant': True})

(directory / 'fixtures.json').write_text(json.dumps({'cases': cases, 'paragraphMapping': mapping}, indent=2) + '\n')
print(json.dumps({'cases': len(cases), 'paragraphScenes': len(mapping)}))
