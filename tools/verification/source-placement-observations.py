"""Read owned shape pixels/PDF paths only; full UI captures remain private local data.

Window captures are manually grounded with cua-driver snapshots before/after
background launch. This script does not open, activate or edit any application.
"""
import hashlib,json,plistlib
from pathlib import Path
from PIL import Image
import fitz
ROOT=Path('.codex-work/source-placement')
sha=lambda b:hashlib.sha256(b).hexdigest()
# Exact slides bounds from the inspected window images; no toolbar/tab pixels.
probes=[
 ('root-scale','observations/root-scale','wps-root-scale','wps-root-state',[386,376,1135,798]),
 ('group-zero','observations/group-zero','wps-group-zero','wps-group-zero-state',[386,212,1135,634]),
 ('group-zero-rot','observations/group-zero-rot','wps-zero-rot','wps-zero-state',[386,212,1135,634]),
 ('group-positive','group-axis-2000001-1000003-0','wps-group-positive','wps-group-positive-state',[386,376,1135,798]),
 ('placeholder-absent','placeholder-absent','wps-placeholder-absent','wps-placeholder-absent-state',[386,376,1135,798]),
 ('placeholder-empty','placeholder-empty','wps-placeholder-empty','wps-placeholder-empty-state',[386,376,1135,798]),
 ('placeholder-rotation','placeholder-rotation','wps-placeholder-rotation','wps-placeholder-rotation-state',[386,376,1135,798]),
 ('placeholder-full','placeholder-full','wps-placeholder-full','wps-placeholder-full-state',[386,376,1135,798]),
]
cases=[]
for name,stem,screen,state,rect in probes:
    source=ROOT/(stem+'.pptx');image=ROOT/(screen+'.png');ax=json.loads((ROOT/(state+'.json')).read_text());assert ax['elements'][0]['label']==source.name,(name,ax['elements'][0]['label'])
    # Capture hashes identify local observation evidence without publishing
    # application tabs or other documents visible outside the inspected slide.
    im=Image.open(image).convert('RGB');pts=[]
    for y in range(rect[1],rect[3]):
        for x in range(rect[0],rect[2]):
            r,g,b=im.getpixel((x,y))
            if r>160 and g<90 and 25<b<130:pts.append((x-rect[0],y-rect[1]))
    bbox=[min(x for x,y in pts),min(y for x,y in pts),max(x for x,y in pts)+1,max(y for x,y in pts)+1] if pts else None
    pdf=ROOT/'observations'/(Path(stem).name+'.pdf');doc=fitz.open(pdf);boxes=[list(d['rect']) for d in doc[0].get_drawings() if d.get('fill') and abs(d['fill'][0]-219/255)<.01 and abs(d['fill'][1]-35/255)<.01]
    cases.append({'name':name,'sourcePath':str(source),'sourceSha256':sha(source.read_bytes()),'windowCaptureSha256':sha(image.read_bytes()),'slidePixelBounds':rect,'wpsRedPixelBoundsInSlide':bbox,'wpsRedPixelCount':len(pts),'pdfPath':str(pdf),'pdfSha256':sha(pdf.read_bytes()),'libreOfficeRedPathBoundsPoints':boxes})
apps={}
for name in ['wpsoffice','LibreOffice']:
    with open('/Applications/'+name+'.app/Contents/Info.plist','rb') as f:p=plistlib.load(f)
    apps[name]={'version':p['CFBundleShortVersionString'],'build':p['CFBundleVersion']}
by={c['name']:c for c in cases}
for name in ['group-zero','group-zero-rot','placeholder-rotation']:assert by[name]['wpsRedPixelCount']==0
for name in ['root-scale','group-positive','placeholder-absent','placeholder-empty','placeholder-full']:assert by[name]['wpsRedPixelCount']>0
for name in ['placeholder-absent','placeholder-empty']:
    b=by[name]['wpsRedPixelBoundsInSlide'];assert b[3]-b[1]>b[2]-b[0]
b=by['placeholder-full']['wpsRedPixelBoundsInSlide'];assert b[2]-b[0]>b[3]-b[1]
result={'format':'musteroffice.source-placement-observations/1','apps':apps,'scope':'Owned rectangle visibility and approximate bounds only; not full visual, native-editability or Office acceptance. WPS screenshots are private local window captures; only slide pixel statistics and capture digests enter this report.','cases':cases,'differences':['WPS shows no red child for the two zero child-extent group probes; LibreOffice has explicit red paths.','Empty local placeholder xfrm inherits the visible master rotation in WPS; the draft/LibreOffice resets orientation.','Rotation-only local placeholder xfrm has no visible red shape in WPS; the draft/LibreOffice inherits missing position and size.'],'notObserved':['Microsoft Office','WPS save/reopen/editability','all group and placeholder combinations']}
(ROOT/'observations.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'observations':len(cases),'differences':len(result['differences']),'apps':apps}))
