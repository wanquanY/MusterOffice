"""Independent ZIP/lxml theme oracle for successful owned source fixtures.

Checks declarations, physical source ordinals, family overrides and relationship
provenance. It does not evaluate colors, select installed fonts or render effects.
"""
import argparse
from functools import lru_cache
import hashlib
import json
from pathlib import Path, PurePosixPath
import posixpath
import zipfile
from lxml import etree as E
from mce_reference import project, A, R

NS = {'a': A}


from drawingml_reference import local, color
from line_reference import line

def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def font(node):
    return {'typeface': node.get('typeface'), 'panose': node.get('panose').strip() if node.get('panose') is not None else None,
            'pitchFamily': int(node.get('pitchFamily')) if node.get('pitchFamily') is not None else None,
            'charset': int(node.get('charset')) if node.get('charset') is not None else None}


def collection(node):
    result = {target: font(node.find('a:' + child, NS)) for target, child in [('latin', 'latin'), ('eastAsian', 'ea'), ('complexScript', 'cs')]}
    result['supplemental'] = [{'script': n.get('script'), 'typeface': n.get('typeface')} for n in node.findall('a:font', NS)]
    return result


def theme(raw, expected_kind):
    root, compatibility, ordinals = project(raw, with_ordinals=True)
    assert root.tag == E.QName(A, 'theme' if expected_kind == 'theme' else 'themeOverride')
    parent = root.find('a:themeElements', NS) if expected_kind == 'theme' else root
    result = {'sha256': digest(raw), 'kind': expected_kind, 'name': root.get('name'),
              'colorScheme': None, 'fontScheme': None, 'formatScheme': None, 'compatibility': compatibility}
    colors = parent.find('a:clrScheme', NS)
    if colors is not None:
        result['colorScheme'] = {'sourceOrdinal': ordinals[colors], 'name': colors.get('name'),
                                 'colors': {local(slot): color(slot[0], ordinals) for slot in colors if slot.tag != E.QName(A, 'extLst')}}
    fonts = parent.find('a:fontScheme', NS)
    if fonts is not None:
        result['fontScheme'] = {'sourceOrdinal': ordinals[fonts], 'name': fonts.get('name'),
                                'major': collection(fonts.find('a:majorFont', NS)), 'minor': collection(fonts.find('a:minorFont', NS))}
    styles = parent.find('a:fmtScheme', NS)
    if styles is not None:
        result['formatScheme'] = {'sourceOrdinal': ordinals[styles], 'name': styles.get('name')}
        for target, source in [('fills', 'fillStyleLst'), ('lines', 'lnStyleLst'), ('effects', 'effectStyleLst'), ('backgroundFills', 'bgFillStyleLst')]:
            result['formatScheme'][target] = [{'sourceOrdinal': ordinals[n], 'localName': local(n), **({'line':line(n,ordinals)} if target=='lines' else {})} for n in styles.find('a:' + source, NS)]
    return result


