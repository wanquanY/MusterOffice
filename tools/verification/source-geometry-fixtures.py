"""Real PPTX geometry corpus, built independently of the kernel writer.

Official definitions are local verification input only, not runtime templates.
The downloaded standard archive and generated decks remain in ignored storage.
"""
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A, P, MC

ROOT = Path('.codex-work/source-geometry')
ROOT.mkdir(exist_ok=True)
NS = {'a': A, 'p': P}
def sha(b): return hashlib.sha256(b).hexdigest()
manifest = json.loads(Path('.codex-work/pptx-color-map-fixtures/manifest.json').read_text())
base = next(c for c in manifest['cases'] if c['name'] == 'authored')
assert sha(Path(base['path']).read_bytes()) == base['sha256']
with zipfile.ZipFile(base['path']) as z:
    parts = {n: z.read(n) for n in z.namelist()}
cases = []
def emit(name, xml, error=None, conforming=True, utf16=False, part='ppt/slides/slide1.xml', extra=''):
    owned = copy.deepcopy(parts)
    original = E.fromstring(owned[part])
    root = E.Element(original.tag, nsmap={**original.nsmap, 'mc': MC, 'u': 'urn:owned:future'})
    root.attrib.update(original.attrib)
    for child in original: root.append(child)
    props = root.find('p:cSld/p:spTree/p:sp/p:spPr', NS)
    if props is None:
        shape = copy.deepcopy(E.fromstring(parts['ppt/slides/slide1.xml']).find('p:cSld/p:spTree/p:sp', NS))
        root.find('p:cSld/p:spTree', NS).append(shape)
        props = shape.find('p:spPr', NS)
    for child in list(props): props.remove(child)
    wrapper = E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}" xmlns:mc="{MC}" xmlns:u="urn:owned:future">{xml}</root>')
    for child in wrapper: props.append(child)
    if extra:
        wrapper = E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}">{extra}</root>')
        for child in wrapper: root.find('p:cSld/p:spTree', NS).append(child)
    owned[part] = E.tostring(root, xml_declaration=True, encoding='UTF-16' if utf16 else 'UTF-8')
    path = ROOT / (name + '.pptx')
    with zipfile.ZipFile(path, 'w') as z:
        for n, b in owned.items():
            info = zipfile.ZipInfo(n, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, b)
    cases.append({'name': name, 'path': str(path), 'sha256': sha(path.read_bytes()),
                  'part': '/' + part, 'error': error, 'xsdConforming': conforming and error is None})

xsd = Path('.codex-work/ecma376/xsd/dml-main.xsd')
assert sha(xsd.read_bytes()) == '6978ba7e889070b0c3cb5b546b23e5a6c3516134afc53b87a21f482ca33f3858'
names = sorted(E.parse(str(xsd)).xpath('//x:simpleType[@name="ST_ShapeType"]//x:enumeration/@value', namespaces={'x': 'http://www.w3.org/2001/XMLSchema'}))
assert len(names) == 187
for name in names: emit('preset-' + name, f'<a:prstGeom prst="{name}"/>')
definitions = Path('.codex-work/ecma376/presetShapeDefinitions.xml')
assert sha(definitions.read_bytes()) == '2f7c868d857c1e3c4b5a6068759fe0e07d77ad58377a6618d1b02ba3507b6939'
templates = E.parse(str(definitions)).getroot()
assert len(templates) == 187 and len({n.tag for n in templates}) == 186
assert set(names) - {n.tag for n in templates} == {'upArrow'}
assert [n.tag for n in templates].count('upDownArrow') == 2
for ordinal, node in enumerate(templates):
    custom = E.Element(f'{{{A}}}custGeom', nsmap={'a': A})
    for child in node: custom.append(copy.deepcopy(child))
    emit(f'definition-{ordinal:03}-' + node.tag, E.tostring(custom).decode())

def custom(inner): return '<a:custGeom>' + inner + '</a:custGeom>'
def path(inner='', attrs=''): return custom(f'<a:pathLst><a:path {attrs}>{inner}</a:path></a:pathLst>')
emit('absent', '')
emit('empty-adjustments', '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>')
emit('preset-token-space', '<a:prstGeom prst=" rect "/>')
emit('duplicate-guides', custom('<a:avLst><a:gd name="adj" fmla="val 1"/><a:gd name="adj" fmla="val 2"/></a:avLst><a:gdLst><a:gd name="adj" fmla="val adj"/></a:gdLst><a:pathLst/>'))
emit('deferred-formula', custom('<a:gdLst><a:gd name="" fmla="not a known formula"/></a:gdLst><a:pathLst/>'))
emit('empty-lists', custom('<a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:pathLst/>'))
emit('empty-path', path())
emit('explicit-defaults', path(attrs='w="0" h="0" fill="norm" stroke="true" extrusionOk="1"'))
emit('extent-max', path(attrs='w="27273042316900" h="+0021600"'))
for fill in ['none', 'norm', 'lighten', 'lightenLess', 'darken', 'darkenLess']:
    emit('fill-' + fill, path(attrs=f'fill="{fill}" stroke="0" extrusionOk="false"'))
