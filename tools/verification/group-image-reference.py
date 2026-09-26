"""Independent PNG + source XML interior reference, with application samples."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/group-image');parity=json.loads((root/'parity.json').read_text());observed=json.loads((root/'observations.json').read_text())
def entry(p):
 b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):assert entry(r['path'])==r;return Path(r['path']).read_bytes()
ns=dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',p='http://schemas.openxmlformats.org/presentationml/2006/main')
white=(255,255,255,255);pattern=[(255,0,0,255),(0,0,0,0),(0,0,255,255),(255,255,0,255)];results=[]
for name in ['siblings','root','different-receivers','stretch-clip','layout-placeholder']:
 actual=next(c for c in parity['cases'] if c['name']=='new/'+name);control=next(c for c in parity['cases'] if c['name']=='new/'+name+'-explicit');pixels=load(actual['pixels'])
 observation=next(c for c in observed['cases'] if c['name']==name);external=load(observation['pixels'])
 assert json.loads(load(actual['request']))['sampling']=='nearest'
 layers=[];xedges=[];yedges=[]
 # The explicit control differs only in where fill properties are declared;
 # it supplies raw XML extents/fills to this independent reference. No kernel
 # layout, placement, paths, clips, brush matrix or pixel result is consulted.
 with zipfile.ZipFile(control['source']['path']) as z:
  assert z.read('ppt/media/owned-image.png')==Path('fixtures/presentations/resource-page/transparent.png').read_bytes()
  s=X.fromstring(z.read('ppt/slides/slide1.xml'))
  for g in s.findall('.//p:grpSp/p:grpSpPr/a:xfrm',ns):
   assert not g.attrib and g.find('a:off',ns).attrib==g.find('a:chOff',ns).attrib==dict(x='0',y='0')
   assert g.find('a:ext',ns).attrib==g.find('a:chExt',ns).attrib
  for shape in s.findall('.//p:sp',ns):
   t=shape.find('p:spPr/a:xfrm',ns);assert not set(t.attrib)-{'flipH'}
   off=t.find('a:off',ns);size=t.find('a:ext',ns);x,y=[int(off.get(k))/4000 for k in ['x','y']];w,h=[int(size.get(k))/4000 for k in ['cx','cy']]
   flip=t.get('flipH')=='1';f=shape.find('p:spPr/a:blipFill/a:stretch/a:fillRect',ns)
   assert f is not None and not set(f.attrib)-{'l','r'}
   left=int(f.get('l','0'))/100000;right=1-int(f.get('r','0'))/100000
   for edge in [0,1,left,(left+right)/2,right]:xedges.append(x+w*(1-edge if flip else edge))
   yedges.extend([y,y+h/2,y+h]);layers.append((x,y,w,h,flip,left,right))
 checked=0;external_different=0;max_delta=0;difference_bounds=None
 for iy in range(300):
  for ix in range(400):
   px,py=ix+.5,iy+.5
   if any(abs(px-v)<2 for v in xedges) or any(abs(py-v)<2 for v in yedges):continue
   color=white
   for x,y,w,h,flip,left,right in layers:
    if x<px<x+w and y<py<y+h:
     u=(px-x)/w;u=1-u if flip else u
     if left<u<right:
      rgba=pattern[int(2*(py-y)/h)*2+int(2*(u-left)/(right-left))]
      if rgba[3]:color=rgba
   start=(iy*400+ix)*4;got=tuple(pixels[start:start+4]);assert got==color,(name,ix,iy,got,color)
   start=(iy*400+ix)*3;external_pixel=tuple(external[start:start+3]);delta=max(abs(a-b) for a,b in zip(external_pixel,color[:3]))
   if delta:
    difference_bounds=[ix,iy,ix,iy] if difference_bounds is None else [min(ix,difference_bounds[0]),min(iy,difference_bounds[1]),max(ix,difference_bounds[2]),max(iy,difference_bounds[3])]
   external_different+=int(delta!=0);max_delta=max(max_delta,delta);checked+=1
 results.append(dict(name=name,source=actual['source'],controlSource=control['source'],pixels=actual['pixels'],applicationPixels=observation['pixels'],interiorPixels=checked,applicationPixelsDifferent=external_different,maximumApplicationChannelDifference=max_delta,applicationDifferenceBounds=difference_bounds))
report=dict(format='musteroffice.group-image-reference/1',scope='Five original static 2x2 image cases with independent raw source XML rectangle/flip/clip reference. Excludes AA, shape/clip/texel boundaries; no advanced content or Office/WPS acceptance.',cases=results,interiorPixels=sum(c['interiorPixels'] for c in results),applicationPixelsDifferent=sum(c['applicationPixelsDifferent'] for c in results))
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k not in ['cases','scope']}))
