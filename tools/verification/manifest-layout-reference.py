"""Compare current real Native manifest paths to a frozen executable baseline.

Also independently recompute glyph origins with Fraction and linear outline
bounds. The owned corpus has no curved paths. This verifies resource reuse and
layout/path continuity, not Office/WPS typography or a new WASM operation.
"""
import hashlib
import json
from fractions import Fraction as F
from pathlib import Path
from line_geometry_math import reference

ROOT = Path('.codex-work/manifest-layout')
NEW = ROOT / 'native-paths.json'
OLD = ROOT / 'previous/frozen-runtime-paths.json'
source = json.loads(NEW.read_text())
baseline = json.loads(OLD.read_text())
U = 1 << 32


def entry(path):
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


def bounds(points):
    if not points:
        return None
    return dict(min=dict(x=str(min(p[0] for p in points)), y=str(min(p[1] for p in points))),
                max=dict(x=str(max(p[0] for p in points)), y=str(max(p[1] for p in points))))


assert len(source['operations']) == len(baseline) == 20
assert source['manifestVerifiedFaces'] == 1
counts = dict(operations=20, errors=0, evaluated=0, scenes=0, paths=0, glyphs=0, originCoordinates=0, bounds=0)
cases = []
for ordinal, (new, old) in enumerate(zip(source['operations'], baseline)):
    assert new['request'] == old['request']
    q = new['request']
    expected = old['response']
    if expected['status'] == 'error':
        assert new['legacy'] == new['prepared'] == dict(error=expected['error']['message'])
        counts['errors'] += 1
        cases.append(dict(case=ordinal, error=expected['error']))
        continue
    assert expected['status'] == 'evaluated'
    legacy, prepared = new['legacy']['result'], new['prepared']['result']
    assert legacy == prepared['paths'] == expected['result']
    assert prepared['profile'] == 'explicit-font-resource-manifest-draft-v1'
    assert prepared['bindings'] == [dict(style=0, typeface=0, face=0, fontStyle='regular',
                                         candidate=dict(font=0, variations=[]), policy=dict(kind='exactFamily'))]
    counts['evaluated'] += 1
    r = prepared['paths']
    assert r['layout']['work']['verifiedFaces'] == 1
    scene = r['scene']
    cases.append(dict(case=ordinal, decisions=len(r['layout']['decisions']),
                      issues=r['layout']['issues'], scene=scene is not None))
    if scene is None:
        continue
    counts['scenes'] += 1
    counts['paths'] += len(scene['paths'])
    layout = q['layout']
    gq = dict(shaping=dict(paragraph=layout['paragraph']), styles=layout['styles'],
              strutStyle=layout['strutStyle'], spacing=layout['spacing'])
    exact = reference(gq, r['layout']['geometry'], True)
    expected_positions = [dict(g, line=i) for i, line in enumerate(exact['lines']) for g in line['glyphs']]
    assert len(expected_positions) == len(scene['glyphs'])
    path_points = []
    for p in scene['paths']:
        points = []
        for command in p['commands']:
            assert command['kind'] in ['move', 'line', 'close']
            if 'to' in command:
                points.append(tuple(int(command['to'][k]) for k in ['x', 'y']))
        path_points.append(points)
        assert p['bounds'] == bounds(points)
        counts['bounds'] += 1
    all_points = []
    for actual, expected in zip(scene['glyphs'], expected_positions):
        assert all(actual[k] == expected[k] for k in ['line', 'source', 'glyph'])
        assert all(F(int(actual['origin'][k]), U) == expected[k] for k in ['x', 'y'])
        dx, dy = [int(actual['origin'][k]) for k in ['x', 'y']]
        all_points.extend((x + dx, y + dy) for x, y in path_points[actual['path']])
        counts['glyphs'] += 1
        counts['originCoordinates'] += 2
    assert scene['bounds'] == bounds(all_points)
    counts['bounds'] += 1

previous = json.loads(Path('docs/reviews/evidence/2026-09-25-native-fonts-library-verification.json').read_text())
for item in previous['previousReleaseArtifactsVerifiedUnchanged'].values():
    assert entry(Path(item['path'])) == item
report = dict(format='musteroffice.manifest-layout-native-reference/1', scope=__doc__, counts=counts,
              inputs=[entry(OLD), entry(NEW)], cases=cases,
              frozenRuntimeEvidence=entry(Path('docs/reviews/evidence/2026-09-25-text-body-verification.json')))
(ROOT / 'reference.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(counts))
