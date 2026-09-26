"""Owned slide/layout/master color-map corpus, with visible solid-color probes."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import P, A, MC

NS = {'p': P, 'a': A}
SLIDE = 'ppt/slides/slide1.xml'
LAYOUT = 'ppt/slideLayouts/slideLayout2.xml'
MASTER = 'ppt/slideMasters/slideMaster2.xml'


def encode(root):
    return E.tostring(root, encoding='UTF-8', xml_declaration=True)


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('manifest', type=Path); parser.add_argument('directory', type=Path)
    args = parser.parse_args(); args.directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(args.manifest.read_text())
    authored = next(c for c in manifest['cases'] if c['name'] == 'authored')
    with zipfile.ZipFile(authored['path']) as package:
        original = {n: package.read(n) for n in package.namelist()}
    root = E.fromstring(original[SLIDE])
    shape = root.xpath('.//p:sp[p:nvSpPr/p:cNvPr/@name="round:1"]', namespaces=NS)[0]
    fill = shape.find('p:spPr/a:solidFill', NS); fill.clear()
    E.SubElement(fill, E.QName(A, 'schemeClr'), val='accent1')
    original[SLIDE] = encode(root)
    root = E.fromstring(original['ppt/theme/theme2.xml'])
    for slot, rgb in [('accent1', 'FF0000'), ('accent2', '0000FF'), ('accent3', '00FF00'), ('accent4', 'FFFF00')]:
        root.find('a:themeElements/a:clrScheme/a:' + slot + '/a:srgbClr', NS).set('val', rgb)
    original['ppt/theme/theme2.xml'] = encode(root)
    attrs = dict(E.fromstring(original[MASTER]).find('p:clrMap', NS).attrib)

    def override(parts, part, slot):
        root = E.fromstring(parts[part]); parent = root.find('p:clrMapOvr', NS)
        for child in list(parent): parent.remove(child)
        E.SubElement(parent, E.QName(A, 'overrideClrMapping'), attrs | {'accent1': slot})
        parts[part] = encode(root)

    def emit(name, parts, part=None, accent=None, error=False, rgb=None):
        path = args.directory / (name + '.pptx')
        with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as package:
            for name_part, raw in parts.items():
                info = zipfile.ZipInfo(name_part, (2026, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
                package.writestr(info, raw)
        expected = {'error': 'INPUT_INVALID'} if error else {'objects': 15, 'edit': 'success', 'colorMapping': None if part is None else {'part': '/' + part, 'accent1': accent}}
        sample = {'name': name, 'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'expect': expected}
        if rgb: sample['externalColorProbe'] = {'slide': 0, 'object': 'round:1', 'rgb': rgb}
        manifest['cases'].append(sample)

    inherited = copy.deepcopy(original); override(inherited, LAYOUT, 'accent2')
    emit('color-map-layout-master-marker', inherited, LAYOUT, 'accent2', rgb=[0, 0, 255])
    parts = copy.deepcopy(inherited); root = E.fromstring(parts[SLIDE]); root.remove(root.find('p:clrMapOvr', NS)); parts[SLIDE] = encode(root)
    emit('color-map-layout-omitted-marker', parts, LAYOUT, 'accent2', rgb=[0, 0, 255])
    parts = copy.deepcopy(inherited); override(parts, SLIDE, 'accent3')
    emit('color-map-slide-override', parts, SLIDE, 'accent3', rgb=[0, 255, 0])
    parts = copy.deepcopy(original); root = E.fromstring(parts[MASTER]); root.find('p:clrMap', NS).set('accent1', 'accent4'); parts[MASTER] = encode(root)
    emit('color-map-master', parts, MASTER, 'accent4', rgb=[255, 255, 0])
    parts = copy.deepcopy(original); root = E.fromstring(parts[MASTER]); root.remove(root.find('p:clrMap', NS)); parts[MASTER] = encode(root)
    emit('color-map-missing-master-map', parts)
    parts = copy.deepcopy(inherited); root = E.fromstring(parts[LAYOUT]); parent = root.find('p:clrMapOvr', NS); node = parent[0]; parent.remove(node)
    alternate = E.SubElement(parent, E.QName(MC, 'AlternateContent'), nsmap={'mc': MC, 'future': 'urn:owned:future'})
    E.SubElement(E.SubElement(alternate, E.QName(MC, 'Choice'), Requires='future'), E.QName(A, 'overrideClrMapping'), accent1='invalid')
    E.SubElement(alternate, E.QName(MC, 'Fallback')).append(node); parts[LAYOUT] = encode(root)
    emit('color-map-mce', parts, LAYOUT, 'accent2')
    parts = copy.deepcopy(inherited); root = E.fromstring(parts[LAYOUT]); node = root.find('p:clrMapOvr/a:overrideClrMapping', NS)
    ext = E.SubElement(E.SubElement(node, E.QName(A, 'extLst')), E.QName(A, 'ext'), uri='urn:owned:probe')
    E.SubElement(ext, E.QName(A, 'overrideClrMapping'), accent1='invalid').text = 'retained extension data'
    parts[LAYOUT] = encode(root); emit('color-map-extension', parts, LAYOUT, 'accent2')
    parts = copy.deepcopy(inherited); parts[LAYOUT] = E.tostring(E.fromstring(parts[LAYOUT]), encoding='UTF-16', xml_declaration=True)
    emit('color-map-utf16', parts, LAYOUT, 'accent2')

    for name in ['missing-attribute', 'invalid-slot', 'duplicate-master-map', 'duplicate-override', 'empty-override', 'duplicate-choice', 'nested-choice', 'wrong-location']:
        parts = copy.deepcopy(inherited); part = MASTER if name == 'duplicate-master-map' else SLIDE
        root = E.fromstring(parts[part]); parent = root.find('p:clrMapOvr', NS)
        if name in ['missing-attribute', 'invalid-slot']:
            parent.remove(parent[0]); node = E.SubElement(parent, E.QName(A, 'overrideClrMapping'), attrs)
            if name == 'missing-attribute': del node.attrib['bg1']
            else: node.set('accent1', 'bg1')
        elif name == 'duplicate-master-map': root.append(copy.deepcopy(root.find('p:clrMap', NS)))
        elif name == 'duplicate-override': root.append(copy.deepcopy(parent))
        elif name == 'empty-override': parent.remove(parent[0])
        elif name == 'duplicate-choice': E.SubElement(parent, E.QName(A, 'overrideClrMapping'), attrs)
        elif name == 'nested-choice': E.SubElement(parent[0], E.QName(A, 'overrideClrMapping'), attrs)
        else: root.append(E.Element(E.QName(P, 'clrMap'), attrs))
        parts[part] = encode(root); emit('color-map-invalid-' + name, parts, error=True)
    (args.directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'cases': len(manifest['cases']), 'manifest': str(args.directory / 'manifest.json')}))


if __name__ == '__main__':
    main()
