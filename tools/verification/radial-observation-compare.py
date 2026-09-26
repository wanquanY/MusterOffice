"""Quantify owned WPS screenshots against explicit, competing geometric hypotheses.

This is an observation probe, not the kernel renderer or an Office oracle.
Full window captures stay private; only their hashes and owned canvas crops
are retained in the report. Registration locations were inspected visually.
"""
import hashlib,json,plistlib,zipfile
from decimal import Decimal as D
from pathlib import Path
import numpy as np
from PIL import Image
from lxml import etree as X

root=Path('.codex-work/radial-observation');out=root/'comparisons';out.mkdir(exist_ok=True)
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
ns=dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',p='http://schemas.openxmlformats.org/presentationml/2006/main')
parser=X.XMLParser(resolve_entities=False,no_network=True)
sources=json.loads((root/'sources.json').read_text())
app=Path('/Applications/wpsoffice.app/Contents/Info.plist');version=plistlib.loads(app.read_bytes())
assert version['CFBundleIdentifier']=='com.kingsoft.wpsoffice.mac'

def registration(im):
    assert im.shape==(1063,1568,3)
    # Four native magenta marks on the owned page, away from app chrome.
    mask=(im[:,:,0]>245)&(im[:,:,1]<20)&(im[:,:,2]>245)
    y,x=np.where(mask);keep=(x>200)&(x<1100)&(y>300)&(y<900);x,y=x[keep],y[keep]
    centers=[]
    for right,down in [(False,False),(True,False),(False,True),(True,True)]:
        m=((x>650)==right)&((y>600)==down);assert 20<=sum(m)<=100
        assert np.ptp(x[m])<12 and np.ptp(y[m])<12
        centers.append([float(np.mean(x[m])+.5),float(np.mean(y[m])+.5)])
    assert abs(centers[0][0]-centers[2][0])<.5 and abs(centers[1][1]-centers[0][1])<.5
    sx=(centers[1][0]-centers[0][0])/1450000;sy=(centers[2][1]-centers[0][1])/1050000
    ox=centers[0][0]-60000*sx;oy=centers[0][1]-60000*sy
    return dict(markerCenters=centers,scale=[sx,sy],origin=[ox,oy],note='Pure-color marker centers; screenshot quantization and resampling remain observation uncertainty.')

def ellipse(q,c,s):
    def inside(t):
        d=s+(1-s)*t[...,None];v=q-c*(1-t[...,None]);zero=d==0
        good=np.all(~zero|(v==0),axis=-1)
        return good&(np.sum(np.where(zero,0,(v/np.where(zero,1,d))**2),axis=-1)<=1)
    lo=np.zeros(q.shape[:-1]);hi=np.ones_like(lo);initial=inside(lo);seen=initial.copy();exits=0;last=initial
    # Locate first observed entry; do not presume nested ellipses in this probe.
    for k in range(1,129):
        current=inside(np.full_like(lo,k/128));new=current&~seen
        lo=np.where(new,(k-1)/128,lo);hi=np.where(new,k/128,hi)
        exits+=int(np.count_nonzero(last&~current));seen|=current;last=current
    assert seen.all(),'owned points must be inside final outer circle'
    for _ in range(45):
        t=(lo+hi)/2;yes=inside(t);hi=np.where(yes,t,hi);lo=np.where(yes,lo,t)
    return np.where(initial,0,(lo+hi)/2),exits

def metrics(actual,scalar):
    expected=32+192*scalar;d=np.abs(actual.astype(float)-expected)
    return dict(meanAbsolute=float(np.mean(d)),percentile99=float(np.percentile(d,99)),maximum=float(np.max(d)),pixels=int(d.size))

