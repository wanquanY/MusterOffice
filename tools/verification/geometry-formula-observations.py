"""Measure simple guide-controlled rectangles in an explicit LibreOffice build.

This is a diagnostic application observation. It is not an Office/WPS oracle,
and does not make malformed formulas or undocumented builtins conforming.
"""
import argparse
import hashlib
import json
import subprocess
import tempfile
import zipfile
from pathlib import Path
import fitz
from lxml import etree as E
from mce_reference import A,P
p=argparse.ArgumentParser();p.add_argument('--soffice',type=Path,required=True);args=p.parse_args()
ROOT=Path('.codex-work/geometry-eval/application-probes');ROOT.mkdir(exist_ok=True)
OUT=ROOT/'pdf';OUT.mkdir(exist_ok=True)
NS={'a':A,'p':P};sha=lambda b:hashlib.sha256(b).hexdigest()
base=json.loads(Path('.codex-work/source-geometry/manifest.json').read_text())['base']
assert sha(Path(base['path']).read_bytes())==base['sha256']
with zipfile.ZipFile(base['path']) as z:original={n:z.read(n) for n in z.namelist()}
cases=[]
for name,formula,reference,explanation in [
    ('control','val 914400',72,'literal one inch'),
    ('wd32','val wd32',22.5,'width divided by 32 hypothesis'),
    ('hd10','val hd10',28.8,'height divided by 10 hypothesis'),
    ('wd12','val wd12',60,'width divided by 12 hypothesis'),
    ('cd3','*/ cd3 w 21600000',240,'one third circle hypothesis'),
    ('extra-argument','+- 914400 914400 0 0',144,'ignore extra argument hypothesis'),
    ('sqrt-negative','sqrt -836127360000',72,'Microsoft absolute-value square root'),
    ('atan-quadrant','at2 -1 1',637.7952755905512,'raw 135-degree guide units used as coordinate'),
    ('division-zero','*/ 914400 1 0',None,'undefined arithmetic observation'),
]:
    parts=dict(original);root=E.fromstring(parts['ppt/slides/slide1.xml']);tree=root.find('p:cSld/p:spTree',NS)
    for n in list(tree):
        if E.QName(n).localname not in ['nvGrpSpPr','grpSpPr']:tree.remove(n)
    xml=f'''<p:sp xmlns:p="{P}" xmlns:a="{A}"><p:nvSpPr><p:cNvPr id="2" name="Owned geometry probe"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="9144000" cy="3657600"/></a:xfrm><a:custGeom><a:gdLst><a:gd name="g" fmla="{formula}"/></a:gdLst><a:pathLst><a:path w="9144000" h="3657600" stroke="false"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="g" y="0"/></a:lnTo><a:lnTo><a:pt x="g" y="914400"/></a:lnTo><a:lnTo><a:pt x="0" y="914400"/></a:lnTo><a:close/></a:path></a:pathLst></a:custGeom><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>'''
    tree.append(E.fromstring(xml));parts['ppt/slides/slide1.xml']=E.tostring(root,xml_declaration=True,encoding='UTF-8')
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    cases.append({'name':name,'formula':formula,'sourcePath':str(path),'sourceSha256':sha(path.read_bytes()),'referenceWidthPt':reference,'referenceMeaning':explanation})
    (OUT/(name+'.pdf')).unlink(missing_ok=True)
with tempfile.TemporaryDirectory(prefix='mo-geometry-observer-') as profile:
    common=[str(args.soffice),'-env:UserInstallation='+Path(profile).as_uri(),'--headless']
    version=subprocess.run([*common,'--version'],capture_output=True,text=True,check=True,timeout=30).stdout.strip()
    run=subprocess.run([*common,'--convert-to','pdf:impress_pdf_Export','--outdir',str(OUT),*[c['sourcePath'] for c in cases]],capture_output=True,text=True,timeout=60)
    (ROOT/'application.log').write_text(run.stdout+run.stderr);assert run.returncode==0,run.stderr
for c in cases:
    path=OUT/(c['name']+'.pdf');c.update(pdfPath=str(path),pdfSha256=sha(path.read_bytes()))
    with fitz.open(path) as pdf:
        drawings=[d for d in pdf[0].get_drawings() if d['fill'] is not None and abs(d['fill'][0]-1)<1e-6 and abs(d['fill'][1])+abs(d['fill'][2])<1e-6]
        c['redPaths']=[{'rectPt':list(d['rect']),'widthPt':d['rect'].width,'heightPt':d['rect'].height} for d in drawings]
        c['referenceDifferencePt']=None if len(drawings)!=1 or c['referenceWidthPt'] is None else drawings[0]['rect'].width-c['referenceWidthPt']
result={'format':'musteroffice.geometry-guide-application-observations/1','application':{'version':version,'launcherSha256':sha(args.soffice.read_bytes())},'base':base,'cases':cases,
        'scope':'First-page red PDF path bounds of owned probe rectangles. References include explicit hypotheses; observations do not authorize builtin or malformed-formula repair. No Office/WPS acceptance, kernel frame comparison or editing roundtrip.'}
(ROOT/'observations.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'application':result['application'],'observations':[{k:c[k] for k in ['name','redPaths','referenceDifferencePt']} for c in cases]}))
