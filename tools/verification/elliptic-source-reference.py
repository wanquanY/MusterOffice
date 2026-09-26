"""Independent raw DrawingML circle gradient oracle; no generated scene/frame inputs."""
import hashlib,json,math,zipfile
from pathlib import Path
from lxml import etree as X
from fractions import Fraction as F
from decimal import Decimal as D, localcontext
from elliptic_polynomial_reference import inside, poly, sturm, at, variations

def first(values):
    if inside(values): return 0.
    p=poly(values);seq=sturm(p);start=variations(seq,F(0))
    if start==variations(seq,F(1)): return 1.
    a,b=F(0),F(1)
    for _ in range(36):
        mid=(a+b)/2;left=start-variations(seq,mid)
        if left:
            if left==1 and at(p,mid)==0:return float(mid)
            b=mid
        else:a=mid
    return float((a+b)/2)

root=Path('.codex-work/elliptic-source')
ns=dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',p='http://schemas.openxmlformats.org/presentationml/2006/main')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
files=sorted((root/'native-final').glob('*.pptx'));assert len(files)==29
parts=0
for source in files:
    with zipfile.ZipFile(source) as z:
        for n in z.namelist():
            if not n.endswith('.xml'):continue
            e=X.fromstring(z.read(n));q=X.QName(e)
            if q.namespace==ns['p'] and q.localname in ['sld','sldMaster','sldLayout','presentation']:
                schema.assertValid(e);parts+=1

def solid(node):
    c=node.find('a:solidFill/a:srgbClr',ns)
    return list(bytes.fromhex(c.get('val')))+[255] if c is not None else None

def gradient(node,box):
    g=node.find('a:gradFill',ns)
    if g is None:return None
    x,y,w,h=box;r=g.find('a:tileRect',ns)
    pct=lambda s:F(s[:-1])/100 if s.endswith('%') else F(s)/100000
    l,t,rr,b=[0 if r is None else pct(r.get(k,'0')) for k in ['l','t','r','b']]
    x+=l*w;y+=t*h;w*=1-l-rr;h*=1-t-b
    path=g.find('a:path',ns);assert path.get('path')=='circle'
    focus=path.find('a:fillToRect',ns)
    margins=[pct(focus.get(k,'0')) if focus is not None else 0 for k in ['l','t','r','b']]
    stops=[]
    for stop in g.findall('a:gsLst/a:gs',ns):
        c=stop.find('a:srgbClr',ns);rgb=[v/255 for v in bytes.fromhex(c.get('val'))];alpha=c.find('a:alpha',ns)
        stops.append((pct(stop.get('pos')),rgb+[1 if alpha is None else pct(alpha.get('val'))]))
    def sample(px,py):
        mirror=lambda v:1-abs((v%2)-1)
        u,v=mirror((px-x)/w),mirror((py-y)/h)
        # Source percentages are classified as exact rationals. Independent
        # high precision geometry is rounded only here to binary64 for Sturm;
        # no Rust layout result or encoded frame enters this oracle.
        with localcontext() as ctx:
            ctx.prec=100
            wd,hd=D(str(float(w))),D(str(float(h)))
            radius=(wd*wd+hd*hd).sqrt()/2
            scales=[1-margins[i]-margins[i+2] for i in range(2)]
            centers=[]
            for i,extent in enumerate([wd,hd]):
                # Exact zero margin sum retains the center (V2 policy).
                shift=F(0) if margins[i]+margins[i+2]==0 else (margins[i]-margins[i+2])/2
                centers.append(float(extent*(D(shift.numerator)/D(shift.denominator))/radius))
            q=[float((D(str(float(u)))-D('.5'))*wd/radius),float((D(str(float(v)))-D('.5'))*hd/radius)]
        t=first([*q,*centers,*map(float,scales)])
        if t<=stops[0][0]:return stops[0][1]
        if t>=stops[-1][0]:return stops[-1][1]
        for i in range(len(stops)-1):
            pa,ca=stops[i];pb,cb=stops[i+1]
            if pa<=t<pb:
                u=(t-pa)/(pb-pa)
                special=(len(stops)==2 and stops[0][0]==0 and stops[-1][0]==1) or (len(stops)==3 and stops[0][0]==0 and stops[-1][0]==1 and 0<stops[1][0]<1 and stops[0][1]==stops[-1][1])
                return [a+(b-a)*((1-(1-u)**1.875 if b>a else u**1.875) if special and j<3 else u) for j,(a,b) in enumerate(zip(ca,cb))]
        raise AssertionError(t)
    return sample
