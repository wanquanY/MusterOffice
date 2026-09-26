"""Independent checks of bytes actually downloaded through MCP resources."""
import hashlib
import io
import json
from pathlib import Path
import sys
import zipfile
from lxml import etree as X
from PIL import Image
from pptx import Presentation

root = Path(sys.argv[1]); output = Path(sys.argv[2]); assert not output.exists()
fixture = json.loads(Path('fixtures/presentations/delivery/input.json').read_text())
lock_path = Path('docs/reviews/evidence/2026-09-24-ecma-schema-inputs.json')
lock = json.loads(lock_path.read_text()); xsd = Path('.codex-work/ecma376/xsd')
sha = lambda data: hashlib.sha256(data).hexdigest()
for item in lock['files']: assert sha((xsd/item['name']).read_bytes()) == item['sha256']
safe = X.XMLParser(resolve_entities=False, no_network=True)
P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
ns = dict(p=P, a=A)
schemas = {P:X.XMLSchema(X.parse(str(xsd/'pml.xsd'), safe)), A:X.XMLSchema(X.parse(str(xsd/'dml-main.xsd'), safe))}
reports = []
for era in ['legacy', 'modern']:
    directory = root/f'{era}-candidate'
    bundle = json.loads((directory/'bundle.json').read_text())
    assets = {}
    for index, asset in enumerate(bundle['assets']):
        raw = (directory/f'{index:03}.bin').read_bytes()
        assert len(raw) == int(asset['byteLength']) and sha(raw) == asset['sha256']
        assets[asset['id']] = raw
    raw = assets[bundle['pptxAssetId']]; checked = []; identities = []; text = []
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        assert archive.testzip() is None
        for part in archive.namelist():
            if not part.endswith('.xml'): continue
            xml = X.fromstring(archive.read(part), safe)
            namespace = X.QName(xml).namespace
            if namespace in schemas: schemas[namespace].assertValid(xml); checked.append(part)
            for node in xml.xpath('//p:sp|//p:pic|//p:grpSp|//p:cxnSp', namespaces=ns):
                identities.extend(node.xpath('./*/p:cNvPr/@name', namespaces=ns))
                text.extend(node.xpath('./p:txBody/a:p/a:r/a:t/text()', namespaces=ns))
    assert len(identities) == len(set(identities)) == 15
    assert set(identities) == set(fixture['document']['objects'])
    assert text == ['A A']*8
    assert len(Presentation(io.BytesIO(raw)).slides) == 2
    pages = []
    for index, preview in enumerate(bundle['previews']):
        with Image.open(io.BytesIO(assets[preview['imageAssetId']])) as image:
            image.load()
            assert image.size == (640, 360) and image.mode == 'RGBA' and image.info['srgb'] == 0
            assert image.getpixel((620,340)) == (255,255,255,255)
            if index == 0:
                for xy, rgba in [((75,150),(27,114,232,255)), ((257,175),(38,185,154,255)), ((37,37),(22,34,56,255))]:
                    assert image.getpixel(xy) == rgba
            pages.append(dict(pageId=preview['pageId'], rgbaSha256=sha(image.tobytes()), pixels=640*360))
    assert len(pages) == 2
    reports.append(dict(era=era, pptxSha256=sha(raw), checkedParts=checked, nativeObjects=15, textRuns=8, pages=pages))
assert reports[0]['pptxSha256'] == reports[1]['pptxSha256']
assert reports[0]['pages'] == reports[1]['pages']
report = dict(files=reports, schemaInputs=dict(path=str(lock_path), sha256=sha(lock_path.read_bytes())),
    limitations='Self-owned synthetic font fixture. Independent structure/object/pixel probes, not full visual, Office/WPS editing/playback or product acceptance.')
output.write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(exports=2, pages=4, xsdParts=sum(len(r['checkedParts']) for r in reports), nativeObjectsPerExport=15, protocolBytesEqual=True)))
