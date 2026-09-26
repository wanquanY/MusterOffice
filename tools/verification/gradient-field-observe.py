"""Owned source observation in LibreOffice, without Office/WPS acceptance claims."""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path
import fitz
p=argparse.ArgumentParser();p.add_argument('--soffice',type=Path,required=True);a=p.parse_args()
root=Path('.codex-work/gradient-field');out=root/'libreoffice';out.mkdir(exist_ok=True)
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
sources=[p for p in sorted((root/'native').glob('*.pptx')) if not p.stem.endswith('-control')];assert len(sources)==16
version=subprocess.check_output([str(a.soffice),'--version'],text=True,timeout=30).strip()
with tempfile.TemporaryDirectory(prefix='mo-owned-gradient-') as profile:
    r=subprocess.run([str(a.soffice),'-env:UserInstallation='+Path(profile).as_uri(),'--headless','--convert-to','pdf:impress_pdf_Export','--outdir',str(out),*[str(p) for p in sources]],capture_output=True,text=True,timeout=60)
    (root/'libreoffice-convert.log').write_text(r.stdout+r.stderr);assert r.returncode==0,r.stderr
records=[]
for source in sources:
    pdf=out/(source.stem+'.pdf')
    with fitz.open(pdf) as doc:
        assert len(doc)==2
        page=doc[0];pix=page.get_pixmap(matrix=fitz.Matrix(400/page.rect.width,300/page.rect.height),alpha=False)
        assert (pix.width,pix.height,pix.n)==(400,300,3)
        observed=pix.samples
        pixels=out/(source.stem+'.rgb');pixels.write_bytes(observed)
        raw=source.with_suffix('.rgba').read_bytes();flattened=bytes(raw[i+k]+255-raw[i+3] for i in range(0,len(raw),4) for k in range(3))
        different=0;maximum=0;interior=0;interior_different=0
        for i in range(0,len(flattened),3):
            delta=max(abs(flattened[i+k]-observed[i+k]) for k in range(3))
            different+=delta!=0;maximum=max(maximum,delta)
            x=(i//3)%400;y=(i//3)//400
            if 1<=x<399 and 1<=y<299 and all(
                flattened[(yy*400+xx)*3:(yy*400+xx+1)*3]==flattened[i:i+3]
                and observed[(yy*400+xx)*3:(yy*400+xx+1)*3]==observed[i:i+3]
                for yy in range(y-1,y+2) for xx in range(x-1,x+2)):
                interior+=1;interior_different+=delta!=0
        sample_points=[(50,75),(225,75),(50,175),(225,175),(10,10)]
        samples=[dict(x=x,y=y,kernelOverWhite=list(flattened[(y*400+x)*3:(y*400+x+1)*3]),application=list(observed[(y*400+x)*3:(y*400+x+1)*3])) for x,y in sample_points]
        records.append(dict(name=source.stem,source=entry(source),pdf=entry(pdf),pixels=entry(pixels),kernelPixels=entry(source.with_suffix('.rgba')),pixelsDifferent=different,maximumChannelDifference=maximum,constantNeighbourhoodPixels=interior,constantNeighbourhoodDifferences=interior_different,samples=samples,pageCount=len(doc),observedPage=0))
report=dict(format='musteroffice.gradient-field-observation/1',application=dict(version=version,launcher=entry(a.soffice)),scope='First-page PDF samples at 400x300. Kernel premultiplied output composited over white for comparison. Includes AA and texel edges; records all differences without waiving them or claiming Office/WPS acceptance.',cases=records)
(root/'observations.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(application=version,cases=[{k:c[k] for k in ['name','pixelsDifferent','maximumChannelDifference','constantNeighbourhoodDifferences']} for c in records])))