results=[];excluded=[]
for source in files:
    name=source.stem
    if name not in ['center-point','off-center','focus-line','outset','focus-area','zero-edge','whole-focus','three-percent','tiny-margin','decimal-point','tiled','alpha','hard-stops','background','background-window']:
        excluded.append(dict(name=name,reason='This source oracle verifies axis-aligned rectangular coverage, not curved/rotated AA.'));continue
    with zipfile.ZipFile(source) as z:page=X.fromstring(z.read('ppt/slides/slide1.xml'))
    bg=page.find('p:cSld/p:bg/p:bgPr',ns);bg_grad=gradient(bg,(0,0,400,300));bg_solid=solid(bg)
    objects=[]
    for sp in page.findall('p:cSld/p:spTree/p:sp',ns):
        props=sp.find('p:spPr',ns);xf=props.find('a:xfrm',ns);assert not xf.attrib
        origin=xf.find('a:off',ns);ext=xf.find('a:ext',ns)
        box=tuple(int(n.get(k))/4000 for n,k in [(origin,'x'),(origin,'y'),(ext,'cx'),(ext,'cy')])
        assert all(v==int(v) for v in box)
        objects.append((box,sp.get('useBgFill')=='1',gradient(props,box),solid(props)))
    def over_white(c):
        # Reference interpolation in straight sRGB, then premultiply and round.
        premul=[math.floor(c[k]*c[3]*255+.5) for k in range(3)];alpha=math.floor(c[3]*255+.5)
        return [v+255-alpha for v in premul]+[255]
    actual=source.with_suffix('.rgba').read_bytes();expected=bytearray();maximum=0;different=0
    checked=0
    for y in range(0,300,16):
        for x in range(0,400,16):
            bg=over_white(bg_grad(x+.5,y+.5)) if bg_grad else bg_solid
            pixel=bg
            for (l,t,w,h),window,g,c in objects:
                if l<=x+.5<l+w and t<=y+.5<t+h:
                    pixel=bg if window else over_white(g(x+.5,y+.5)) if g else c
            assert pixel is not None
            i=(y*400+x)*4;delta=max(abs(a-b) for a,b in zip(pixel,actual[i:i+4]))
            assert delta<=1,(name,x,y,pixel,list(actual[i:i+4]),delta)
            maximum=max(maximum,delta);different+=delta!=0;expected.extend(pixel);checked+=1
    out=root/'reference';out.mkdir(exist_ok=True);path=out/(name+'.sampled-rgba');path.write_bytes(expected)
    results.append(dict(name=name,source=entry(source),pixels=entry(source.with_suffix('.rgba')),reference=entry(path),verifiedPixels=checked,samplingStep=16,maximumChannelDifference=maximum,pixelsDifferingByOne=different))
report=dict(format='musteroffice.elliptic-source-reference/1',officialXsdParts=parts,sourceFiles=[entry(p) for p in files],cases=results,excluded=excluded,verifiedPixels=sum(r['verifiedPixels'] for r in results),scope='Raw native XML percentages, Decimal100 circumscribed geometry then exact Sturm first-root topology of binary64 parameters; source-defined Office ramp, straight alpha and integer rectangle coverage. Step16 sampled source pixels, <=1 RGBA8 difference. Does not establish target-application interoperability or curved/rotated AA.')
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(parts=parts,cases=len(results),pixels=report['verifiedPixels'],maximum=max(r['maximumChannelDifference'] for r in results))))
