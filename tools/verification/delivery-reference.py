"""Independent stored-byte, JSON closure, PNG, native-object and XSD checks.
No renderer metadata is used to invent expected content or visual fidelity.
"""
import argparse
import hashlib
import importlib.util
import json
import struct
import zipfile
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from lxml import etree as X
from PIL import Image
from pptx import Presentation

parser = argparse.ArgumentParser()
parser.add_argument('directory', type=Path)
args = parser.parse_args()
root = args.directory
def sha(b): return hashlib.sha256(b).hexdigest()
def entry(p):
    p = Path(p); b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=sha(b))
def digest(domain, value):
    d = domain.encode()
    return sha(struct.pack('>Q', len(d)) + d + json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode())

fixture_path = Path('fixtures/presentations/delivery/input.json')
fixture = json.loads(fixture_path.read_text())
bundle = json.loads((root/'bundle.json').read_text())
files = json.loads((root/'files.json').read_text())
assert len(files) == len(bundle['assets']) == 12
assert len({f['name'] for f in files}) == len(files)
assert len({f['asset']['id'] for f in files}) == len(files)
by_name = {f['name']: f for f in files}
by_id = {f['asset']['id']: f for f in files}
for f in files:
    assert Path(f['file']).name == f['file']
    b = (root/f['file']).read_bytes()
    assert sha(b) == f['asset']['sha256']
    assert len(b) == int(f['asset']['byteLength'])
    assert f['asset'] in bundle['assets']
def data(name): return (root/by_name[name]['file']).read_bytes()
def document(name): return json.loads(data(name))

common = json.loads(Path('docs/contracts/common.schema.json').read_text())
schema = json.loads(Path('docs/contracts/delivery.schema.json').read_text())
registry = Registry().with_resource(common['$id'], Resource.from_contents(common))
Draft202012Validator(schema, registry=registry).validate(bundle)
Draft202012Validator(json.loads(Path('contracts/generated/presentation-delivery-bundle.schema.json').read_text())).validate(bundle)
model = document('model')
# Model defaults may add fields; the exact source input still roundtrips through
# the document validator/serializer rather than losing authored declarations.
def compare_authored(expected, actual):
    if isinstance(expected, dict):
        assert isinstance(actual, dict) and expected.keys() <= actual.keys()
        for k, v in expected.items(): compare_authored(v, actual[k])
        for k in actual.keys() - expected.keys(): assert actual[k] == {'kind':'inherit'}, (k, actual[k])
    elif isinstance(expected, list):
        assert isinstance(actual, list) and len(expected) == len(actual)
        for a, b in zip(expected, actual): compare_authored(a, b)
    else: assert expected == actual
compare_authored(fixture['document'], model['document'])
Draft202012Validator(json.loads(Path('contracts/generated/document.schema.json').read_text())).validate(model['document'])
assert model['semanticDigest'] == digest('musteroffice.presentation/0.1-draft', model['document'])
assert model['revision'] == digest('musteroffice.revision.genesis/1', model['semanticDigest'])
assert bundle['document'] == dict(documentId=model['document']['id'], revision=model['revision'], modelAssetId=by_name['model']['asset']['id'])
assert bundle['pptxAssetId'] == by_name['presentation']['asset']['id']
context = document('delivery-context')
assert context['settings'] == fixture['settings']
assert context['modelAssetId'] == by_name['model']['asset']['id']
assert context['resourceAssets'] == {'resource:checker': by_name['resource:0']['asset']['id']}
assert data('resource:0') == Path('fixtures/presentations/native-export/resources.bin').read_bytes()
assert data('font-bundle') == Path('fixtures/fonts/owned.ttf').read_bytes()
font = document('font-profile')
assert font['manifest'] == fixture['settings']['fonts']
assert font['bundleAssetId'] == context['fontBundleAssetId'] == by_name['font-bundle']['asset']['id']
assert font['bundleSha256'] == sha(data('font-bundle'))
assert int(font['byteLength']) == len(data('font-bundle'))
profile = document('feature-registry')
assert profile['fullPresentationCapability'] is False and profile['targetApplicationValidated'] is False
assert profile['completeNativeCatalogue'] is False and profile['scope'] == 'delivery-composition'
assert [entry['capabilityId'] for entry in profile['entries']] == ['F13.04', 'F14.01']
for feature in profile['entries']:
    assert feature['implementationStatus'] == 'partial' and feature['acceptanceStatus'] == 'not_accepted'
    assert feature['fixtureIds'] == ['presentations/delivery']
    assert Path('contracts/generated/' + feature['parameters']['settingsSchema'] + '.schema.json').exists()
assert profile['entries'][0]['nativeId'] == bundle['profileId']
assert profile['entries'][1]['nativeId'] == context['previewRenderer']['profile']
assert context['settingsDigest'] == digest('musteroffice.delivery-input/1', [fixture['settings'], font['bundleSha256'], context['previewRenderer']])
assert bundle['versions']['featureRegistrySha256'] == sha(data('feature-registry'))
assert bundle['versions']['fontProfileSha256'] == sha(data('font-profile'))
assert context['fontProfileAssetId'] == by_name['font-profile']['asset']['id']
assert context['registryAssetId'] == by_name['feature-registry']['asset']['id']

