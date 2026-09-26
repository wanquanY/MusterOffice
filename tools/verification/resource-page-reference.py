"""Independent interior-pixel oracle for original static source-page probes.
No compiled layout/paint fields supply expected pixel colors or coordinates.
"""
import hashlib,json,math,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/resource-page');parity=json.loads((root/'parity.json').read_text())
def entry(p):
 b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):assert entry(r['path'])==r;return Path(r['path']).read_bytes()
white=(255,255,255,255);green=(0,170,0,255);cyan=(0,255,255,255)
pattern=[(255,0,0,255),(0,0,0,0),(0,0,255,255),(255,255,0,255)]
ns=dict(p='http://schemas.openxmlformats.org/presentationml/2006/main',a='http://schemas.openxmlformats.org/drawingml/2006/main')
results=[]
for name in ['dual-fill','stretch-clip','distinct-images','background-shared','ellipse']:
 c=next(c for c in parity['cases'] if c['name']==name);pixels=load(c['pixels']);source=load(c['source']);q=json.loads(load(c['request']))
 assert q['sampling']=='nearest';assert q['page']['viewport']['width']==400 and q['page']['viewport']['height']==300
 with zipfile.ZipFile(c['source']['path']) as z:
  s=X.fromstring(z.read('ppt/slides/slide1.xml'));pic=s.find('.//p:pic',ns);t=pic.find('p:spPr/a:xfrm',ns)
  assert not t.attrib and t.find('a:off',ns).attrib==dict(x='0',y='0') and t.find('a:ext',ns).attrib==dict(cx='800000',cy='800000')
  assert pic.find('p:spPr/a:solidFill/a:srgbClr',ns).get('val')=='00AA00'
  assert pic.find('p:spPr/a:prstGeom',ns).get('prst')==('ellipse' if name=='ellipse' else 'rect')
  expected_rect=dict(l='25000',r='25000') if name=='stretch-clip' else {}
  assert pic.find('p:blipFill/a:stretch/a:fillRect',ns).attrib==expected_rect
  assert z.read('ppt/media/owned-image.png')==Path('fixtures/presentations/resource-page/transparent.png').read_bytes()
 checked=0;excluded=0
 for iy in range(300):
  y=iy+.5
  for ix in range(400):
   x=ix+.5
   # Exclude shape, hard clip and source-texel boundaries. The oracle concerns
   # opaque/transparent interiors, not antialias coverage or filtering math.
   xedges=[0,100,200];yedges=[0,100,200]
   if name=='stretch-clip':xedges += [50,150]
   if name=='background-shared':yedges += [150];xedges += [400]
   if any(abs(x-v)<2 for v in xedges) or any(abs(y-v)<2 for v in yedges) or name=='ellipse' and abs(math.hypot(x-100,y-100)-100)<2:
    excluded+=1;continue
   color=white
   if name=='background-shared':
    v=pattern[(int(y//150))*2+int(x//200)];color=v if v[3] else color
   inside=0<x<200 and 0<y<200
   if name=='ellipse':inside=math.hypot(x-100,y-100)<100
   if inside:
    color=green
    left,right=(50,150) if name=='stretch-clip' else (0,200)
    if left<x<right:
     v=pattern[int(y//100)*2+int((x-left)//((right-left)/2))]
     if v[3]:color=v
    if name=='distinct-images':color=cyan
   actual=tuple(pixels[(iy*400+ix)*4:(iy*400+ix+1)*4])
   assert actual==color,(name,ix,iy,actual,color)
   checked+=1
 results.append(dict(name=name,source=c['source'],request=c['request'],pixels=c['pixels'],interiorPixels=checked,excludedBoundaryPixels=excluded))
report=dict(format='musteroffice.resource-page-reference/1',scope='Independent original 2x2 PNG, XML fill/extent, rectangle/circle and compositing reference. Excludes antialias/clip/texel boundaries; no real-font or Office/WPS visual acceptance.',interiorPixels=sum(c['interiorPixels'] for c in results),cases=results)
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(cases=len(results),interiorPixels=report['interiorPixels'])))