complete = custom('''<a:avLst><a:gd name="adj" fmla="val +0001"/></a:avLst>
<a:gdLst><a:gd name="g" fmla="*/ w adj 100000"/></a:gdLst>
<a:ahLst><a:ahXY gdRefX="adj" minX="-20" maxX="g" gdRefY="g" minY="0" maxY="h"><a:pos x="+0001" y="2.5cm"/></a:ahXY><a:ahPolar gdRefR="adj" minR="0" maxR="w" gdRefAng="g" minAng="0" maxAng="cd2"><a:pos x="w" y="h"/></a:ahPolar></a:ahLst>
<a:cxnLst><a:cxn ang="cd4"><a:pos x="hc" y="vc"/></a:cxn></a:cxnLst><a:rect l="l" t="t" r="r" b="b"/>
<a:pathLst><a:path w="0" h="21600" fill="lightenLess" stroke="0" extrusionOk="false"><a:moveTo><a:pt x="-0007" y="0"/></a:moveTo><a:lnTo><a:pt x="w" y="h"/></a:lnTo><a:arcTo wR="wd2" hR="hd2" stAng="cd4" swAng="-5400000"/><a:quadBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/></a:quadBezTo><a:cubicBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/><a:pt x="5" y="6"/></a:cubicBezTo><a:close/></a:path><a:path/></a:pathLst>''')
emit('all-records', complete)
emit('utf16', complete, utf16=True)
emit('master', complete, part='ppt/slideMasters/slideMaster1.xml')
emit('layout', complete, part='ppt/slideLayouts/slideLayout1.xml')
emit('picture-geometry', '', extra=f'<p:pic><p:nvPicPr><p:cNvPr id="101" name="Geometry picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill/><p:spPr>{complete}</p:spPr></p:pic>')
emit('connector-geometry', '', extra=f'<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="101" name="Geometry connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr>{complete}</p:spPr></p:cxnSp>')
emit('group-child-geometry', '', extra=f'<p:grpSp><p:nvGrpSpPr><p:cNvPr id="101" name="Geometry group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="102" name="Geometry child"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>{complete}</p:spPr></p:sp></p:grpSp>')
emit('unresolved-coordinate', path('<a:moveTo><a:pt x="unknown guide" y="99999999999999999999999999"/></a:moveTo>'))
emit('retained-attribute', path('<a:moveTo><a:pt x="0" y="0" owned="retain"/></a:moveTo>'), conforming=False)
emit('extension-decoy', '<a:prstGeom prst="rect"/><a:extLst><a:ext uri="owned"><u:data><a:prstGeom prst="invalid"/></u:data></a:ext></a:extLst>')
emit('selected-fallback', '<mc:AlternateContent><mc:Choice Requires="u"><a:prstGeom prst="invalid"/></mc:Choice><mc:Fallback>' + path('<mc:AlternateContent><mc:Choice Requires="u"><a:lnTo/></mc:Choice><mc:Fallback><a:moveTo><a:pt x="0" y="0"/></a:moveTo></mc:Fallback></mc:AlternateContent>') + '</mc:Fallback></mc:AlternateContent>')
invalid = {
    'missing-preset': '<a:prstGeom/>', 'unknown-preset': '<a:prstGeom prst="invented"/>',
    'preset-guides': '<a:prstGeom prst="rect"><a:gdLst/></a:prstGeom>',
    'missing-path-list': '<a:custGeom/>', 'order': custom('<a:pathLst/><a:avLst/>'),
    'duplicate-list': custom('<a:pathLst/><a:pathLst/>'),
    'missing-formula': custom('<a:gdLst><a:gd name="g"/></a:gdLst><a:pathLst/>'),
    'missing-position': custom('<a:ahLst><a:ahXY/></a:ahLst><a:pathLst/>'),
    'missing-connection-position': custom('<a:cxnLst><a:cxn ang="0"/></a:cxnLst><a:pathLst/>'),
    'duplicate': '<a:prstGeom prst="rect"/><a:prstGeom prst="ellipse"/>',
    'move': path('<a:moveTo/>'), 'point': path('<a:lnTo><a:pt x="0"/></a:lnTo>'),
    'quadratic': path('<a:quadBezTo><a:pt x="0" y="0"/></a:quadBezTo>'),
    'cubic': path('<a:cubicBezTo><a:pt x="0" y="0"/><a:pt x="1" y="1"/></a:cubicBezTo>'),
    'close': path('<a:close><a:pt x="0" y="0"/></a:close>'),
    'arc': path('<a:arcTo wR="1" hR="1" stAng="0"/>'), 'text': path('not geometry'),
}
for name, attrs in [('negative', 'w="-1"'), ('overflow', 'h="27273042316901"'), ('fill', 'fill="unknown"'), ('stroke', 'stroke="yes"'), ('extrusion', 'extrusionOk="TRUE"')]:
    invalid[name] = path(attrs=attrs)
for name, xml in invalid.items(): emit('reject-' + name, xml, error='INPUT_INVALID')
emit('reject-unknown-child', custom('<a:unknown/><a:pathLst/>'), error='MAPPING_NOT_IMPLEMENTED')
(ROOT / 'manifest.json').write_text(json.dumps({'base': base, 'vocabularyXsdSha256': sha(xsd.read_bytes()),
    'standardDefinitions': {'path': str(definitions), 'sha256': sha(definitions.read_bytes()), 'use': 'verification only',
        'records': 187, 'uniqueNames': 186, 'duplicateName': 'upDownArrow', 'missingName': 'upArrow'}, 'cases': cases}, indent=2) + '\n')
print(json.dumps({'cases': len(cases), 'valid': sum(c['error'] is None for c in cases), 'presets': len(names), 'standardDefinitions': len(templates)}))
