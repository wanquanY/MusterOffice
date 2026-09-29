"""Independently check native XML origins, stroke fields and table topology.

This verifies source queries and contracts, not visual conflict rules or pixels.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree
from jsonschema import Draft202012Validator

NS = {'a': 'http://schemas.openxmlformats.org/drawingml/2006/main',
      'p': 'http://schemas.openxmlformats.org/presentationml/2006/main'}
CELL_EDGES = dict(zip(['left', 'right', 'top', 'bottom', 'topLeftToBottomRight', 'bottomLeftToTopRight'],
                      ['lnL', 'lnR', 'lnT', 'lnB', 'lnTlToBr', 'lnBlToTr']))
STYLE_EDGES = {**dict(zip(['left', 'right', 'top', 'bottom'], ['left', 'right', 'top', 'bottom'])),
               'insideHorizontal': 'insideH', 'insideVertical': 'insideV',
               'topLeftToBottomRight': 'tl2br', 'topRightToBottomLeft': 'tr2bl'}


class Source:
    def __init__(self, package):
        parser = etree.XMLParser(resolve_entities=False, no_network=True)
        self.parts = {'/' + n: etree.fromstring(package.read(n), parser)
                      for n in package.namelist() if n.endswith('.xml')}
        self.nodes = {p: list(root.iter()) for p, root in self.parts.items()}
        self.origins = self.references = self.fields = self.topologies = 0

    def table(self, part, native_id):
        frame = next(n for n in self.parts[part].findall('.//p:graphicFrame', NS)
                     if int(n.find('p:nvGraphicFramePr/p:cNvPr', NS).get('id')) == native_id)
        return frame.find('.//a:tbl', NS)

    def anchor(self, owner):
        part, target = owner['part'], owner['target']
        table = self.table(part, target['nativeId'])
        if target['kind'] == 'tableCellBorder':
            cell = target['cell']
            tc = table.findall('a:tr', NS)[cell['row']].findall('a:tc', NS)[cell['column']]
            return part, tc.find('a:tcPr/a:' + CELL_EDGES[target['edge']], NS)
        assert target['kind'] == 'tableStyleBorder'
        props = table.find('a:tblPr', NS)
        style = props.find('a:tableStyle', NS)
        if style is None:
            key = props.find('a:tableStyleId', NS).text.upper()
            part = '/ppt/tableStyles.xml'
            style = next(n for n in self.parts[part].findall('a:tblStyle', NS) if n.get('styleId').upper() == key)
        return part, style.find('a:' + target['region'] + '/a:tcStyle/a:tcBdr/a:' + STYLE_EDGES[target['edge']], NS)

    def stroke_owner(self, origin):
        obj = origin.get('object', origin.get('via'))
        target = {'nativeId': obj['nativeId'], 'edge': origin['edge']}
        if origin['kind'] == 'tableCell':
            target.update(kind='tableCellBorder', cell=origin['cell'])
        else:
            target.update(kind='tableStyleBorder', region=origin['region'])
        return {'part': obj['part'], 'target': target}

    def origin(self, origin):
        kind = origin['kind']
        if kind == 'profileDefault':
            return None
        if kind == 'schemaDefault':
            part = origin['part']
            anchor = self.parts[part]
        elif kind in ['declaration', 'tableCell', 'tableStyle']:
            owner = origin['owner'] if kind == 'declaration' else (
                origin['via'] if kind == 'tableStyle' and 'object' not in origin else self.stroke_owner(origin))
            part, anchor = self.anchor(owner)
            if kind == 'tableStyle':
                assert part == origin['part']
        elif kind in ['theme', 'tableTheme']:
            owner = origin['via'] if kind == 'theme' else self.stroke_owner(origin)
            part, declaration = self.anchor(owner)
            reference = self.nodes[part][origin['referenceOrdinal']]
            assert reference in declaration.iter() and reference.tag == '{' + NS['a'] + '}lnRef'
            assert int(reference.get('idx')) == origin['styleIndex']
            part = origin['part']
            entries = self.parts[part].find('.//a:fmtScheme/a:lnStyleLst', NS)
            anchor = entries[origin['styleIndex'] - 1]
        else:
            raise AssertionError(kind)
        node = self.nodes[part][origin['sourceOrdinal']]
        assert anchor is not None and node in anchor.iter(), origin
        self.origins += 1
        return node

    def walk(self, value):
        if isinstance(value, list):
            for child in value:
                self.walk(child)
        elif isinstance(value, dict):
            if 'declaredBy' in value:
                self.origin(value['declaredBy'])
            if placeholder := value.get('placeholder'):
                part, anchor = self.anchor(placeholder['owner'])
                assert placeholder['sourcePart'] == part
                ref = self.nodes[part][placeholder['referenceOrdinal']]
                assert ref in anchor.iter() and ref.tag == '{' + NS['a'] + '}lnRef'
                if placeholder.get('colorOrdinal') is not None:
                    assert self.nodes[part][placeholder['colorOrdinal']] in ref
                self.references += 1
            for child in value.values():
                self.walk(child)

    def field(self, value, attribute):
        node = self.origin(value['declaredBy'])
        if node is not None:
            assert node.get(attribute) == str(value['value']), (attribute, value)
            self.fields += 1

    def stroke(self, result):
        assert result['status'] == 'resolved'
        g = result['geometry']
        for key, attribute in [('width', 'w'), ('cap', 'cap'), ('compound', 'cmpd'), ('alignment', 'algn')]:
            self.field(g[key], attribute)
        if g['join']['kind'] == 'miter':
            self.field(g['join']['limit'], 'lim')
        if g['dash']['kind'] == 'preset':
            self.field(g['dash']['value'], 'val')
        for end in ['head', 'tail']:
            for key, attribute in [('kind', 'type'), ('width', 'w'), ('length', 'len')]:
                self.field(g[end][key], attribute)

    def topology(self, part, result):
        target, actual = result['target'], result['topology']
        table = self.table(part, target['nativeId'])
        rows = [row.findall('a:tc', NS) for row in table.findall('a:tr', NS)]
        regions = {}
        for r, row in enumerate(rows):
            for c, cell in enumerate(row):
                if any(cell.get(k) in ['1', 'true'] for k in ['hMerge', 'vMerge']):
                    continue
                region = {'origin': {'row': r, 'column': c}, 'rows': int(cell.get('rowSpan', '1')), 'columns': int(cell.get('gridSpan', '1'))}
                for rr in range(r, r + region['rows']):
                    for cc in range(c, c + region['columns']):
                        assert (rr, cc) not in regions
                        regions[rr, cc] = region
        r, c = target['cell']['row'], target['cell']['column']
        edge = target['edge']
        rtl = table.find('a:tblPr', NS).get('rtl') in ['1', 'true']
        neighbour = None
        if edge in ['top', 'bottom']:
            position = {'kind': 'horizontal', 'row': r + int(edge == 'bottom'), 'column': c}
            neighbour = (r + (1 if edge == 'bottom' else -1), c)
        elif edge in ['left', 'right']:
            higher = (edge == 'right') != rtl
            position = {'kind': 'vertical', 'row': r, 'column': c + int(higher)}
            neighbour = (r, c + (1 if higher else -1))
        else:
            position = {'kind': 'diagonal', 'cell': target['cell'], 'edge': edge}
        region = regions[r, c]
        nr = regions.get(neighbour)
        assert actual == {'position': position, 'region': region,
                          'neighbour': {'row': neighbour[0], 'column': neighbour[1]} if nr else None,
                          'neighbourRegion': nr, 'insideMerge': bool(nr and nr['origin'] == region['origin'])}
        self.topologies += 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['corpus-dir', 'parity-dir', 'schema-dir', 'contracts-dir', 'output']:
        parser.add_argument('--' + key, required=True, type=Path)
    args = parser.parse_args()
    assert not args.output.exists()
    xsd = etree.XMLSchema(etree.parse(str(args.schema_dir / 'pml.xsd')))
    dml = etree.XMLSchema(etree.parse(str(args.schema_dir / 'dml-main.xsd')))
    validators = {(api, kind): Draft202012Validator(json.loads((args.contracts_dir / (prefix + '-' + kind + '.schema.json')).read_text()))
                  for api, prefix in [('border', 'pptx-table-border'), ('fill', 'pptx-fill-color')] for kind in ['query', 'response']}
    schema_count = 0
    rows = []
    digest = lambda b: hashlib.sha256(b).hexdigest()
    for case in json.loads((args.parity_dir / 'verification.json').read_text())['cases']:
        path = args.corpus_dir / (case['name'] + '.pptx')
        assert digest(path.read_bytes()) == case['sourceSha256']
        with zipfile.ZipFile(path) as package:
            source = Source(package)
        xsd_count = 0
        for root in source.parts.values():
            if root.tag.startswith('{' + NS['p'] + '}'):
                xsd.assertValid(root)
                xsd_count += 1
            elif root.tag in ['{' + NS['a'] + '}theme', '{' + NS['a'] + '}tblStyleLst']:
                dml.assertValid(root)
                xsd_count += 1
        for record in case['queries']:
            values = {}
            for name, schema in [('request', 'query'), ('response', 'response')]:
                raw = (args.parity_dir / (record['stem'] + '.' + name + '.json')).read_bytes()
                assert digest(raw) == record[name + 'Sha256']
                values[name] = json.loads(raw)
                validators[record['api'], schema].validate(values[name])
                schema_count += 1
            response = values['response']
            source.walk(response)
            if record['api'] == 'border' and response['status'] == 'evaluated':
                for target in response['borders']['targets']:
                    source.stroke(target['stroke'])
                    source.topology(values['request']['surface'], target)
        rows.append({'name': case['name'], 'origins': source.origins, 'placeholderReferences': source.references,
                     'strokeFields': source.fields, 'topologies': source.topologies, 'xsdParts': xsd_count})
    out = {'scope': __doc__.strip(), 'jsonSchemaChecks': schema_count, 'cases': rows,
           **{k: sum(r[k] for r in rows) for k in ['origins', 'placeholderReferences', 'strokeFields', 'topologies', 'xsdParts']}}
    args.output.write_text(json.dumps(out, indent=2) + '\n')
    print(json.dumps({k: v for k, v in out.items() if k != 'cases'}))


if __name__ == '__main__':
    main()
