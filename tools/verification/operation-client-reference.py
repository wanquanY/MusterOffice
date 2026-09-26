"""Independent schema, persisted-state, PPTX and pixel checks of SDK outputs."""
from contextlib import closing
import hashlib
import io
import json
from pathlib import Path
import sqlite3
import sys
from urllib.parse import unquote, urlparse
import zipfile
import jsonschema
from lxml import etree as X
from PIL import Image
from pptx import Presentation

root=Path(sys.argv[1]); output=Path(sys.argv[2]); assert not output.exists()
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(Path(p).read_text())
def path(value):
    parsed=urlparse(value)
    return Path(unquote(parsed.path)) if parsed.scheme=='file' else Path(value)
def validator(name):return jsonschema.Draft202012Validator(read('contracts/generated/'+name+'.schema.json'))
validators={n:validator(n) for n in ['host-request','host-response','presentation-delivery-bundle']}
calls=0
for name in ['initial','reconnected','denied']:
    for call in read(root/f'{name}.calls.json'):
        validators['host-request'].validate(call['request']);validators['host-response'].validate(call['response']);calls+=1
    for call in read(root/f'{name}.binary.json'):
        if call['kind']=='append':validators['host-response'].validate(call['response'])
    assert not (root/f'{name}.stderr').read_bytes()
report=read(root/'report.json'); assert report['controlCalls']==calls
bundle=read(root/'bundle.json');validators['presentation-delivery-bundle'].validate(bundle)
listed=read(root/'assets.json');assert len(listed)==len(bundle['assets'])==12
assets={}
for record, asset in zip(listed,bundle['assets']):
    assert record['asset']==asset
    location=path(record['candidate']['path']).resolve()
    assert location.is_relative_to((root/'candidate').resolve())
    raw=location.read_bytes();assert len(raw)==int(asset['byteLength'])==record['candidate']['byteLength']
    assert sha(raw)==asset['sha256']==record['candidate']['sha256'];assets[asset['id']]=raw
model= json.loads(assets[bundle['document']['modelAssetId']])
assert model['document']['id']==bundle['document']['documentId']==report['initial']['documentId']
assert model['document']['title']=='SDK real integration'
assert model['revision']==bundle['document']['revision']==report['edited']['revision']
assert model['semanticDigest']==report['edited']['semanticDigest']
with closing(sqlite3.connect((root/'host.sqlite').resolve().as_uri()+'?mode=ro&immutable=1',uri=True)) as conn:
    assert conn.execute('SELECT count(*) FROM jobs').fetchone()[0]==3
    assert conn.execute('SELECT count(*) FROM heads').fetchone()[0]==1
    assert conn.execute('SELECT count(*) FROM revisions').fetchone()[0]==2
    for row, in conn.execute('SELECT info FROM jobs'):
        job=json.loads(row);assert job['state']=='succeeded' and job['cancelRequested'] is False
    large=json.loads(conn.execute("SELECT info FROM asset_uploads WHERE request_id='large'").fetchone()[0])
    assert large['receivedBytes']=='786469' and large['state']=='sealed'
    assert large['asset']==report['largeAsset']

lock_path=Path('docs/reviews/evidence/2026-09-24-ecma-schema-inputs.json')
lock=read(lock_path);xsd=Path('.codex-work/ecma376/xsd')
for item in lock['files']:assert sha((xsd/item['name']).read_bytes())==item['sha256']
safe=X.XMLParser(resolve_entities=False,no_network=True)
P='http://schemas.openxmlformats.org/presentationml/2006/main'
A='http://schemas.openxmlformats.org/drawingml/2006/main'
schemas={P:X.XMLSchema(X.parse(str(xsd/'pml.xsd'),safe)),A:X.XMLSchema(X.parse(str(xsd/'dml-main.xsd'),safe))}
pptx=assets[bundle['pptxAssetId']];checked=[];identities=[];text=[]
with zipfile.ZipFile(io.BytesIO(pptx)) as archive:
    assert archive.testzip() is None
    for part in archive.namelist():
        if not part.endswith('.xml'):continue
        xml=X.fromstring(archive.read(part),safe);namespace=X.QName(xml).namespace
        if namespace in schemas:schemas[namespace].assertValid(xml);checked.append(part)
        for node in xml.xpath('//p:sp|//p:pic|//p:grpSp|//p:cxnSp',namespaces={'p':P,'a':A}):
            identities.extend(node.xpath('./*/p:cNvPr/@name',namespaces={'p':P}))
            text.extend(node.xpath('./p:txBody/a:p/a:r/a:t/text()',namespaces={'p':P,'a':A}))
assert len(checked)==10 and len(identities)==len(set(identities))==15
assert set(identities)==set(model['document']['objects']) and text==['A A']*8
assert len(Presentation(io.BytesIO(pptx)).slides)==2
pages=[]
for i,preview in enumerate(bundle['previews']):
    with Image.open(io.BytesIO(assets[preview['imageAssetId']])) as image:
        image.load();assert image.size==(640,360) and image.mode=='RGBA' and image.info['srgb']==0
        assert image.getpixel((620,340))==(255,255,255,255)
        if i==0:
            for xy,rgba in [((75,150),(27,114,232,255)),((257,175),(38,185,154,255)),((37,37),(22,34,56,255))]:
                assert image.getpixel(xy)==rgba
        pages.append(dict(pageId=preview['pageId'],rgbaSha256=sha(image.tobytes()),pixels=640*360))
assert len(pages)==2
for claim in bundle['claims']:
    assert claim['subjectSha256']==sha(pptx) and set(claim['evidenceAssetIds'])<=set(assets)
    assert claim['status']==('passed' if claim['kind']=='structure' else 'not_proven')
# Same source page content; only document metadata/title changed in this SDK run.
previous=read('.codex-work/mcp-recovery/file-checks-final.json')['files'][0]
assert pages==previous['pages']
result=dict(controlCallsSchemaChecked=calls,assetsVerified=12,checkedParts=checked,nativeObjects=15,textRuns=8,
    pages=pages,pptxSha256=sha(pptx),samePreviewPixelsAsMcp=True,persistedJobs=3,persistedRevisions=2,
    largeUploadBytes=786469,schemaInputs=dict(path=str(lock_path),sha256=sha(lock_path.read_bytes())),
    limitations='Actual SDK/native-host transfer and owned fixture checks. Not complete visual fidelity, Office/WPS, browser/Musterwork or publication acceptance.')
output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(controlCalls=calls,assets=12,xsdParts=10,nativeObjects=15,pages=2,storedJobs=3)))