quality = document('quality')
pptx_sha = sha(data('presentation'))
assert quality['subjectSha256'] == pptx_sha
assert quality['modelSemanticDigest'] == model['semanticDigest']
assert quality['contextAssetId'] == by_name['delivery-context']['asset']['id']
assert quality['pageCoverage'] == model['document']['slideOrder']
assert all(not quality[k] for k in ['layoutQualityProven', 'nativeEditabilityProven', 'playbackProven', 'targetApplicationProven'])
assert len(bundle['claims']) == 5
for c in bundle['claims']:
    assert c['subjectSha256'] == pptx_sha
    assert c['evidenceAssetIds'] == [by_name['quality']['asset']['id']]
    assert c['reason']
    assert c['status'] == ('passed' if c['kind'] == 'structure' else 'not_proven')

pages = []
assert [p['pageId'] for p in bundle['previews']] == model['document']['slideOrder']
for i, p in enumerate(bundle['previews']):
    f = by_id[p['imageAssetId']]
    assert f['name'] == f'preview:{i}' and p['sample'] == {'mode': 'editor'}
    evidence = document(f'preview-evidence:{i}')
    assert quality['previews'][i] == by_name[f'preview-evidence:{i}']['asset']['id']
    assert evidence['pageId'] == p['pageId'] and evidence['pptxSha256'] == pptx_sha
    assert evidence['previewAsset'] == f['asset']
    with Image.open(root/f['file']) as image:
        image.load()
        assert image.size == (p['width'], p['height']) == (640, 360)
        assert image.mode == 'RGBA' and image.info['srgb'] == 0
        pixels = image.tobytes()
    premul = bytes(v if j % 4 == 3 else (v*pixels[j//4*4+3]+127)//255 for j, v in enumerate(pixels))
    raster = evidence['render']['page']['scene']['raster']
    assert raster['sha256'] == sha(premul)
    assert int(raster['byteLength']) == len(premul)
    # Independent content probes, well inside opaque geometry and background.
    at = lambda x, y: tuple(pixels[(y*640+x)*4:(y*640+x+1)*4])
    assert at(620, 340) == (255,255,255,255)
    if i == 0:
        assert at(75, 150) == (27,114,232,255)
        assert at(257, 175) == (38,185,154,255)
        assert at(37, 37) == (22,34,56,255)  # original synthetic A triangle
    pages.append(dict(pageId=p['pageId'], png=entry(root/f['file']), premultipliedSha256=sha(premul), pixels=len(premul)//4))

# Reuse only the frozen independent geometry/appearance checks. The new style
# oracle explicitly checks matching-level semantics; old defPPr checks remain
# unchanged for reproduction of their historical evidence.
program = Path('tools/verification/pptx-independent.py')
spec = importlib.util.spec_from_file_location('native_reference', program)
native = importlib.util.module_from_spec(spec); spec.loader.exec_module(native)
safe = X.XMLParser(resolve_entities=False, no_network=True)
xsd_root = Path('.codex-work/ecma376/xsd')
receipt = json.loads(Path('docs/reviews/evidence/2026-09-24-ecma-schema-inputs.json').read_text())
for f in receipt['files']: assert sha((xsd_root/f['name']).read_bytes()) == f['sha256']
schemas = {native.P:X.XMLSchema(X.parse(str(xsd_root/'pml.xsd'),safe)), native.A:X.XMLSchema(X.parse(str(xsd_root/'dml-main.xsd'),safe))}
checked = []; objects = {}; text_runs = 0
with zipfile.ZipFile(root/by_name['presentation']['file']) as z:
    assert z.testzip() is None
    for name in z.namelist():
        if not name.endswith('.xml'): continue
        xml = X.fromstring(z.read(name), safe)
        ns = X.QName(xml).namespace
        if ns in schemas: schemas[ns].assertValid(xml); checked.append(name)
        assert not xml.xpath('//a:defPPr', namespaces=native.NS)
        for node in xml.xpath('//p:sp|//p:pic|//p:grpSp|//p:cxnSp', namespaces=native.NS):
            identity = node.xpath('./*/p:cNvPr', namespaces=native.NS)[0].get('name')
            assert identity not in objects
            objects[identity] = node
    assert set(objects) == set(fixture['document']['objects']) and len(objects) == 15
    for name, node in objects.items():
        author = fixture['document']['objects'][name]
        native.native_geometry(node, author); native.native_appearance(node, author); native.native_stroke(node, author)
        body = author['content'].get('text')
        if body:
            assert native.rich_text(node.find('p:txBody', native.NS)) == native.author_text(body)
            props = node.find('p:txBody/a:lstStyle/a:lvl1pPr/a:defRPr', native.NS)
            assert int(props.get('sz'))*127 == int(body['style']['size']['value'])
            expected = body['style']['color']['value']['rgba']
            assert props.find('a:solidFill/a:srgbClr', native.NS).get('val') == ''.join(f'{expected[c]:02X}' for c in ['red','green','blue'])
            text_runs += sum(len(p['runs']) for p in body['paragraphs'])
    presentation = X.fromstring(z.read('ppt/presentation.xml'), safe)
    for slot in ['latin','ea','cs']:
        assert presentation.find(f'p:defaultTextStyle/a:lvl1pPr/a:defRPr/a:{slot}',native.NS).get('typeface') == fixture['settings']['defaults']['fontFamily']
assert text_runs == 8
assert len(Presentation(root/by_name['presentation']['file']).slides) == 2
report = dict(format='musteroffice.delivery-reference/1', artifacts=[entry(root/f['file']) for f in files], fixture=entry(fixture_path), bundle=entry(root/'bundle.json'), independentProgram=entry(program), schemasChecked=checked, nativeObjects=15, nativeTextRuns=8, pages=pages, limitation='Actual private calculation, PNG and generated-subset file checks; not public export, full layout, playback, external editing or Musterwork acceptance.')
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(artifacts=len(files), pages=len(pages), xsdParts=len(checked), nativeObjects=15, nativeTextRuns=8)))