cases=[]
for case in sources['cases']:
    assert entry(case['source']['path'])==case['source']
    name=case['name'];capture=root/(name+'.png');state=root/(name+'.json')
    data=json.loads(state.read_text());assert (data['pid'],data['window_id'])==(763,249)
    assert 'radial-'+name+'.pptx' in data['tree_markdown'],name
    im=np.asarray(Image.open(capture).convert('RGB'));reg=registration(im);sx,sy=reg['scale'];ox,oy=reg['origin']
    # Fixed original XML, not the evaluated kernel plan, supplies the hypothesis.
    with zipfile.ZipFile(case['source']['path']) as z:page=X.fromstring(z.read('ppt/slides/slide1.xml'),parser)
    sp=page.find('p:cSld/p:spTree/p:sp',ns);g=sp.find('p:spPr/a:gradFill',ns)
    f=g.find('a:path/a:fillToRect',ns);assert g.find('a:path',ns).get('path')=='circle'
    def pct(v):return D(v[:-1])/100 if v.endswith('%') else D(v)/100000
    l,t,r,b=[pct(f.get(k)) for k in ['l','t','r','b']]
    sums=[l+r,t+b];scales=np.array([float(1-v) for v in sums]);square=np.array([float(l-r),float(t-b)])
    aspect=np.array([1200000,600000])/np.hypot(1200000,600000)
    anchor=np.array([0.0 if sums[i]==0 else square[i]*aspect[i] for i in range(2)])
    rounded=np.where(scales==1,0,anchor)
    radius=np.hypot(1200000,600000)/2
    xs,ys=np.meshgrid(np.arange(np.ceil(ox+110000*sx),np.floor(ox+1290000*sx),2).astype(int),np.arange(np.ceil(oy+210000*sy),np.floor(oy+790000*sy),2).astype(int))
    q=np.stack([((xs+.5-ox)/sx-700000)/radius,((ys+.5-oy)/sy-500000)/radius],axis=-1)
    actual=im[ys,xs,0];assert np.max(np.ptp(im[ys,xs].astype(int),axis=-1))<=1
    models={}
    for key,center in [('squareFocus',square),('anchorFocusExact',anchor),('anchorFocusRoundedEqual',rounded)]:
        scalar,exits=ellipse(q,center,scales);models[key]={**metrics(actual,scalar),'sampledInsideToOutsideTransitions':exits}
    if name=='center-ellipse':
        distance=np.linalg.norm(q,axis=-1);inner=distance/np.linalg.norm(q/scales,axis=-1)
        ray=np.clip((distance-inner)/(1-inner),0,1)
        models['centeredRay']=metrics(actual,ray)
    box=(int(np.ceil(ox+100000*sx)),int(np.ceil(oy+200000*sy)),int(np.floor(ox+1300000*sx)),int(np.floor(oy+800000*sy)))
    crop=out/(name+'-owned.png');Image.open(capture).crop(box).save(crop)
    cases.append(dict(name=name,source=case['source'],capture=entry(capture),state=entry(state),ownedCrop=entry(crop),registration=reg,models=models))
    print(json.dumps(dict(name=name,models={k:round(v['meanAbsolute'],5) for k,v in models.items()})),flush=True)
report=dict(format='musteroffice.radial-wps-observation/1',application=dict(bundleId=version['CFBundleIdentifier'],version=version['CFBundleShortVersionString'],build=version['CFBundleVersion']),
            inputs=entry(root/'sources.json'),cases=cases,sampledPixels=sum(v['models']['anchorFocusExact']['pixels'] for v in cases),
            scope='11 original PPTX files actually opened in WPS; visually inspected owned page and four-marker registration. Competing scalar models use original XML, not kernel output. First-entry probe samples 128 intervals then bisects; not a certified quartic solver or production renderer.',
            limitations=['Screenshots are resampled and have subpixel registration uncertainty; no pixel-exact interoperability claim.',
                         'Only axis-aligned rectangular receivers and these focus cases were observed; no arbitrary shape/tile or PowerPoint acceptance.',
                         'Near-equal-width differs from exact source arithmetic. Rounded-equal is an observational hypothesis, not kernel behavior.',
                         'Ellipse interpolation is inferred from these observations; a complete bounded scalar solver remains to be implemented.'])
(root/'observations.json').write_text(json.dumps(report,indent=2)+'\n')
