"""Independent integer pixel oracle and owned LibreOffice page observations.

The oracle is input-authored rectangular geometry/colors, never a reread of the
kernel plan. PDF observations are separate; a known difference is not forgiven
as an exact visual match. No Office/WPS acceptance is inferred.
"""
import argparse,hashlib,json,plistlib,shutil,subprocess,tempfile
from pathlib import Path
import fitz
import numpy as np
from PIL import Image
ROOT=Path('.codex-work/source-page')
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=json.loads((ROOT/'manifest.json').read_text());parity=json.loads((ROOT/'parity.json').read_text())
parser=argparse.ArgumentParser();parser.add_argument('--observe',action='store_true');args=parser.parse_args()
if args.observe:
    inputs=[c for c in manifest['cases'] if c['oracle']]
    for c in inputs:assert sha(Path(c['path']).read_bytes())==c['sha256']
    # A separate profile and owned output directory do not attach to a user's
    # open office process or change an existing document.
    with tempfile.TemporaryDirectory(prefix='lo-observe-',dir=ROOT) as directory:
        directory=Path(directory).resolve();output=directory/'pdf';output.mkdir()
        command=['/Applications/LibreOffice.app/Contents/MacOS/soffice','-env:UserInstallation='+(directory/'profile').as_uri(),'--headless','--convert-to','pdf','--outdir',str(output),*[str(Path(c['path']).resolve()) for c in inputs]]
        process=subprocess.run(command,capture_output=True,text=True,timeout=60)
        (ROOT/'libreoffice-convert.log').write_text(process.stdout+process.stderr)
        assert process.returncode==0,process.stderr
        target=ROOT/'libreoffice';target.mkdir(exist_ok=True)
        for c in inputs:
            pdf=output/(c['name']+'.pdf');assert pdf.is_file()
            shutil.copyfile(pdf,target/pdf.name)
rasters={c['name']:c for c in parity['cases'] if c.get('pixelsPath')}
refs=[];observations=[]
ys,xs=np.mgrid[:450,:800];wx=(2*xs+1)*7620;wy=(2*ys+1)*7620
for c in manifest['cases']:
    if not c['oracle']:continue
    assert sha(Path(c['path']).read_bytes())==c['sha256']
    r=rasters[c['name']];raw=Path(r['pixelsPath']).read_bytes();assert sha(raw)==r['pixelsSha256']
    image=np.frombuffer(raw,dtype=np.uint8).reshape(450,800,4)
    expected=np.zeros((450,800,4),dtype=np.uint8);expected[:]=c['oracle'].get('background',[255,255,255])+[255]
    mask=np.ones((450,800),dtype=bool)
    for rect in c['oracle']['rectangles']:
        x0,y0,x1,y1=rect['box'];inside=(wx>=x0)&(wx<x1)&(wy>=y0)&(wy<y1)
        expected[inside]=rect['color']+[255]
        # Two pixels around analytical edges are explicitly not an AA oracle.
        mask &= (abs(wx-x0)>30480)&(abs(wx-x1)>30480)&(abs(wy-y0)>30480)&(abs(wy-y1)>30480)
    mismatch=np.any(image!=expected,axis=2)&mask
    assert not mismatch.any(),(c['name'],int(mismatch.sum()))
    refs.append({'name':c['name'],'pixelsSha256':r['pixelsSha256'],'interiorPixelsChecked':int(mask.sum()),'mismatchedInteriorPixels':0})
    pdf=ROOT/'libreoffice'/(c['name']+'.pdf');doc=fitz.open(pdf)
    pix=doc[0].get_pixmap(matrix=fitz.Matrix(800/doc[0].rect.width,450/doc[0].rect.height),alpha=False)
    observed=np.frombuffer(pix.samples,dtype=np.uint8).reshape(pix.height,pix.width,3)[:450,:800]
    difference=np.any(abs(observed.astype(int)-expected[:,:,:3].astype(int))>1,axis=2)&mask
    # useBgFill is the one explicitly recorded application discrepancy.
    if c['name']=='master-background-context':assert int(difference.sum())>5000
    else:assert not difference.any(),(c['name'],int(difference.sum()))
    observations.append({'name':c['name'],'sourcePath':c['path'],'sourceSha256':c['sha256'],'pdfPath':str(pdf),'pdfSha256':sha(pdf.read_bytes()),'interiorPixelsCompared':int(mask.sum()),'differentInteriorPixels':int(difference.sum()),'matchesOracleAtTolerance1':not bool(difference.any())})
for name in ['layers-None-None','master-color-context','group-ellipse','preset-heart','native-round-stroke']:
    r=rasters[name];image=Image.frombytes('RGBA',(800,450),Path(r['pixelsPath']).read_bytes());image.save(ROOT/(name+'.png'))
info=plistlib.loads(Path('/Applications/LibreOffice.app/Contents/Info.plist').read_bytes())
result={'format':'musteroffice.source-page-reference/1','oracleScope':'Owned axis-aligned opaque rectangles only, exact integer pixel-center membership away from edges. Does not certify antialiasing, curves, general color math or Office/WPS fidelity.','cases':refs,'pixelsChecked':sum(c['interiorPixelsChecked'] for c in refs),'libreOffice':{'version':info['CFBundleShortVersionString'],'build':info['CFBundleVersion'],'cases':observations,'differences':['The master useBgFill probe retains its direct red fill in LibreOffice; the kernel samples the final page background per the declared native background-fill rule. Office/WPS behavior remains unverified.']},'notObserved':['Microsoft Office','WPS - cua-driver process lacks Accessibility and Screen Recording grants','editing/save/reopen']}
(ROOT/'reference.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'oracleCases':len(refs),'interiorPixels':result['pixelsChecked'],'libreOfficeCases':len(observations),'matchingObservations':sum(c['matchesOracleAtTolerance1'] for c in observations)}))
