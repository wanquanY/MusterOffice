"""Observe an explicitly provided LibreOffice installation on owned line probes.

PDF path properties are limited observations, not Office/WPS acceptance or a
complete visual comparison. The app is a verification tool, not a dependency.
"""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path
import fitz
p=argparse.ArgumentParser();p.add_argument('--soffice',type=Path,required=True);args=p.parse_args()
root=Path('.codex-work/line-style');manifest=json.loads((root/'manifest.json').read_text());out=root/'libreoffice';out.mkdir(exist_ok=True)
cases=[c for c in manifest['cases'] if not c['name'].startswith(('unresolved','retained'))]
sha=lambda b:hashlib.sha256(b).hexdigest()
for c in cases:
 assert sha(Path(c['path']).read_bytes())==c['sha256']
 (out/(c['name']+'.pdf')).unlink(missing_ok=True)
with tempfile.TemporaryDirectory(prefix='mo-line-style-') as profile:
 common=[str(args.soffice),'-env:UserInstallation='+Path(profile).as_uri(),'--headless']
 version=subprocess.run([*common,'--version'],capture_output=True,text=True,check=True,timeout=30).stdout.strip()
 run=subprocess.run([*common,'--convert-to','pdf:impress_pdf_Export','--outdir',str(out),*[c['path'] for c in cases]],capture_output=True,text=True,timeout=120)
 (root/'libreoffice.log').write_text(run.stdout+run.stderr);assert run.returncode==0,run.stderr
records=[]
for c in cases:
 path=out/(c['name']+'.pdf');raw=path.read_bytes()
 with fitz.open(path) as pdf:
  strokes=[{'width':d['width'],'color':d['color'],'dash':d['dashes'],'cap':d['lineCap'],'join':d['lineJoin'],'rect':list(d['rect'])} for d in pdf[0].get_drawings() if d['type'] in ['s','fs']]
 records.append({'name':c['name'],'sourceSha256':c['sha256'],'pdfPath':str(path),'pdfSha256':sha(raw),'strokes':strokes})
result={'format':'musteroffice.line-style-application-observations/1','application':{'version':version,'launcherSha256':sha(args.soffice.read_bytes())},'cases':records,'scope':'LibreOffice PDF first-page stroke observations only. No target-app fidelity acceptance; no kernel pixel comparison or editing roundtrip.'}
(root/'libreoffice.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'cases':len(records),'application':result['application']}))
