"""Check owned table-style corpus projections against independent XML locations."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree

A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
NS = {'a': A}
REGIONS = ('wholeTbl band1H band2H band1V band2V lastCol firstCol lastRow '
           'seCell swCell firstRow neCell nwCell').split()
EDGES = 'left right top bottom insideH insideV tl2br tr2bl'.split()


class Verify:
    def __init__(self, xml):
        self.root = etree.fromstring(xml)
        self.ordinals = {node: i for i, node in enumerate(self.root.iter())}
        self.checked = 0

    def at(self, node, value):
        assert node is not None and value is not None
        assert value['sourceOrdinal'] == self.ordinals[node]
        self.checked += 1

    def color(self, node, value):
        if node is None:
            assert value is None
            return
        self.at(node, value)
        tag = etree.QName(node).localname
        if tag == 'schemeClr':
            assert value['value'] == {'kind': 'scheme', 'slot': node.get('val')}
        elif tag == 'srgbClr':
            assert value['value'] == {'kind': 'srgb', 'rgb': list(bytes.fromhex(node.get('val')))}
        else:
            raise AssertionError(('unexpected owned fixture color', tag))
        assert value['transforms'] == [{'kind': etree.QName(c).localname, 'value': c.get('val')} for c in node]

    def reference(self, node, value):
        self.at(node, value)
        assert value['index'] == int(node.get('idx'))
        self.color(next(iter(node), None), value['color'])
        assert value['retainedOrdinals'] == []

    def fill(self, parent, value):
        reference = parent.find('a:fillRef', NS)
        direct = parent.find('a:fill', NS)
        if reference is not None:
            assert value['kind'] == 'reference'
            self.reference(reference, value['reference'])
        elif direct is not None:
            node = direct[0]
            assert value['kind'] == 'direct'
            self.at(node, value['fill'])
            decl = value['fill']['definition']
            if etree.QName(node).localname == 'noFill':
                assert decl == {'kind': 'none'}
            else:
                assert etree.QName(node).localname == 'solidFill'
                assert decl['kind'] == 'solid'
                self.color(node[0], decl['color'])
        else:
            assert value is None

    def text(self, node, value):
        if node is None:
            assert value is None
            return
        self.at(node, value)
        assert value['bold'] == node.get('b')
        assert value['italic'] == node.get('i')
        ref, collection = node.find('a:fontRef', NS), node.find('a:font', NS)
        font = value['font']
        if ref is not None:
            self.at(ref, font)
            assert font['kind'] == 'reference' and font['index'] == ref.get('idx')
            self.color(next(iter(ref), None), font['color'])
        elif collection is not None:
            self.at(collection, font)
            assert font['kind'] == 'collection'
            for child, key in [('latin', 'latin'), ('ea', 'eastAsian'), ('cs', 'complexScript')]:
                assert font['fonts'][key]['typeface'] == collection.find('a:' + child, NS).get('typeface')
            assert font['fonts']['supplemental'] == [dict(c.attrib) for c in collection.findall('a:font', NS)]
        else:
            assert font is None
        color = next((c for c in node if etree.QName(c).localname.endswith('Clr')), None)
        self.color(color, value['color'])

    def style(self, node, value):
        self.at(node, value)
        assert value['styleId'] == node.get('styleId')
        assert value['name'] == node.get('styleName')
        assert value['retainedOrdinals'] == []
        actual = {etree.QName(c).localname: c for c in node if etree.QName(c).localname in REGIONS}
        assert set(actual) == set(value['parts'])
        for region, child in actual.items():
            part = value['parts'][region]
            self.at(child, part)
            assert part['retainedOrdinals'] == []
            self.text(child.find('a:tcTxStyle', NS), part['text'])
            cell = child.find('a:tcStyle', NS)
            if cell is None:
                assert part['cell'] is None
                continue
            result = part['cell']
            self.at(cell, result)
            self.fill(cell, result['fill'])
            borders = cell.find('a:tcBdr', NS)
            if borders is not None:
                self.at(borders, result['borders'])
                for edge, decl in zip(EDGES, result['borders']['edges'], strict=True):
                    wrapper = borders.find('a:' + edge, NS)
                    if wrapper is None:
                        assert decl is None
                    elif etree.QName(wrapper[0]).localname == 'lnRef':
                        assert decl['kind'] == 'reference'
                        self.reference(wrapper[0], decl['reference'])
                    else:
                        assert decl['kind'] == 'direct'
                        self.at(wrapper[0], decl['line'])
                        assert decl['line']['width'] == wrapper[0].get('w')
            else:
                assert result['borders'] is None
        bg = node.find('a:tblBg', NS)
        if bg is not None:
            self.at(bg, value['background'])
            self.fill(bg, value['background']['fill'])
            effect = bg.find('a:effect/a:effectLst', NS)
            if effect is not None:
                effects = value['background']['effects']
                assert effects['kind'] == 'direct'
                self.at(effect, effects['effects'])
                ids = [self.ordinals[c] for c in effect]
                assert effects['effects']['definition'] == {'kind': 'list', 'nodes': ids}
                assert set(value['effectNodes']) == {str(i) for i in ids}
                for child in effect:
                    decl = value['effectNodes'][str(self.ordinals[child])]
                    self.at(child, decl)
        else:
            assert value['background'] is None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--parity-dir', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    assert not args.output.exists()
    manifest = json.loads((args.parity_dir / 'verification.json').read_text())
    rows = []
    for case in manifest['cases']:
        name = case['name']
        data = (args.parity_dir / (name + '.pptx')).read_bytes()
        assert hashlib.sha256(data).hexdigest() == case['outputSha256']
        index = json.loads((args.parity_dir / (name + '.after.json')).read_text())['index']
        checks, styles = 0, 0
        with zipfile.ZipFile(args.parity_dir / (name + '.pptx')) as package:
            if catalog := index.get('tableStyles'):
                data = package.read(catalog['part'].lstrip('/'))
                assert hashlib.sha256(data).hexdigest() == catalog['sha256']
                v = Verify(data)
                v.at(v.root, catalog)
                assert catalog['defaultStyleId'] == v.root.get('def')
                xml = v.root.findall('a:tblStyle', NS)
                assert {c.get('styleId').upper() for c in xml} == set(catalog['styles'])
                for child in xml:
                    v.style(child, catalog['styles'][child.get('styleId').upper()])
                    styles += 1
                checks += v.checked
            for part, surface in index['surfaces'].items():
                v = Verify(package.read(part.lstrip('/')))
                by_ordinal = {i: n for n, i in v.ordinals.items()}
                for obj in surface['objects']:
                    props = (obj.get('table') or {}).get('properties') or {}
                    if style := props.get('inlineStyle'):
                        v.style(by_ordinal[style['sourceOrdinal']], style)
                        styles += 1
                checks += v.checked
        rows.append({'name': name, 'styles': styles, 'physicalDeclarations': checks})
    report = {'scope': 'owned native table style declarations and physical source locations; no layout or visual acceptance',
              'styles': sum(r['styles'] for r in rows), 'physicalDeclarations': sum(r['physicalDeclarations'] for r in rows), 'cases': rows}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'cases'}))


if __name__ == '__main__':
    main()
