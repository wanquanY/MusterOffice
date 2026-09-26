"""Official source grammar and independent rectangle/PNG/alpha source oracle."""
import hashlib, json, zipfile
from pathlib import Path
from lxml import etree as X
root = Path('.codex-work/compositing')
ns = dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',
          p='http://schemas.openxmlformats.org/presentationml/2006/main',
          r='http://schemas.openxmlformats.org/officeDocument/2006/relationships')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
files=sorted((root/'native').glob('*.pptx'));assert len(files)==30
parts=0
for p in files:
    with zipfile.ZipFile(p) as z:
        for n in z.namelist():
            if not n.endswith('.xml'):continue
            node=X.fromstring(z.read(n));q=X.QName(node)
            if q.namespace==ns['p'] and q.localname in ['sld','sldLayout','sldMaster','presentation']:
                schema.assertValid(node);parts+=1
def color(fill):
    c=fill.find('a:solidFill/a:srgbClr',ns)
    if c is None:return None
    rgb=bytes.fromhex(c.get('val'));a=c.find('a:alpha',ns)
    alpha=255 if a is None else (int(a.get('val'))*255+50000)//100000
    return tuple((v*alpha+127)//255 for v in rgb)+(alpha,)
def over(a,b):return tuple(a[i]+(b[i]*(255-a[3])+127)//255 for i in range(4))
results=[]
for name in ['opaque-white','alpha-opaque','alpha-clear','empty-clear','image-clear','clipped-image','multiple-windows','hidden-window']:
    base=root/'native'/name;q=json.loads(base.with_suffix('.request.json').read_text())
    assert q['viewport']['width']==400 and q['viewport']['height']==300
    assert q['viewport']['scale']==dict(numerator=1,denominator=4000)
    assert q['viewport']['origin']==dict(x='0',y='0')
    clear=q['viewport']['background'];clear=tuple((v*clear[3]+127)//255 for v in clear[:3])+(clear[3],)
    with zipfile.ZipFile(base.with_suffix('.pptx')) as z:
        s=X.fromstring(z.read('ppt/slides/slide1.xml'))
        assert z.read('ppt/media/owned-image.png')==Path('fixtures/presentations/resource-page/transparent.png').read_bytes()
    bg=s.find('p:cSld/p:bg/p:bgPr',ns);solid=color(bg);image=bg.find('a:blipFill',ns)
    left,right=0,1
    if image is not None:
        assert image.find('a:blip',ns).get('{'+ns['r']+'}embed')=='owned-image'
        fill=image.find('a:stretch/a:fillRect',ns);assert fill is not None
        assert not set(fill.attrib)-{'l','r'}
        left=int(fill.get('l','0'))/100000;right=1-int(fill.get('r','0'))/100000
    pattern=[(255,0,0,255),(0,0,0,0),(0,0,255,255),(255,255,0,255)]
    shapes=[]
    for shape in s.findall('p:cSld/p:spTree/p:sp',ns):
        if shape.find('p:nvSpPr/p:cNvPr',ns).get('hidden')=='1':continue
        sp=shape.find('p:spPr',ns);t=sp.find('a:xfrm',ns);assert not t.attrib
        assert sp.find('a:prstGeom',ns).get('prst')=='rect'
        off=t.find('a:off',ns);size=t.find('a:ext',ns)
        x,y=[int(off.get(k))/4000 for k in ['x','y']];w,h=[int(size.get(k))/4000 for k in ['cx','cy']]
        assert all(v==int(v) for v in [x,y,w,h])
        shapes.append((x,y,x+w,y+h,shape.get('useBgFill')=='1',color(sp)))
    expected=bytearray()
    for y in range(300):
        for x in range(400):
            background=over(solid,clear) if solid else clear
            if image is not None and left*400<=x+.5<right*400:
                texel=pattern[int((y+.5)*2/300)*2+int(((x+.5)/400-left)*2/(right-left))]
                background=over(texel,clear)
            pixel=background
            for l,t,r,b,window,fill in shapes:
                if l<=x+.5<r and t<=y+.5<b:
                    if window:pixel=background
                    elif fill:pixel=over(fill,pixel)
            expected.extend(pixel)
    actual=base.with_suffix('.rgba').read_bytes();assert bytes(expected)==actual,name
    output=root/'source-runtime'/(name+'.expected.rgba');output.parent.mkdir(exist_ok=True);output.write_bytes(expected)
    results.append(dict(name=name,source=entry(base.with_suffix('.pptx')),pixels=entry(base.with_suffix('.rgba')),reference=entry(output),verifiedPixels=400*300))
report=dict(format='musteroffice.compositing-reference/1',officialXsdParts=parts,sourceFiles=[entry(p) for p in files],cases=results,verifiedPixels=sum(c['verifiedPixels'] for c in results),scope='Raw XML rectangles, original 2x2 PNG colors and integer alpha reference. Full integer-aligned pixels; excludes the rotated ellipse and tiled-image case from this independent oracle.')
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(officialXsdParts=parts,cases=len(results),verifiedPixels=report['verifiedPixels'])))
