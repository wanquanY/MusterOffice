"""Validate table fill provenance against XML independently of the Rust reader.

Checks every returned origin and consulted placeholder declaration, plus the
native schema and public contracts. This is not a visual/Office acceptance.
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


class Source:
    def __init__(self, package):
        parser = etree.XMLParser(resolve_entities=False, no_network=True)
        self.parts = {'/' + n: etree.fromstring(package.read(n), parser)
                      for n in package.namelist() if n.endswith('.xml')}
        self.nodes = {p: list(root.iter()) for p, root in self.parts.items()}
        self.origins = 0
        self.references = 0

    def anchor(self, owner):
        part, target = owner['part'], owner['target']
        frame = next(n for n in self.parts[part].findall('.//p:graphicFrame', NS)
                     if int(n.find('p:nvGraphicFramePr/p:cNvPr', NS).get('id')) == target['nativeId'])
        table = frame.find('.//a:tbl', NS)
        props = table.find('a:tblPr', NS)
        if target['kind'] == 'tableCell':
            cell = target['cell']
            return part, table.findall('a:tr', NS)[cell['row']].findall('a:tc', NS)[cell['column']]
        if target['kind'] == 'tableBackground':
            return part, props
        assert target['kind'] == 'tableStyleFill'
        style = props.find('a:tableStyle', NS)
        if style is None:
            key = props.find('a:tableStyleId', NS).text.upper()
            part = '/ppt/tableStyles.xml'
            style = next(n for n in self.parts[part].findall('a:tblStyle', NS) if n.get('styleId').upper() == key)
        region = target.get('region') or 'tblBg'
        return part, style.find('a:' + region, NS)

    def origin(self, origin):
        kind = origin['kind']
        if kind == 'profileDefault':
            return
        if kind in ['declaration', 'tableStyle']:
            part, anchor = self.anchor(origin['owner' if kind == 'declaration' else 'via'])
            if kind == 'tableStyle':
                assert part == origin['part']
            assert self.nodes[part][origin['sourceOrdinal']] in anchor.iter()
        elif kind == 'theme':
            part, anchor = self.anchor(origin['via'])
            ref = self.nodes[part][origin['referenceOrdinal']]
            assert ref in anchor.iter() and ref.tag == '{' + NS['a'] + '}fillRef'
            assert int(ref.get('idx')) == origin['styleIndex']
            index = origin['styleIndex']
            scheme = self.parts[origin['part']].find('.//a:fmtScheme', NS)
            entries = scheme.find('a:' + ('bgFillStyleLst' if index > 1000 else 'fillStyleLst'), NS)
            selected = entries[index - (1001 if index > 1000 else 1)]
            assert self.nodes[origin['part']][origin['sourceOrdinal']] in selected.iter()
        elif kind == 'schemaDefault':
            assert 0 <= origin['sourceOrdinal'] < len(self.nodes[origin['part']])
        else:
            raise AssertionError(kind)
        self.origins += 1

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
                assert ref in anchor.iter() and ref.tag == '{' + NS['a'] + '}fillRef'
                if ordinal := placeholder.get('colorOrdinal'):
                    assert self.nodes[part][ordinal] in ref
                self.references += 1
            for child in value.values():
                self.walk(child)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['corpus-dir', 'parity-dir', 'schema-dir', 'contracts-dir', 'output']:
        parser.add_argument('--' + key, required=True, type=Path)
    args = parser.parse_args()
    assert not args.output.exists()
    xsd = etree.XMLSchema(etree.parse(str(args.schema_dir / 'pml.xsd')))
    dml = etree.XMLSchema(etree.parse(str(args.schema_dir / 'dml-main.xsd')))
    validators = {n: Draft202012Validator(json.loads((args.contracts_dir / ('pptx-fill-color-' + n + '.schema.json')).read_text())) for n in ['query', 'response']}
    schema_count = 0
    rows = []
    for case in json.loads((args.parity_dir / 'verification.json').read_text())['cases']:
        name = case['name']
        path = args.corpus_dir / (name + '.pptx')
        assert hashlib.sha256(path.read_bytes()).hexdigest() == case['sourceSha256']
        with zipfile.ZipFile(path) as package:
            source = Source(package)
        xsd_count = 0
        for part, root in source.parts.items():
            if root.tag.startswith('{' + NS['p'] + '}'):
                xsd.assertValid(root)
                xsd_count += 1
            elif root.tag in ['{' + NS['a'] + '}theme', '{' + NS['a'] + '}tblStyleLst']:
                dml.assertValid(root)
                xsd_count += 1
        responses = sorted(args.parity_dir.glob(name + '-*.response.json'))
        for path in responses:
            response = json.loads(path.read_text())
            query = json.loads(path.with_name(path.name.replace('.response.json', '.request.json')).read_text())
            validators['query'].validate(query)
            validators['response'].validate(response)
            schema_count += 2
            source.walk(response)
        for suffix, kind in [('stale-request', 'query'), ('stale-response', 'response')]:
            validators[kind].validate(json.loads((args.parity_dir / (name + '.' + suffix + '.json')).read_text()))
            schema_count += 1
        rows.append({'name': name, 'origins': source.origins, 'placeholderReferences': source.references, 'xsdParts': xsd_count})
    out = {'scope': 'XML/XSD provenance and public contracts for native table fill queries; no rendering',
           'jsonSchemaChecks': schema_count, 'xsdParts': sum(r['xsdParts'] for r in rows),
           'origins': sum(r['origins'] for r in rows), 'placeholderReferences': sum(r['placeholderReferences'] for r in rows), 'cases': rows}
    args.output.write_text(json.dumps(out, indent=2) + '\n')
    print(json.dumps({k: v for k, v in out.items() if k != 'cases'}))


if __name__ == '__main__':
    main()
