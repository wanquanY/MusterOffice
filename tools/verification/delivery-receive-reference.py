"""Independently inspect received public files, wire schemas and bundle digest."""
import hashlib
import io
import json
from pathlib import Path
import sys
from urllib.parse import unquote, urlparse
import jsonschema
from PIL import Image
from pptx import Presentation

directory = Path(sys.argv[1])
output = Path(sys.argv[2])
assert not output.exists()
read = lambda path: json.loads(Path(path).read_text())
sha = lambda data: hashlib.sha256(data).hexdigest()
def entry(path):
    path = Path(path); data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=sha(data))
request_schema = jsonschema.Draft202012Validator(read('contracts/generated/delivery-inspect-request.schema.json'))
response_schema = jsonschema.Draft202012Validator(read('contracts/generated/delivery-inspect-response.schema.json'))
parity = read(directory/'report.json')
invalid_schema_inputs = []
for case in parity['cases']:
    for key in ['input', 'bytes', 'response']:
        assert entry(case[key]['path']) == case[key]
    response = read(case['response']['path']); response_schema.validate(response)
    request = read(case['input']['path'])
    if list(request_schema.iter_errors(request)):
        invalid_schema_inputs.append(case['name'])
    if case['expected'] == 'inspected':
        bundle = request['bundle']
        canonical = json.dumps(bundle, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
        domain = b'musteroffice.delivery-bundle/1'
        digest = sha(len(domain).to_bytes(8, 'big') + domain + canonical)
        assert response['report']['bundleDigest'] == digest
        assert response['report']['declaredClaims'] == bundle['claims']
        assert response['report']['pages'] == 2 and response['report']['assetsVerified'] == 12
        assert response['report']['totalBytes'] == '69623'

fixture = Path('fixtures/presentations/delivery-receive')
request = read(fixture/'request.json'); data = (fixture/'assets.bin').read_bytes()
request_schema.validate(request)
assets = {b['assetId']: data[int(b['byteOffset']):int(b['byteOffset'])+int(b['byteLength'])] for b in request['contents']}
previous = read('.codex-work/operation-client/native-2/assets.json')
for asset, old in zip(request['bundle']['assets'], previous, strict=True):
    assert asset == old['asset']
    raw = assets[asset['id']]
    assert sha(raw) == asset['sha256'] and len(raw) == int(asset['byteLength'])
    uri = urlparse(old['candidate']['path'])
    path = Path(unquote(uri.path)) if uri.scheme == 'file' else Path(old['candidate']['path'])
    assert raw == path.read_bytes()
model = json.loads(assets[request['bundle']['document']['modelAssetId']])
pptx = assets[request['bundle']['pptxAssetId']]
assert len(Presentation(io.BytesIO(pptx)).slides) == 2
pages = []
for preview in request['bundle']['previews']:
    with Image.open(io.BytesIO(assets[preview['imageAssetId']])) as image:
        image.load(); assert image.mode == 'RGBA' and image.info['srgb'] == 0
        assert image.size == (preview['width'], preview['height'])
        rgba = image.tobytes()
        premultiplied = bytearray(len(rgba))
        for offset in range(0, len(rgba), 4):
            r, g, b, a = rgba[offset:offset+4]
            premultiplied[offset:offset+4] = bytes(((r*a+127)//255, (g*a+127)//255, (b*a+127)//255, a))
        evidence = [json.loads(assets[a['id']]) for a in request['bundle']['assets'] if a['mediaType'] == 'application/json']
        evidence = next(e for e in evidence if e['pageId'] == preview['pageId'])
        assert sha(premultiplied) == evidence['render']['page']['scene']['raster']['sha256']
        assert evidence['pptxSha256'] == sha(pptx)
        pages.append(dict(pageId=preview['pageId'], pngSha256=sha(assets[preview['imageAssetId']]), premultipliedSha256=sha(premultiplied)))
assert model['revision'] == request['expected']['revision']
assert model['semanticDigest'] == request['expected']['semanticDigest']
report = dict(cases=len(parity['cases']), responseSchemasChecked=len(parity['cases']),
    requestSchemaFailures=invalid_schema_inputs, fixtureAssetsUnchangedFromNativeSdk=len(assets),
    canonicalBundleDigestsChecked=3, independentPngPages=pages, pptxSha256=sha(pptx),
    limitations='Independent byte, schema, digest and pixel checks. No independent rendering, full XSD, Office/WPS or Musterwork acceptance.')
output.write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(cases=report['cases'], assets=len(assets), pngPages=len(pages))))
