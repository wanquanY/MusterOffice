"""Bind WPS/LibreOffice observations and independent source-edit preservation."""
import hashlib
import importlib.util
import json
from pathlib import Path
from PIL import Image
import numpy as np

ROOT=Path('.codex-work/angle-export')
def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes()
    return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}

def load(path):
    spec=importlib.util.spec_from_file_location('source_reference',path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

reference=load('tools/verification/pptx-source-independent.py')
preserved=[]
for c in read(ROOT/'source-parity.json')['cases']:
    preserved.append(reference.verify({'name':c['name'],'source':c['sourcePath'],'output':c['pptxPath'],
                                       'request':c['requestPath'],'sha256':c['pptxSha256'],'noop':False}))
old=read('.codex-work/group-compat/observations.json');new=read(ROOT/'observations.json');comparisons=[]
# The canvas interior was explicitly inspected in both window captures. This
# exact rectangle is specific to those captures; it contains no external tabs.
roi=[277,250,1155,907]
for a,b in zip(old['cases'],new['cases'],strict=True):
    assert a['name']==b['name'] and a['registration']==b['registration']
    w=read(ROOT/('wps-'+b['name']+'.windows.json'))
    assert any(x['title']=='canonical-'+b['name']+'.pptx' and x['window_id']==249 for x in w['windows'])
    images=[];digests=[]
    for case in [a,b]:
        p=case['privateCapture'];assert entry(p['path'])==p
        image=np.array(Image.open(p['path']).convert('RGB'))[roi[1]:roi[3],roi[0]:roi[2]]
        images.append(image);digests.append(hashlib.sha256(image.tobytes()).hexdigest())
    changed=int(np.count_nonzero(np.any(images[0]!=images[1],axis=2)))
    assert changed==0 and digests[0]==digests[1]
    distances=[c['libreOfficeMaxVertexBoundaryDistancePt'] for c in b['cases']]
    assert len(distances)==16 and max(distances)<.2
    comparisons.append({'name':b['name'],'rawInput':a['source'],'canonicalInput':b['source'],
                        'rawPrivateCapture':a['privateCapture'],'canonicalPrivateCapture':b['privateCapture'],
                        'canvasRectCapturePx':roi,'canvasPixelsCompared':images[0].shape[0]*images[0].shape[1],
                        'canvasRgbSha256':digests[0],'pixelsDifferent':changed,
                        'beforeLibreOfficeMaxPt':max(c['libreOfficeMaxVertexBoundaryDistancePt'] for c in a['cases']),
                        'afterLibreOfficeMaxPt':max(distances)})
report={'format':'musteroffice.angle-export-observations/1',
        'scope':'Three identical WPS canvas captures for raw vs canonical static-angle packages, 48 actual LibreOffice vector observations, and independent ZIP/XML preservation of four text-edit candidates. No complete pixel fidelity, Office, playback or external editing acceptance.',
        'external':new,'wpsCanvasComparisons':comparisons,'sourcePreservation':preserved}
(ROOT/'compatibility.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'canvasPixelsCompared':sum(c['canvasPixelsCompared'] for c in comparisons),'pixelsDifferent':0,
                  'wpsGeometricProbes':48,'libreOfficeGeometricProbes':48,'preservedSourceEditBatches':len(preserved),
                  'unchangedCompressedEntries':sum(c['unchangedCompressedEntriesVerified'] for c in preserved)}))
