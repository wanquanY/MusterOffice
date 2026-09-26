"""Independent XML/relationships check of declared and inherited color maps."""
import argparse
from functools import lru_cache
import hashlib
import json
from pathlib import Path, PurePosixPath
import posixpath
import zipfile
from lxml import etree as E
from mce_reference import project, P, A, R

NS = {'p': P, 'a': A}
SLOTS = ['bg1', 'tx1', 'bg2', 'tx2', 'accent1', 'accent2', 'accent3', 'accent4', 'accent5', 'accent6', 'hlink', 'folHlink']


def verify(case, schema):
    index = case['response']['index']; maps = {}; parents = {}; checked = []; excluded = []
    with zipfile.ZipFile(case['source']) as package:
        for part, surface in index['surfaces'].items():
            raw = package.read(part.lstrip('/')); root, _, ordinals = project(raw, with_ordinals=True)
            master = root.tag == E.QName(P, 'sldMaster')
            node = root.find('p:clrMap', NS) if master else root.find('p:clrMapOvr/a:overrideClrMapping', NS)
            if node is not None:
                value = {'kind': 'explicit', 'sourceOrdinal': ordinals[node], 'mapping': {slot: node.get(slot).strip() for slot in SLOTS}}
            else:
                marker = root.find('p:clrMapOvr/a:masterClrMapping', NS)
                value = None if marker is None else {'kind': 'master', 'sourceOrdinal': ordinals[marker]}
            assert value == surface['colorMapping'], (case['name'], part, value, surface['colorMapping'])
            maps[part] = value
            path = PurePosixPath(part); relpath = str(path.parent / '_rels' / (path.name + '.rels')).lstrip('/')
            parents[part] = None
            if not master and relpath in package.namelist():
                kind = 'slideLayout' if root.tag == E.QName(P, 'sld') else 'slideMaster'
                for rel in E.fromstring(package.read(relpath)):
                    if rel.get('Type') == R + '/' + kind and rel.get('TargetMode') != 'External':
                        parents[part] = posixpath.normpath(posixpath.join(str(path.parent), rel.get('Target')))
            # Other source cases contain deliberate out-of-schema preserved data.
            # Validate all surfaces only in the new dedicated color-map corpus.
            if schema is not None and case['name'].startswith('inspect-color-map-'):
                if case['name'] == 'inspect-color-map-missing-master-map' and part == '/ppt/slideMasters/slideMaster2.xml':
                    excluded.append({'part': part, 'reason': 'Deliberate missing master map; returned unresolved, not XSD-conforming.'})
                else:
                    schema.assertValid(root); checked.append(part)

    @lru_cache(None)
    def effective(part):
        declaration = maps[part]
        if declaration and declaration['kind'] == 'explicit':
            return {'part': part, 'sourceOrdinal': declaration['sourceOrdinal']}
        return effective(parents[part]) if parents[part] else None

    for part, surface in index['surfaces'].items():
        assert effective(part) == surface['resolvedColorMapping'], (case['name'], part)
    return {'name': case['name'], 'sourceSha256': index['sourceSha256'], 'surfaceDeclarations': len(maps),
            'explicitMaps': sum(m is not None and m['kind'] == 'explicit' for m in maps.values()),
            'mappingAttributesCompared': 12 * sum(m is not None and m['kind'] == 'explicit' for m in maps.values()),
            'resolvedSurfaces': sum(effective(p) is not None for p in maps), 'schemasChecked': checked, 'schemaExclusions': excluded}


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('parity_report', type=Path); parser.add_argument('--xsd-dir', type=Path)
    args = parser.parse_args(); report = json.loads(args.parity_report.read_text())
    schema = E.XMLSchema(E.parse(str(args.xsd_dir / 'pml.xsd'), E.XMLParser(resolve_entities=False, no_network=True))) if args.xsd_dir else None
    cases = [verify(c, schema) for c in report['cases'] if c.get('response', {}).get('status') == 'inspected']
    print(json.dumps({'format': 'musteroffice.color-map-independent/1', 'passed': len(cases), 'cases': cases,
                      'scope': 'Source declarations, physical bindings and nearest parent mapping. Independent target-application samples are recorded separately; no general Office/WPS certification.'}, indent=2))


if __name__ == '__main__':
    main()
