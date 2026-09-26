"""Verify real source bindings, ECMA grammar and native edit preservation."""
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import project, A, P
from geometry_reference import geometry

ROOT = Path('.codex-work/source-geometry')
XSD = Path('.codex-work/ecma376/xsd')
report = json.loads((ROOT / 'parity.json').read_text())
manifest = json.loads((ROOT / 'manifest.json').read_text())
lookup = {c['name']: c for c in report['cases']}
NS = {'p': P, 'a': A}
schema = E.XMLSchema(E.parse(str(XSD / 'pml.xsd'), E.XMLParser(resolve_entities=False, no_network=True)))
checks = []
count = xsd_count = negative_xsd = 0
coverage = {k: set() for k in ['presets', 'commands', 'handles', 'fill', 'formulas']}
def sha(b): return hashlib.sha256(b).hexdigest()
for c in manifest['cases']:
    record = lookup['inspect-' + c['name']]
    response = Path(record['responsePath']).read_bytes()
    assert sha(response) == record['responseSha256']
    r = json.loads(response)
    assert sha(Path(c['path']).read_bytes()) == c['sha256']
    with zipfile.ZipFile(c['path']) as z: raw = {n: z.read(n) for n in z.namelist()}
    if c['error']:
        assert r['error']['code'] == c['error']
        assert not schema.validate(project(raw[c['part'].lstrip('/')])[0]), c['name']
        negative_xsd += 1
        continue
    geometries = parts = 0
    for part, surface in r['index']['surfaces'].items():
        tree, compatibility, ordinals = project(raw[part.lstrip('/')], with_ordinals=True)
        assert compatibility == surface['compatibility']
        for node in tree.findall('.//p:sp', NS) + tree.findall('.//p:pic', NS) + tree.findall('.//p:cxnSp', NS):
            if any(E.QName(a).localname == 'extLst' for a in node.iterancestors()): continue
            identity = node.find('./*/p:cNvPr', NS)
            obj = next(o for o in surface['objects'] if o['nativeId'] == int(identity.get('id')))
            props = node.find('p:spPr', NS)
            found = [] if props is None else [n for n in props if E.QName(n).localname in ['prstGeom', 'custGeom'] and E.QName(n).namespace == A]
            assert len(found) <= 1
            expected = geometry(found[0], ordinals) if found else None
            assert obj.get('geometry') == expected, (c['name'], part, expected, obj.get('geometry'))
            if not found: continue
            geometries += 1
            for n in found[0].iter():
                local = E.QName(n).localname
                if local == 'prstGeom': coverage['presets'].add(n.get('prst').strip())
                if local in ['moveTo', 'lnTo', 'arcTo', 'quadBezTo', 'cubicBezTo', 'close']: coverage['commands'].add(local)
                if local in ['ahXY', 'ahPolar']: coverage['handles'].add(local)
                if local == 'path' and 'fill' in n.attrib: coverage['fill'].add(n.get('fill'))
                if local == 'gd' and n.get('fmla').split(): coverage['formulas'].add(n.get('fmla').split()[0])
        if c['xsdConforming'] or part != c['part']:
            schema.assertValid(tree)
            parts += 1
    edit = lookup['edit-' + c['name']]
    assert sha(Path(edit['pptxPath']).read_bytes()) == edit['pptxSha256']
    with zipfile.ZipFile(edit['pptxPath']) as z: after = {n: z.read(n) for n in z.namelist()}
    assert raw.keys() == after.keys()
    for part, b in raw.items():
        if part != 'ppt/slides/slide1.xml': assert b == after[part], (c['name'], part)
    before_tree = E.fromstring(raw['ppt/slides/slide1.xml'])
    after_tree = E.fromstring(after['ppt/slides/slide1.xml'])
    path = 'p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r/a:t'
    a = before_tree.find(path, NS)
    b = after_tree.find(path, NS)
    assert b.text == 'geometry preserved 中文 & < >'
    b.text = a.text
    assert E.tostring(before_tree) == E.tostring(after_tree), c['name']
    # Re-read both the original and candidate XML with independent bindings.
    candidate = json.loads(Path(edit['responsePath']).read_text())
    assert sha(Path(edit['responsePath']).read_bytes()) == edit['responseSha256']
    for part, s in r['index']['surfaces'].items():
        assert [o.get('geometry') for o in s['objects']] == [o.get('geometry') for o in candidate['index']['surfaces'][part]['objects']]
    count += geometries
    xsd_count += parts
    checks.append({'name': c['name'], 'sourceSha256': c['sha256'], 'responseSha256': record['responseSha256'], 'geometries': geometries,
                   'xsdParts': parts, 'editSha256': edit['pptxSha256'], 'geometryXmlAndOtherPartsPreserved': True})
result = {'format': 'musteroffice.source-geometry-independent/1', 'cases': checks, 'geometryDeclarations': count,
          'validXsdParts': xsd_count, 'negativeXsdCases': negative_xsd, 'intentionalXsdExclusions': [c['name'] for c in manifest['cases'] if not c['error'] and not c['xsdConforming']],
          'coverage': {k: sorted(v) for k, v in coverage.items()},
          'xsdInputs': [{'path': str(p), 'sha256': sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],
          'scope': 'Source declarations and physical bindings only; no formula execution, geometry evaluation, preset expansion or rendering acceptance.'}
(ROOT / 'independent.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'cases': len(checks), 'geometries': count, 'xsdParts': xsd_count, 'negativeXsdCases': negative_xsd,
                  'coverage': {k: len(v) for k, v in coverage.items()}}))