def verify(case, schema=None):
    index = case['response']['index']
    with zipfile.ZipFile(case['source']) as package:
        raw = {n: package.read(n) for n in package.namelist()}
    links = {}; roles = {}
    for part, surface in index['surfaces'].items():
        path = PurePosixPath(part)
        relpath = str(path.parent / '_rels' / (path.name + '.rels')).lstrip('/')
        current = {'parent': None, 'base': None, 'override': None}
        if relpath in raw:
            for rel in E.fromstring(raw[relpath]):
                if rel.get('TargetMode') == 'External':
                    continue
                kind = rel.get('Type').removeprefix(R + '/')
                target = posixpath.normpath(posixpath.join(str(path.parent), rel.get('Target')))
                if kind == ('slideLayout' if surface['kind'] == 'slide' else 'slideMaster') and surface['kind'] != 'master':
                    current['parent'] = target
                if kind in ['theme', 'themeOverride']:
                    role = 'theme' if kind == 'theme' else 'override'
                    roles[target] = role
                    current['base' if kind == 'theme' else 'override'] = target
        links[part] = current
    expected = {part: theme(raw[part.lstrip('/')], role) for part, role in roles.items()}
    assert expected.keys() == index['themes'].keys(), case['name']
    schema_checks = []; schema_exclusions = []
    for part, result in expected.items():
        actual = {key: value for key, value in index['themes'][part].items() if key != 'notices'}
        assert result == actual, (case['name'], part, result, actual)
        if schema is not None:
            # Deliberately out-of-schema metadata probes tolerant preservation.
            # It is never presented as a conforming DrawingML theme input.
            if case['name'] == 'inspect-theme-mce-retained' and part == '/ppt/theme/theme2.xml':
                schema_exclusions.append({'part': part, 'reason': 'Deliberate unknown root declaration retained with a notice; not XSD-conforming.'})
            else:
                schema.assertValid(project(raw[part.lstrip('/')])[0])
                schema_checks.append(part)

    @lru_cache(None)
    def selected(part):
        link = links[part]
        chain = selected(link['parent'])[0] if link['parent'] else []
        chain = [link['base']] if link['base'] else list(chain)
        if link['override']:
            chain.append(link['override'])
        selection = {'colors': None, 'fonts': None, 'format': None}
        for declared in chain:
            for family, key in [('colors', 'colorScheme'), ('fonts', 'fontScheme'), ('format', 'formatScheme')]:
                scheme = expected[declared][key]
                if scheme is not None:
                    selection[family] = {'part': declared, 'sourceOrdinal': scheme['sourceOrdinal']}
        return chain, selection

    for part, surface in index['surfaces'].items():
        assert selected(part)[1] == surface['themeSelection'], (case['name'], part)
    colors = [c for t in expected.values() if t['colorScheme'] for c in t['colorScheme']['colors'].values()]
    return {'name': case['name'], 'sourceSha256': index['sourceSha256'], 'themeParts': len(expected),
            'surfaceSelections': len(links), 'colors': len(colors), 'transforms': sum(len(c['transforms']) for c in colors),
            'colorKinds': sorted({c['value']['kind'] for c in colors}),
            'transformKinds': sorted({t['kind'] for c in colors for t in c['transforms']}),
            'schemasChecked': schema_checks, 'schemaExclusions': schema_exclusions}


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('parity_report', type=Path)
    parser.add_argument('--xsd-dir', type=Path)
    args = parser.parse_args(); report = json.loads(args.parity_report.read_text())
    schema = E.XMLSchema(E.parse(str(args.xsd_dir / 'dml-main.xsd'), E.XMLParser(resolve_entities=False, no_network=True))) if args.xsd_dir else None
    cases = [verify(c, schema) for c in report['cases'] if c.get('response', {}).get('status') == 'inspected']
    facts = []
    if args.xsd_dir:
        path = args.xsd_dir / 'dml-main.xsd'; xsd = E.fromstring(path.read_bytes())
        definitions = json.loads(Path('contracts/generated/pptx-source-response.schema.json').read_text())['$defs']
        for schema, native in [('PresetColor', 'ST_PresetColorVal'), ('SystemColor', 'ST_SystemColorVal'), ('SchemeColor', 'ST_SchemeColorVal')]:
            values = xsd.xpath('//x:simpleType[@name=$name]/x:restriction/x:enumeration/@value', namespaces={'x': 'http://www.w3.org/2001/XMLSchema'}, name=native)
            assert set(definitions[schema]['enum']) == set(values), (schema, native)
            facts.append({'schema': schema, 'nativeType': native, 'values': len(values), 'xsdSha256': digest(path.read_bytes())})
    print(json.dumps({'format': 'musteroffice.theme-independent/1', 'passed': len(cases), 'cases': cases, 'officialEnumChecks': facts,
                      'scope': 'Declared theme data and independent family source selection; not computed RGBA, font layout, rendering or Office/WPS acceptance.'}, indent=2))


if __name__ == '__main__':
    main()
