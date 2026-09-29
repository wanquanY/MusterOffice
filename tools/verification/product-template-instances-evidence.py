"""Inspect actual product template-instance files without the kernel parser.

Consumes the owned template_engine conformance outputs. This is byte/XML
evidence for scalar boundary cases, not a layout or application verdict.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import xml.etree.ElementTree as ET
import zipfile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(prefix, name):
    root = Path(str(prefix) + '-' + name)
    result = json.loads((root / 'result.json').read_text())
    assert result['status'] == 'completed'
    files = {}
    for role, descriptor in result['files'].items():
        assert Path(role).name == role and role not in ('.', '..')
        data = (root / role).read_bytes()
        assert sha(data) == descriptor['sha256']
        assert len(data) == descriptor['byte_length']
        files[role] = data
    for entry in result.get('delivery_assets', {}).values():
        data = files[entry['role']]
        asset = entry['asset']
        assert sha(data) == asset['sha256']
        assert len(data) == int(asset['byteLength'])
        assert result['files'][entry['role']]['media_type'] == asset['mediaType']
    if 'receipt' in result:
        assets = result['receipt']['bundle']['assets']
        assert len(assets) == len(result['delivery_assets'])
        for declared in assets:
            assert result['delivery_assets'][declared['id']]['asset'] == declared
    return result, files


def asset(result, files, asset_id):
    return files[result['delivery_assets'][asset_id]['role']]


def compressed(data, item):
    offset = item.header_offset
    assert data[offset:offset + 4] == b'PK\x03\x04'
    name_length, extra_length = struct.unpack_from('<HH', data, offset + 26)
    start = offset + 30 + name_length + extra_length
    return data[start:start + item.compress_size]


def target_text(tree, binding, run):
    ns = {'p': 'http://schemas.openxmlformats.org/presentationml/2006/main',
          'a': 'http://schemas.openxmlformats.org/drawingml/2006/main'}
    shapes = [s for s in tree.findall('.//p:sp', ns)
              if s.find('p:nvSpPr/p:cNvPr', ns).get('id') == str(binding['nativeId'])]
    assert len(shapes) == 1
    paragraph = shapes[0].findall('p:txBody/a:p', ns)[run['paragraph']]
    runs = [child for child in paragraph
            if child.tag in ('{' + ns['a'] + '}r', '{' + ns['a'] + '}fld')]
    return runs[run['run']].find('a:t', ns)


def verify(prefix):
    source, source_files = load(prefix, 'source')
    defined, defined_files = load(prefix, 'defined')
    package = json.loads(defined_files['template_document'])
    assert package['snapshot'] == json.loads(source_files['snapshot'])
    definition = package['definition']
    target = definition['parameters']['headline']['target']
    assert target['kind'] == 'textRun'
    binding = package['snapshot']['document']['sourceBindings']['objects'][target['object']]
    run = binding['runs'][target['run']]
    part = binding['part'].lstrip('/')
    original = asset(source, source_files, source['receipt']['bundle']['pptxAssetId'])
    results = []
    for name, length in [('minimum', target['minScalars']), ('maximum', target['maxScalars'])]:
        result, files = load(prefix, 'instance-' + name)
        receipt = result['receipt']
        instantiated = result['instantiation']
        assert result['template_digest'] == defined['description']['templateDigest']
        assert result['template_ref']['sha256'] == sha(defined_files['template_document'])
        assert instantiated['template']['boundParameters'] == ['headline']
        for key in ('documentId', 'revision', 'semanticDigest'):
            assert receipt[key] == instantiated[key]
        original_ref = package['resources']['template:source']
        rebound = result['template_resources']['template:source']
        assert original_ref['content_id'] != rebound['content_id']
        for key in ('sha256', 'byte_length', 'media_type'):
            assert original_ref[key] == rebound[key]
        pptx = asset(result, files, receipt['bundle']['pptxAssetId'])
        retained = [asset(result, files, a['id']) for a in receipt['bundle']['assets']
                    if a['role'] == 'source']
        assert retained == [original]
        with zipfile.ZipFile(io.BytesIO(original)) as before, zipfile.ZipFile(io.BytesIO(pptx)) as after:
            assert before.namelist() == after.namelist()
            changed = [p for p in before.namelist() if before.read(p) != after.read(p)]
            assert changed == [part]
            for item in before.infolist():
                if item.filename != part:
                    assert compressed(original, item) == compressed(pptx, after.getinfo(item.filename))
            a, b = ET.fromstring(before.read(part)), ET.fromstring(after.read(part))
            text = target_text(b, binding, run)
            assert text.text == 'A' * length
            text.text = target_text(a, binding, run).text
            assert ET.tostring(a) == ET.tostring(b), 'only the selected text leaf changes'
            parts = len(before.namelist())
        previews = receipt['bundle']['previews']
        source_previews = source['receipt']['bundle']['previews']
        assert len(previews) == len(source_previews) == 2
        assert [p['pageId'] for p in previews] == [p['pageId'] for p in source_previews]
        preview_results = []
        for index, (preview, base) in enumerate(zip(previews, source_previews)):
            pixels = asset(result, files, preview['imageAssetId'])
            assert pixels[:8] == b'\x89PNG\r\n\x1a\n'
            assert struct.unpack('>II', pixels[16:24]) == (640, 360)
            unchanged = pixels == asset(source, source_files, base['imageAssetId'])
            assert unchanged == (index == 1)
            preview_results.append(dict(sha256=sha(pixels), byteLength=len(pixels),
                                        unchangedFromSource=unchanged))
        claims = {c['kind']: c['status'] for c in receipt['bundle']['claims']}
        assert claims == {'structure': 'passed', 'layout': 'not_proven',
                          'native-editability': 'not_proven', 'playback': 'not_proven',
                          'target-application': 'not_proven'}
        results.append(dict(case=name, scalars=length, sha256=sha(pptx), byteLength=len(pptx),
                            parts=parts, changedParts=changed, preservedCompressedParts=parts - 1,
                            previews=preview_results, actualAssetCount=len(result['delivery_assets']),
                            claims=claims))
    return dict(scope='Owned scalar-boundary files, not multilingual or layout acceptance',
                sourceSha256=sha(original), sourceBytes=len(original), instances=results,
                packageUnchanged=True, explicitSourceIdentityRebound=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.prefix)
    with args.report.open('x') as output:
        json.dump(result, output, indent=2)
        output.write('\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
