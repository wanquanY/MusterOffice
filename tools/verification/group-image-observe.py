"""Scoped application observation of original group-fill source pairs.

Requires PyMuPDF and an explicitly selected LibreOffice executable. Runs only
owned fixtures with a temporary profile, never a user's open presentation.
"""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path
import fitz
p=argparse.ArgumentParser();p.add_argument('--soffice',type=Path,required=True);a=p.parse_args()
root=Path('.codex-work/group-image');out=root/'libreoffice';out.mkdir(exist_ok=True)
def entry(p):
 b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
fixtures=json.loads((root/'fixtures.json').read_text());cases=[c for c in fixtures['cases'] if c['status']=='rendered']
version=subprocess.check_output([str(a.soffice),'--version'],text=True,timeout=30).strip()
with tempfile.TemporaryDirectory(prefix='mo-owned-group-image-') as profile:
 r=subprocess.run([str(a.soffice),'-env:UserInstallation='+Path(profile).as_uri(),'--headless','--convert-to','pdf:impress_pdf_Export','--outdir',str(out),*[c['source']['path'] for c in cases]],capture_output=True,text=True,timeout=60)
 (root/'libreoffice-convert.log').write_text(r.stdout+r.stderr);assert r.returncode==0,r.stderr
records=[]
for c in cases:
 assert entry(c['source']['path'])==c['source']
 pdf=out/(c['name']+'.pdf')
 with fitz.open(pdf) as doc:
  assert len(doc)==2,(c['name'],len(doc))  # Original fixture retains its second page.
  page=doc[0];pix=page.get_pixmap(matrix=fitz.Matrix(400/page.rect.width,300/page.rect.height),alpha=False)
  assert (pix.width,pix.height,pix.n)==(400,300,3)
  pixels=out/(c['name']+'.rgb');pixels.write_bytes(pix.samples)
  records.append(dict(name=c['name'],source=c['source'],pdf=entry(pdf),pixels=entry(pixels),pageCount=len(doc),observedPage=0,pagePoints=[page.rect.width,page.rect.height]))
pairs=[]
for c in records:
 if c['name'].endswith('-explicit'):continue
 control=next(v for v in records if v['name']==c['name']+'-explicit')
 x=Path(c['pixels']['path']).read_bytes();y=Path(control['pixels']['path']).read_bytes()
 differences=sum(x[i:i+3]!=y[i:i+3] for i in range(0,len(x),3))
 pairs.append(dict(name=c['name'],pixelsDifferent=differences))
report=dict(format='musteroffice.group-image-application-observation/1',application=dict(version=version,launcher=entry(a.soffice)),scope='LibreOffice import and PDF conversion of original paired group-image sources; first-page samples at 400x300. Records differences without treating LibreOffice as Office/WPS acceptance.',cases=records,pairs=pairs)
(root/'observations.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(cases=len(records),pairs=pairs)))
