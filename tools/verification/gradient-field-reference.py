"""Independent raw DrawingML linear gradient oracle; no generated scene/frame inputs."""
import hashlib,json,math,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/gradient-field')
ns=dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',p='http://schemas.openxmlformats.org/presentationml/2006/main')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
files=sorted((root/'native').glob('*.pptx'));assert len(files)==17
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
    pct=lambda s:float(s[:-1])/100 if s.endswith('%') else int(s)/100000
    l,t,rr,b=[0 if r is None else pct(r.get(k,'0')) for k in ['l','t','r','b']]
    x+=l*w;y+=t*h;w*=1-l-rr;h*=1-t-b
    linear=g.find('a:lin',ns);ang=int(linear.get('ang','0'))/60000*math.pi/180
    nx,ny=math.cos(ang),math.sin(ang)
    if linear.get('scaled','0')!='1':nx*=w;ny*=h
    norm=abs(nx)+abs(ny);offset=max(-nx,0)+max(-ny,0)
    stops=[]
    for stop in g.findall('a:gsLst/a:gs',ns):
        c=stop.find('a:srgbClr',ns);rgb=[v/255 for v in bytes.fromhex(c.get('val'))];alpha=c.find('a:alpha',ns)
        stops.append((pct(stop.get('pos')),rgb+[1 if alpha is None else pct(alpha.get('val'))]))
    def sample(px,py):
        mirror=lambda v:1-abs((v%2)-1)
        t=(nx*mirror((px-x)/w)+ny*mirror((py-y)/h)+offset)/norm
        if t<=stops[0][0]:return stops[0][1]
        if t>=stops[-1][0]:return stops[-1][1]
        for i in range(len(stops)-1):
            pa,ca=stops[i];pb,cb=stops[i+1]
            if pa<=t<pb:return [a+(b-a)*(t-pa)/(pb-pa) for a,b in zip(ca,cb)]
        raise AssertionError(t)
    return sample
results=[]
for source in files:
    name=source.stem
    if name in ['group-inherited','group-inherited-control','rotated-ellipse']:continue
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
    for y in range(300):
        for x in range(400):
            bg=over_white(bg_grad(x+.5,y+.5)) if bg_grad else bg_solid
            pixel=bg
            for (l,t,w,h),window,g,c in objects:
                if l<=x+.5<l+w and t<=y+.5<t+h:
                    pixel=bg if window else over_white(g(x+.5,y+.5)) if g else c
            assert pixel is not None
            i=(y*400+x)*4;delta=max(abs(a-b) for a,b in zip(pixel,actual[i:i+4]))
            assert delta<=1,(name,x,y,pixel,list(actual[i:i+4]),delta)
            maximum=max(maximum,delta);different+=delta!=0;expected.extend(pixel)
    out=root/'reference';out.mkdir(exist_ok=True);path=out/(name+'.rgba');path.write_bytes(expected)
    results.append(dict(name=name,source=entry(source),pixels=entry(source.with_suffix('.rgba')),reference=entry(path),verifiedPixels=120000,maximumChannelDifference=maximum,pixelsDifferingByOne=different))
report=dict(format='musteroffice.gradient-field-reference/1',officialXsdParts=parts,sourceFiles=[entry(p) for p in files],cases=results,verifiedPixels=sum(r['verifiedPixels'] for r in results),scope='Raw native XML, logical rectangle, corrected scaled normal, independent two-axis reflection, source stop order, straight sRGB alpha; <=1 RGBA8 quantization step. Does not establish Office/WPS interoperability or cover rotated/group AA in this oracle.')
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(parts=parts,cases=len(results),pixels=report['verifiedPixels'],maximum=max(r['maximumChannelDifference'] for r in results))))
