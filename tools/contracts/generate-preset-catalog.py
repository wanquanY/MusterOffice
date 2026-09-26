"""Compile pinned ECMA preset data; no network, application code or XML mutation in-place.

The runtime catalog is an implementation derivative, not a corrected publication
of the standard. Every adjustment to the source is enumerated in catalog.json.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import struct
import xml.etree.ElementTree as E

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'components/drawingml-presets'
SOURCE_SHA = '2f7c868d857c1e3c4b5a6068759fe0e07d77ad58377a6618d1b02ba3507b6939'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
E.register_namespace('', A)
source = (BASE / 'presetShapeDefinitions.xml').read_bytes()
assert hashlib.sha256(source).hexdigest() == SOURCE_SHA
nodes = list(E.fromstring(source))
assert len(nodes) == 187
catalog, records = {}, {}
for ordinal, node in enumerate(nodes):
    if node.tag in catalog:
        assert node.tag == 'upDownArrow' and ordinal == 179
        assert E.tostring(node) == E.tostring(nodes[178])
        records[node.tag]['duplicateSourceRecord'] = ordinal
        continue
    catalog[node.tag] = copy.deepcopy(node)
    records[node.tag] = {'sourceRecord': ordinal, 'changes': []}

# The annex contains no upArrow and repeats upDownArrow verbatim. Build the
# vertical reflection of downArrow, preserving adjustment domains and text box.
up = copy.deepcopy(catalog['downArrow'])
up.tag = 'upArrow'
guides = up.find(f'{{{A}}}gdLst')
def mirror(token):
    name = 'moUp_' + token
    if not any(g.get('name') == name for g in guides):
        E.SubElement(guides, f'{{{A}}}gd', {'name': name, 'fmla': f'+- h 0 {token}'})
    return name
for node in up.iter():
    if 'y' in node.attrib:
        node.set('y', mirror(node.get('y')))
    if node.tag == f'{{{A}}}cxn':
        angle = node.get('ang')
        name = 'moUpAngle_' + angle
        E.SubElement(guides, f'{{{A}}}gd', {'name': name, 'fmla': f'+- 0 0 {angle}'})
        node.set('ang', name)
rect = up.find(f'{{{A}}}rect')
top, bottom = rect.get('t'), rect.get('b')
rect.set('t', mirror(bottom)); rect.set('b', mirror(top))
catalog['upArrow'] = up
records['upArrow'] = {'sourceRecord': None, 'changes': [{'kind': 'verticalReflection', 'from': 'downArrow', 'reason': 'Missing annex definition; reflection implements the upward-arrow geometry, including text box, handles and connectors.'}]}

# These eight expressions have four operands although +- has exactly three.
# The last zero is redundant in the published geometry construction.
repairs = 0
for name, node in catalog.items():
    for g in node.iter():
        formula = g.get('fmla')
        if formula and len(formula.split()) == 5:
            assert name in ['circularArrow', 'leftCircularArrow', 'leftRightCircularArrow']
            words = formula.split()
            assert words[0] == '+-' and words[2] == words[4] == '0'
            fixed = ' '.join(words[:4])
            records[name]['changes'].append({'kind': 'formulaArity', 'guide': g.get('name'), 'original': formula, 'runtime': fixed})
            g.set('fmla', fixed); repairs += 1
assert repairs == 8 and len(catalog) == 187

blob, index = bytearray(), bytearray()
for name in sorted(catalog):
    root = catalog[name]
    root.tag = f'{{{A}}}custGeom'
    for node in root.iter():
        node.text = None; node.tail = None
    data = E.tostring(root, encoding='utf-8', short_empty_elements=True)
    index += struct.pack('<II', len(blob), len(data))
    records[name]['byteOffset'] = len(blob)
    records[name]['byteLength'] = len(data)
    records[name]['elements'] = sum(1 for _ in root.iter())
    records[name]['sha256'] = hashlib.sha256(data).hexdigest()
    blob += data
manifest = {'format': 'musteroffice.drawingml-preset-catalog/1', 'sourceSha256': SOURCE_SHA,
    'ordinalConvention': 'Zero-based XML element preorder within the generated custGeom definition; document override ordinals remain in their document part.',
    'catalogSha256': hashlib.sha256(blob).hexdigest(), 'indexSha256': hashlib.sha256(index).hexdigest(),
    'additionalCatalogBuiltins': {'wd12': 'w/12', 'wd32': 'w/32', 'hd10': 'h/10', 'cd3': '21600000/3'},
    'definitions': records}
outputs = {'catalog.xml': bytes(blob), 'catalog-index.bin': bytes(index), 'catalog.json': (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()}
check = argparse.ArgumentParser(); check.add_argument('--check', action='store_true'); args = check.parse_args()
for name, data in outputs.items():
    path = BASE / name
    if args.check: assert path.read_bytes() == data, name
    else: path.write_bytes(data)
print(json.dumps({'definitions': len(catalog), 'runtimeBytes': len(blob) + len(index), 'formulaRepairs': repairs, 'checked': args.check}))
