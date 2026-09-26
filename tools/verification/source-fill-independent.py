"""Independent source fill XML, physical ordinal, XSD and text edit checks."""
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import project, A, P
from fill_reference import declarations, FILL_NAMES
from line_reference import line

ROOT = Path('.codex-work/source-fills'); XSD = Path('.codex-work/ecma376/xsd')
manifest = json.loads((ROOT/'manifest.json').read_text()); parity = json.loads((ROOT/'parity.json').read_text())
lookup = {c['name']: c for c in parity['cases']}; NS = {'a': A, 'p': P}
schemas = {name:E.XMLSchema(E.parse(str(XSD/file), E.XMLParser(resolve_entities=False, no_network=True))) for name,file in [('surface','pml.xsd'), ('theme','dml-main.xsd')]}
sha = lambda b: hashlib.sha256(b).hexdigest()
checks = []; xsd_count = negative_xsd = semantic_range = declaration_count = 0
def first_fill(parent):
    if parent is None: return None
    return next((n for n in parent if E.QName(n).namespace == A and E.QName(n).localname in FILL_NAMES), None)
def verify(actual, node, ordinals):
    global declaration_count
    expected = None if node is None else declarations(node, ordinals)
    assert actual == expected, (actual, expected)
    if node is not None: declaration_count += 1
for case in manifest['cases']:
    item = lookup['inspect-'+case['name']]; b = Path(item['responsePath']).read_bytes(); assert sha(b) == item['responseSha256']; response = json.loads(b)
    with zipfile.ZipFile(case['path']) as z: raw = {n:z.read(n) for n in z.namelist()}
    if case['error']:
        assert response['status'] == 'error'; schema = schemas['theme' if case['part'].startswith('/ppt/theme/') else 'surface']
        dom = project(raw[case['part'].lstrip('/')])[0]
        if case.get('semanticRangeRejection'):
            schema.assertValid(dom)
            position = dom.find('.//a:gs', NS).get('pos')
            assert position.endswith('%') and not 0 <= Decimal(position[:-1]) <= 100
            semantic_range += 1
        else:
            assert not schema.validate(dom), case['name']; negative_xsd += 1
        continue
    index = response['index']; begin = declaration_count; parts = 0
    for part, surface in index['surfaces'].items():
        dom, _, ordinals = project(raw[part.lstrip('/')], with_ordinals=True)
        verify(surface.get('background'), dom.find('p:cSld/p:bg', NS), ordinals)
        verify(surface.get('rootGroupFill'), first_fill(dom.find('p:cSld/p:spTree/p:grpSpPr', NS)), ordinals)
        for node in dom.iter():
            if E.QName(node).namespace != P or E.QName(node).localname not in ['sp','pic','cxnSp','grpSp']: continue
            if any(E.QName(p).localname == 'extLst' for p in node.iterancestors()): continue
            identity = node.find('./*/p:cNvPr', NS); assert identity is not None
            obj = next(o for o in surface['objects'] if o['nativeId'] == int(identity.get('id')))
            properties = node.find('p:grpSpPr' if E.QName(node).localname == 'grpSp' else 'p:spPr', NS)
            verify(obj.get('fill'), first_fill(properties), ordinals)
            verify(obj.get('fillReference'), node.find('p:style/a:fillRef', NS), ordinals)
            verify(obj.get('pictureFill'), node.find('p:blipFill', NS), ordinals)
            ln = None if properties is None else properties.find('a:ln', NS)
            assert obj.get('line') == (None if ln is None else line(ln, ordinals)), case['name']
        if case['xsdConforming'] or part != case['part']: schemas['surface'].assertValid(dom); parts += 1
    for part, theme in index['themes'].items():
        dom, _, ordinals = project(raw[part.lstrip('/')], with_ordinals=True)
        fmt = theme['formatScheme']
        if fmt:
            for family in ['fills', 'backgroundFills']:
                for entry in fmt[family]:
                    node = next(n for n in dom.iter() if ordinals[n] == entry['sourceOrdinal']); verify(entry.get('fill'), node, ordinals)
            for entry in fmt['lines']:
                node = next(n for n in dom.iter() if ordinals[n] == entry['sourceOrdinal']); assert entry['line'] == line(node, ordinals)
        if case['xsdConforming'] or part != case['part']: schemas['theme'].assertValid(dom); parts += 1
    edited = lookup['edit-'+case['name']]; candidate = Path(edited['pptxPath']).read_bytes(); assert sha(candidate) == edited['pptxSha256']
    with zipfile.ZipFile(edited['pptxPath']) as z: after = {n:z.read(n) for n in z.namelist()}
    assert raw.keys() == after.keys()
    for part, b in raw.items():
        if part != 'ppt/slides/slide1.xml': assert b == after[part], (case['name'], part)
    before_dom = E.fromstring(raw['ppt/slides/slide1.xml']); after_dom = E.fromstring(after['ppt/slides/slide1.xml'])
    path = 'p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r/a:t'; a = before_dom.find(path, NS); b = after_dom.find(path, NS)
    assert b.text == 'fill source preserved 中文 & < >'; b.text = a.text
    assert E.tostring(before_dom) == E.tostring(after_dom), case['name']
    xsd_count += parts; checks.append({'name':case['name'], 'declarations':declaration_count-begin, 'xsdParts':parts, 'sourceSha256':case['sha256'], 'nonTextXmlAndOtherPartsPreserved':True})
result = {'format':'musteroffice.source-fill-independent/1', 'cases':checks, 'fillDeclarations':declaration_count, 'validXsdParts':xsd_count, 'negativeXsdCases':negative_xsd, 'semanticRangeRejections':semantic_range, 'xsdInputs':[{'path':str(p), 'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))], 'scope':'Source declarations and physical bindings only. No brush inheritance, imported-page painting or application interoperability acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result, indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['cases','xsdInputs','scope']}))
