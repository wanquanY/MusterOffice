"""Compare frozen/new writer outputs independently of either format reader."""
import hashlib
import json
from pathlib import Path
import zipfile

from lxml import etree as X


def entry(path):
    p = Path(path)
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())


root = Path('.codex-work/delivery-pipeline')
report = json.loads((root/'authored-parity.json').read_text())
xsd_root = Path('.codex-work/ecma376/xsd')
lock = json.loads(Path('docs/reviews/evidence/2026-09-24-ecma-schema-inputs.json').read_text())
for record in lock['files']:
    assert entry(xsd_root/record['name'])['sha256'] == record['sha256']
parser = X.XMLParser(resolve_entities=False, no_network=True)
schemas = {
    'http://schemas.openxmlformats.org/presentationml/2006/main': X.XMLSchema(X.parse(str(xsd_root/'pml.xsd'), parser)),
    'http://schemas.openxmlformats.org/drawingml/2006/main': X.XMLSchema(X.parse(str(xsd_root/'dml-main.xsd'), parser)),
}
cases = []
for case in report['cases']:
    if case['status'] != 'exported':
        continue
    old = root/'prior-authored'/f"{case['name']}.pptx"
    new = Path(report['artifactDirectory'])/f"{case['name']}.pptx"
    assert entry(new)['sha256'] == case['sha256']
    changes = []
    checked = []
    with zipfile.ZipFile(old) as a, zipfile.ZipFile(new) as b:
        assert a.testzip() is None and b.testzip() is None
        assert a.namelist() == b.namelist()
        for name in a.namelist():
            before, after = a.read(name), b.read(name)
            if before != after:
                assert name.endswith('.xml'), (case['name'], name)
                expected = before.replace(b'<a:defPPr>', b'<a:lvl1pPr>').replace(b'</a:defPPr>', b'</a:lvl1pPr>')
                assert expected == after, (case['name'], name)
                changes.append(name)
            if name.endswith('.xml'):
                xml = X.fromstring(after, parser)
                namespace = X.QName(xml).namespace
                if namespace in schemas:
                    schemas[namespace].assertValid(xml)
                    checked.append(name)
    assert changes and checked
    cases.append(dict(name=case['name'], before=entry(old), after=entry(new), changedParts=changes, xsdParts=checked))
assert len(cases) == 7
result = dict(format='musteroffice.delivery-style-delta/1', cases=cases,
              claim='Every decompressed byte is unchanged except defPPr to lvl1pPr element names; current PresentationML/DrawingML parts validate against locked XSD.',
              limitation='No external application editing, visual or full-feature acceptance.')
with (root/'style-delta.json').open('x') as stream:
    json.dump(result, stream, indent=2)
    stream.write('\n')
print(json.dumps(dict(cases=len(cases), xsdParts=sum(len(c['xsdParts']) for c in cases))))
