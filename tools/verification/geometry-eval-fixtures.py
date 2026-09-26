"""Attach real transform extents to native formula/path inputs; no kernel writer."""
import copy
import hashlib
import json
import zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A, P

ROOT = Path('.codex-work/geometry-eval')
ROOT.mkdir(exist_ok=True)
NS = {'a': A, 'p': P}
source = json.loads(Path('.codex-work/source-geometry/manifest.json').read_text())
cases = []
def sha(b): return hashlib.sha256(b).hexdigest()
def emit(base, name, width, height, inner=None, expected=None):
    assert sha(Path(base['path']).read_bytes()) == base['sha256']
    with zipfile.ZipFile(base['path']) as z: parts = {n: z.read(n) for n in z.namelist()}
    part = base['part'].lstrip('/')
    root = E.fromstring(parts[part])
    shape = root.find('p:cSld/p:spTree/p:sp', NS)
    props = shape.find('p:spPr', NS)
    xfrm = E.fromstring(f'<a:xfrm xmlns:a="{A}"><a:off x="0" y="0"/><a:ext cx="{width}" cy="{height}"/></a:xfrm>')
    if inner is not None:
        for n in list(props): props.remove(n)
        props.append(E.fromstring(f'<a:custGeom xmlns:a="{A}">{inner}</a:custGeom>'))
    for n in props.findall('a:xfrm', NS): props.remove(n)
    props.insert(0, xfrm)
    parts[part] = E.tostring(root, xml_declaration=True, encoding='UTF-8')
    path = ROOT / (name + '.pptx')
    with zipfile.ZipFile(path, 'w') as z:
        for n, b in parts.items():
            info = zipfile.ZipInfo(n, (2026, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, b)
    cases.append({'name': name, 'path': str(path), 'sha256': sha(path.read_bytes()), 'part': '/' + part,
                  'objectId': int(shape.find('p:nvSpPr/p:cNvPr', NS).get('id')), 'extent': [width, height],
                  'expectedOutcome': expected, 'base': {'path': base['path'], 'sha256': base['sha256']}})
for c in source['cases']:
    if c['name'].startswith('definition-'):
        for label, w, h in [('wide', 2160000, 1080000), ('tall', 1234567, 3567890), ('square', 1000001, 1000001)]:
            emit(c, c['name'] + '-' + label, w, h)
base = next(c for c in source['cases'] if c['name'] == 'empty-path')
probes = [
    ('operators', ['val 2.5', '*/ 10 3 4', '+- 2 4 7', '+/ 2 3 2', '?: 0 2 3', 'abs -2', 'at2 -1 1', 'cat2 10 -3 4', 'cos 10 cd2', 'max 1 2', 'min 1 2', 'mod 2 3 6', 'pin 4 2 8', 'sat2 10 -3 4', 'sin 10 cd4', 'sqrt -16', 'tan 10 cd8'], 'resolved'),
    ('divide-zero', ['*/ 1 2 0'], 'divisionByZero'), ('direction-zero', ['at2 0 0'], 'undefinedDirection'),
    ('tan-pole', ['tan 1 cd4'], 'tangentPole'), ('arity', ['val 1 2'], 'arity'),
    ('unknown', ['madeup 1'], 'unknownOperation'), ('forward', ['val g1', 'val 1'], 'unknownReference'),
    ('nonfinite', ['val ' + '9' * 309], 'numericRange'),
    ('wide-values', ['mod ' + '1' + '0' * 200 + ' 1 1', 'cat2 10 -' + '1' + '0' * 200 + ' ' + '1' + '0' * 200], 'resolved'),
]
for name, formulas, expected in probes:
    guides = ''.join(f'<a:gd name="g{i}" fmla="{f}"/>' for i, f in enumerate(formulas))
    emit(base, 'probe-' + name, 21600, 10800, f'<a:gdLst>{guides}</a:gdLst><a:pathLst/>', expected)
for angle in [-43200001, -21600000, -16200000, -10800000, -5400000, -1, 0, 1, 2700000, 5400000, 10800000, 16200000, 21600001]:
    guides = ''.join(f'<a:gd name="g{i}" fmla="{op} 1000 {angle}"/>' for i, op in enumerate(['sin', 'cos']))
    emit(base, f'probe-angle-{angle}', 21600, 10800, f'<a:gdLst>{guides}</a:gdLst><a:pathLst/>', 'resolved')
for xy in [(1,1),(-1,1),(-1,-1),(1,-1),(0,1),(0,-1),(1,0),(-1,0)]:
    emit(base, f'probe-direction-{xy[0]}-{xy[1]}',21600,10800,f'<a:gdLst><a:gd name="angle" fmla="at2 {xy[0]} {xy[1]}"/></a:gdLst><a:pathLst/>','resolved')
for name in ['empty-lists','empty-path','explicit-defaults','extent-max','duplicate-guides','deferred-formula','unresolved-coordinate','retained-attribute','selected-fallback','master','layout']:
    c = next(c for c in source['cases'] if c['name'] == name)
    emit(c, 'source-' + name, 21600, 10800)
(ROOT / 'manifest.json').write_text(json.dumps({'cases': cases, 'standardDefinitionVariants': 561}, indent=2) + '\n')
print(json.dumps({'cases': len(cases)}))
