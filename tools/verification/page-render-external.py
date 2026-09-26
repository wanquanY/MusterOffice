"""Observe evaluated page geometry against LibreOffice's actual PDF paths.

This is a sampled boundary observation, not a complete visual acceptance test.
Run with the development PyMuPDF/NumPy runtime. No font or private media input.
"""
import argparse,hashlib,json,math,plistlib,subprocess,tempfile
from pathlib import Path
import fitz
import numpy as np
from PIL import Image
root=Path('.codex-work/page-render/external');U=1<<32
parser=argparse.ArgumentParser()
parser.add_argument('--soffice',type=Path,help='Explicit executable; convert owned inputs using an isolated temporary profile')
args=parser.parse_args()
if args.soffice:
    inputs=json.loads((root/'inputs.json').read_text())
    for case in inputs['cases']:
        assert hashlib.sha256(Path(case['pptxPath']).read_bytes()).hexdigest()==case['pptxSha256']
        (root/'libreoffice'/(case['name']+'.pdf')).unlink(missing_ok=True)
    (root/'libreoffice').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='mo-page-interop-') as profile:
        result=subprocess.run([str(args.soffice),'-env:UserInstallation='+Path(profile).as_uri(),'--headless','--convert-to','pdf:impress_pdf_Export','--outdir',str(root/'libreoffice'),*[c['pptxPath'] for c in inputs['cases']]],capture_output=True,text=True,timeout=60)
        (root/'conversion.log').write_text(result.stdout+result.stderr)
        assert result.returncode==0,result.stderr
    (root/'converted-inputs.json').write_text(json.dumps({c['name']:c['pptxSha256'] for c in inputs['cases']},indent=2)+'\n')


def entry(p):
    p=Path(p);b=p.read_bytes();return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def sample(commands):
    result=[];previous=None;start=None;bound=0
    for kind,points in commands:
        p=[np.array(v,dtype=float) for v in points]
        if kind=='move':start=previous=p[0];result.append(previous)
        elif kind in ['line','close']:
            end=start if kind=='close' else p[0]
            n=max(1,math.ceil(float(np.linalg.norm(end-previous))))
            result.extend(previous+(end-previous)*t for t in np.linspace(0,1,n+1)[1:]);previous=end
        else:
            n=128;t=np.linspace(0,1,n+1)[1:,None];u=1-t
            if kind=='quadratic':
                result.extend(u*u*previous+2*u*t*p[0]+t*t*p[1]);second=2*np.linalg.norm(previous-2*p[0]+p[1])
            else:
                result.extend(u**3*previous+3*u*u*t*p[0]+3*u*t*t*p[1]+t**3*p[2]);second=6*max(np.linalg.norm(previous-2*p[0]+p[1]),np.linalg.norm(p[0]-2*p[1]+p[2]))
            bound=max(bound,float(second)/(8*n*n));previous=p[-1]
    return np.array(result),bound

def directed(a,b):
    start=b[:-1];d=b[1:]-start;norm=np.sum(d*d,axis=1);norm=np.where(norm==0,1,norm);largest=0
    for i in range(0,len(a),128):
        p=a[i:i+128,None,:];v=p-start;t=np.clip(np.sum(v*d,axis=2)/norm,0,1);distance=np.linalg.norm(v-t[:,:,None]*d,axis=2)
        largest=max(largest,float(np.max(np.min(distance,axis=1))))
    return largest

records=[]
for c in json.loads((root/'inputs.json').read_text())['cases']:
    assert json.loads((root/'converted-inputs.json').read_text())[c['name']]==c['pptxSha256']
    q=json.loads(Path(c['requestPath']).read_text());plan=json.loads(Path(c['planPath']).read_text())['plan'];scene=plan['raster']['scene'];source=next(p for p in plan['paintSources'] if p['object']=='shape:1');instance=scene['instances'][source['instance']]
    affine=scene['transforms'][instance['transform']]['affine'];a=np.array([int(v)/U for v in affine['linear']]).reshape(2,2);translation=np.array([int(affine['translation'][k])/U for k in ['x','y']]);scale=q['viewport']['scale']['numerator']/q['viewport']['scale']['denominator']
    commands=[]
    for command in scene['paths'][instance['path']]['commands']:
        points=[]
        for k in ['control','control1','control2','to']:
            if k in command:points.append((a@np.array([int(command[k][axis])/U for axis in ['x','y']])+translation)*scale)
        commands.append((command['kind'],points))
    expected,expected_flatten=sample(commands)
    pdf_path=root/'libreoffice'/(c['name']+'.pdf')
    with fitz.open(pdf_path) as pdf:
        assert len(pdf)==1 and abs(pdf[0].rect.width-600)<.02 and abs(pdf[0].rect.height-450)<.02
        drawings=[d for d in pdf[0].get_drawings() if d['fill'] and max(abs(d['fill'][i]-[180,40,70][i]/255) for i in range(3))<1e-5]
        assert len(drawings)==1
        path=drawings[0];commands=[];previous=None
        for item in path['items']:
            kind=item[0]
            if kind=='re':points=[item[1].tl,item[1].tr,item[1].br,item[1].bl,item[1].tl];segments=[('l',x,y) for x,y in zip(points,points[1:])]
            else:segments=[item]
            for segment in segments:
                kind=segment[0];assert kind in ['c','l'],kind
                points=[np.array([v.x,v.y])*4/3 for v in segment[1:]]
                if previous is None:commands.append(('move',[points[0]]))
                else:assert np.linalg.norm(previous-points[0])<.01
                commands.append(('cubic' if kind=='c' else 'line',points[1:]));previous=points[-1]
        commands.append(('close',[]));actual,actual_flatten=sample(commands)
        pdf[0].get_pixmap(matrix=fitz.Matrix(4/3,4/3)).save(root/(c['name']+'-libreoffice.png'))
    error=max(directed(expected,actual),directed(actual,expected))
    raw=Path(c['pixelsPath']).read_bytes();assert hashlib.sha256(raw).hexdigest()==c['pixelsSha256'];Image.frombytes('RGBA',(800,600),raw).save(root/(c['name']+'-kernel.png'))
    records.append({'name':c['name'],'pptx':entry(c['pptxPath']),'pdf':entry(pdf_path),'plan':entry(c['planPath']),'pixels':entry(c['pixelsPath']),'kernelBoundarySamples':len(expected),'libreOfficeBoundarySamples':len(actual),'kernelCurveFlattenDeviationBoundPx':expected_flatten,'libreOfficeCurveFlattenDeviationBoundPx':actual_flatten,'maximumSampledBoundaryDistancePx96':error,'withinObservationTolerance':error<=.27})
info=plistlib.loads(Path('/Applications/LibreOffice.app/Contents/Info.plist').read_bytes())
report={'format':'musteroffice.page-render-external-observations/1','application':{'name':'LibreOffice','version':info.get('CFBundleShortVersionString'),'build':info.get('CFBundleVersion')},'observationTolerancePx96':.27,'cases':records,'allWithinObservationTolerance':all(c['withinObservationTolerance'] for c in records),'scope':'Same owned author document to evaluated Draw IR / actual CPU raster and native editable PPTX. Compare sampled Draw IR and PDF boundaries; not pixel equality, full fidelity, edit round-trip, WPS or PowerPoint acceptance.'}
(root/'observations.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'pages':len(records),'withinTolerance':sum(c['withinObservationTolerance'] for c in records),'maximumObservedPx96':max(c['maximumSampledBoundaryDistancePx96'] for c in records)}))
