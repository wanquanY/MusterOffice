"""Observe the four owned transform PPTX probes after external PDF conversion.

Run with development Python providing PyMuPDF and Pillow. This does not control
Office/WPS, edit source geometry, or promote observed differences to acceptance.
"""
import hashlib
import json
import math
import subprocess
from pathlib import Path
import fitz
from PIL import Image

root=Path('.codex-work/page-placement')
def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def distance(p,a,b):
    dx=b[0]-a[0];dy=b[1]-a[1];norm=dx*dx+dy*dy
    t=max(0,min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/norm)) if norm else 0
    return math.hypot(p[0]-a[0]-t*dx,p[1]-a[1]-t*dy)
results=[]
for c in read(root/'render.json')['cases']:
    pdf=root/'libreoffice'/(c['name']+'.pdf');source=Path(c['pptxPath']);assert entry(source)['sha256']==c['pptxSha256']
    q=read(root/(c['name']+'.request.json'));result=read(root/(c['name']+'.response.json'))['result'];p=result['surfaces'][0]['objects'][1]
    scene=read(c['scenePath']);matrix=[int(v)/(1<<32) for v in p['affine']['linear']];center=[int(p['affine']['translation'][a])/(1<<32) for a in ['x','y']]
    points=[[int(v['to'][a])/(1<<32) for a in ['x','y']] for v in scene['scene']['paths'][0]['commands'] if 'to' in v]
    expected=[[(matrix[0]*x+matrix[1]*y+center[0])/12700,(matrix[2]*x+matrix[3]*y+center[1])/12700] for x,y in points]
    with fitz.open(pdf) as doc:
        assert len(doc)==1
        colored=[s for s in doc[0].get_drawings() if s['fill'] and max(abs(a-b/255) for a,b in zip(s['fill'],[180,40,70]))<1e-5]
        assert len(colored)==1
        segments=[]
        for item in colored[0]['items']:
            assert item[0]=='l';segments.append([list(item[1]),list(item[2])])
        observed=[p for segment in segments for p in segment]
        expected_edges=list(zip(expected,expected[1:]+expected[:1]))
        max_distance=max([min(distance(p,*edge) for edge in segments) for p in expected]+[min(distance(p,*edge) for edge in expected_edges) for p in observed])
        actual_center=[(min(p[k] for p in observed)+max(p[k] for p in observed))/2 for k in range(2)]
        center_distance=math.hypot(*(actual_center[k]-center[k]/12700 for k in range(2)))
        page_size=list(doc[0].rect)
        preview=pdf.with_suffix('.png');doc[0].get_pixmap(matrix=fitz.Matrix(4/3,4/3),alpha=False).save(preview)
    pixels=Path(c['pixelsPath']).read_bytes();assert hashlib.sha256(pixels).hexdigest()==c['pixelsSha256']
    kernel_preview=root/(c['name']+'.png');Image.frombytes('RGBA',(800,600),pixels).save(kernel_preview)
    results.append({'name':c['name'],'source':entry(source),'pdf':entry(pdf),'pdfPageRectPt':page_size,
                    'expectedVerticesPt':expected,'observedSegmentsPt':segments,'maximumVertexToBoundaryDistancePt':max_distance,
                    'centerDistancePt':center_distance,'observationTolerancePt':0.1,'withinObservationTolerance':max_distance<=0.1,
                    'kernelPreview':entry(kernel_preview),'externalPreview':entry(preview)})
report={'format':'musteroffice.page-placement-external/1','application':subprocess.check_output(['soffice','--version'],text=True).strip(),
        'pdfExtractor':'PyMuPDF '+fitz.VersionBind,'scope':'Owned rectangle/group probes; differences retained. Not Office/WPS visual or round-trip acceptance. Boundary distances are vertex-to-segment observations, not an all-pixel metric.',
        'cases':results,'allWithinObservationTolerance':all(r['withinObservationTolerance'] for r in results)}
(root/'external.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({r['name']:r['maximumVertexToBoundaryDistancePt'] for r in results}))
