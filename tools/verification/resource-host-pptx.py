"""Independently inspect the actual resource-backed native package."""
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile
import xml.etree.ElementTree as ET

root=Path('.codex-work/resource-host/pptx')
report=json.loads((root/'result.json').read_text())
data=(root/'resource-backed.pptx').read_bytes()
assert hashlib.sha256(data).hexdigest()==report['pptxSha256'] and len(data)==report['pptxBytes']
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-operation-host-verification.json').read_text())
assert hashlib.sha256(Path('target/debug/mo-cli').read_bytes()).hexdigest()==parent['currentArtifacts']['nativeCli']['sha256']
baseline=root/'inline-baseline.pptx'
assert not baseline.exists(), 'refuse to overwrite a prior comparison'
run=subprocess.run(['target/debug/mo-cli','pptx-export','fixtures/presentations/native-export/request.json','fixtures/presentations/native-export/resources.bin',str(baseline)],capture_output=True,check=True)
assert not run.stderr and baseline.read_bytes()==data
(root/'inline-baseline-receipt.json').write_bytes(run.stdout)
fixture=json.loads(Path('fixtures/presentations/native-export/request.json').read_text())
bundle=Path('fixtures/presentations/native-export/resources.bin').read_bytes()
declared={b['resourceId']:bundle[int(b['byteOffset']):int(b['byteOffset'])+int(b['byteLength'])] for b in fixture['resourceBindings']}
source_hashes={hashlib.sha256(b).hexdigest() for b in declared.values()}
assert {a['descriptor']['sha256'] for a in report['assets']}==source_hashes
with zipfile.ZipFile(root/'resource-backed.pptx') as archive:
    assert archive.testzip() is None
    images={name:hashlib.sha256(archive.read(name)).hexdigest() for name in archive.namelist() if name.startswith('ppt/media/')}
    assert set(images.values())==source_hashes
    xml_parts=0
    for name in archive.namelist():
        if name.endswith(('.xml','.rels')):
            ET.fromstring(archive.read(name));xml_parts+=1
    slides=sorted(name for name in archive.namelist() if name.startswith('ppt/slides/slide') and name.endswith('.xml'))
    assert len(slides)==len(fixture['document']['slideOrder'])
    surfaces=[name for name in archive.namelist() if name.endswith('.xml') and name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/'))]
    objects=sum(len(ET.fromstring(archive.read(name)).findall('.//{http://schemas.openxmlformats.org/presentationml/2006/main}cNvPr'))-1 for name in surfaces)
    slide_objects=sum(len(ET.fromstring(archive.read(name)).findall('.//{http://schemas.openxmlformats.org/presentationml/2006/main}cNvPr'))-1 for name in slides)
    assert objects==len(fixture['document']['objects']) and slide_objects>0
def entry(path):
    p=Path(path);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
evidence=dict(format='musteroffice.resource-host-pptx-reference/1',pptx=entry(root/'resource-backed.pptx'),inlineBaseline=entry(baseline),baselineByteIdentical=True,producer=entry(root/'result.json'),sourceRequest=entry('fixtures/presentations/native-export/request.json'),sourceResources=entry('fixtures/presentations/native-export/resources.bin'),slides=len(slides),xmlParts=xml_parts,nativeObjects=objects,slideObjects=slide_objects,definitionObjects=objects-slide_objects,mediaHashes=images,mediaBytesMatch=True,zipCrc=True,xmlWellFormed=True,limitations=['Independent ZIP/media/XML checks, not complete XSD, visual fidelity, target editor or public export-job acceptance.'])
Path('.codex-work/resource-host/pptx-reference.json').write_text(json.dumps(evidence,indent=2)+'\n')
print(json.dumps(dict(slides=len(slides),nativeObjects=objects,xmlParts=xml_parts,mediaBytesMatch=True)))
