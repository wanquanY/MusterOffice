"""Independent ZIP/XML, relationship origin and exact encoded-payload checks.

Does not use the Rust resource resolver to derive expected owner/target or bytes.
This is resource binding evidence, not codec or rendered-pixel acceptance.
"""
import hashlib
import json
from pathlib import Path
import posixpath
import sys
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET
import zipfile
from jsonschema import Draft202012Validator

root = Path(sys.argv[1] if len(sys.argv) > 1 else '.codex-work/source-images')
report = json.loads((root / 'parity.json').read_text())
R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
P = '/ppt/'
owners = [P+'slides/slide1.xml', P+'slideLayouts/slideLayout2.xml', P+'slideMasters/slideMaster2.xml', P+'theme/theme2.xml']
schemas = {n: Draft202012Validator(json.loads(Path('contracts/generated/'+n+'.schema.json').read_text())) for n in ['pptx-image-query', 'pptx-image-response']}
cases, payloads, references = [], 0, 0
for c in report['cases']:
    q = json.loads(Path(c['request']['path']).read_text())
    response = json.loads(Path(c['response']['path']).read_text())
    schemas['pptx-image-query'].validate(q)
    schemas['pptx-image-response'].validate(response)
    r = response['images']
    source = Path(c['source']['path']).read_bytes()
    assert hashlib.sha256(source).hexdigest() == r['sourceSha256'] == q['fill']['expectedSourceSha256']
    expected, first_use = bytearray(), []
    with zipfile.ZipFile(c['source']['path']) as z:
        assert z.testzip() is None
        types = {e.attrib['PartName']: e.attrib['ContentType'] for e in ET.fromstring(z.read('[Content_Types].xml')) if e.tag.endswith('}Override')}
        for target in r['targets']:
            o = target['outcome']
            if o['status'] not in ['available', 'externalRequired']:
                assert o['status'] in ['unresolvedReference', 'unresolvedFill', 'notImage']
                continue
            ref = o['binding']['reference']; owner = ref['ownerPart']; slot = 'embed' if q['selection'] == 'embeddedSnapshot' else 'link'
            if c['name'].startswith('owner-'):
                assert owner == owners[int(c['name'][-1])]
            elif c['name'].startswith('inherited-'):
                assert owner == owners[3 if slot == 'embed' else 1]
            elif c['name'] == 'group-background':
                assert owner == owners[2 if target['target']['kind'] == 'background' else 0]
            else:
                assert owner == owners[0]
            xml = ET.fromstring(z.read(owner[1:]))
            assert any(e.attrib.get('{'+R+'}'+slot) == ref['relationshipId'] for e in xml.iter('{'+A+'}blip'))
            relpath = posixpath.join(posixpath.dirname(owner), '_rels', posixpath.basename(owner)+'.rels')
            relations = {e.attrib['Id']:e.attrib for e in ET.fromstring(z.read(relpath[1:]))}
            edge = relations[ref['relationshipId']]
            assert edge['Type'] == R+'/image' and edge['Target'] == ref['targetUri']
            if slot == 'link':
                assert edge['TargetMode'] == 'External' and o['status'] == 'externalRequired'
            else:
                assert edge.get('TargetMode', 'Internal') == 'Internal'
                uri = urlsplit(edge['Target']); assert not uri.fragment and not uri.scheme
                part = posixpath.normpath(posixpath.join(posixpath.dirname(owner), unquote(uri.path)))
                resource = r['resources'][o['resource']]; assert resource['part'] == part
                if part not in first_use: first_use.append(part)
            references += 1
        assert [v['part'] for v in r['resources']] == first_use
        for resource in r['resources']:
            part = resource['part']; data = z.read(part[1:])
            assert types[part] == resource['contentType'] and types[part].startswith('image/')
            assert int(resource['offset']) == len(expected)
            assert int(resource['byteLength']) == len(data)
            assert hashlib.sha256(data).hexdigest() == resource['sha256']
            expected.extend(data); payloads += 1
    assert len(expected) == int(r['bundleByteLength'])
    assert expected == Path(c['bytes']['path']).read_bytes() == Path(c['expectedBytes']['path']).read_bytes()
    cases.append(dict(name=c['name'], resources=len(r['resources']), bytes=len(expected)))
for c in report['failures']:
    schemas['pptx-image-response'].validate(json.loads(Path(c['response']['path']).read_text()))
    # JSON Schema cannot detect duplicate object members after decoding.
    if c['name'] != 'duplicate-member':
        assert schemas['pptx-image-query'].is_valid(json.loads(Path(c['request']['path']).read_text())) == c['validSchema']
counts = dict(cases=len(cases), payloads=payloads, references=references, requestFailures=len(report['failures']))
(root / 'reference.json').write_text(json.dumps(dict(format='musteroffice.source-image-reference/1',counts=counts,cases=cases),indent=2)+'\n')
print(json.dumps(counts))
