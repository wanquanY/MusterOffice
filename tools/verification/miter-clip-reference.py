"""Independent straight-stroke union oracle, with explicit coverage exclusions.

Construct segment rectangles, intersect their exterior tangent lines, clip that
polygon with a half-plane. This does not use the production bisector-coordinate
formula or Skia. Tests fully interior/exterior pixels at least one pixel from
polygon boundaries; it makes no antialias-boundary or target-application claim.
"""
import hashlib
import json
import math
from pathlib import Path
import numpy as np

U = 1 << 32
ROOT = Path('.codex-work/miter-clip')
cases = json.loads(Path('.codex-work/path-raster/parity.json').read_text())['cases']

def cross(a, b): return a[0]*b[1]-a[1]*b[0]
def unit(a): return a / np.linalg.norm(a)
def clip(poly, origin, direction, distance):
    result = []
    for a, b in zip(poly, poly[1:]+poly[:1]):
        da, db = np.dot(a-origin, direction)-distance, np.dot(b-origin, direction)-distance
        if da <= 0: result.append(a)
        if (da <= 0) != (db <= 0): result.append(a + (b-a)*da/(da-db))
    return result

def regions(points, closed, radius, limit, cap):
    # Zero-length verbs have no tangent and cannot introduce an extra join.
    pts = [p for i,p in enumerate(points) if i == 0 or not np.array_equal(p, points[i-1])]
    if closed and len(pts)>1 and np.array_equal(pts[-1],pts[0]): pts.pop()
    polygons, disks = [], []
    if len(pts) < 2:
        return polygons, disks
    pairs = list(zip(pts, pts[1:]+pts[:1] if closed else pts[1:]))
    for a,b in pairs:
        d=unit(b-a); n=np.array([-d[1],d[0]])*radius
        polygons.append([a+n,b+n,b-n,a-n])
    corners = range(len(pts)) if closed else range(1,len(pts)-1)
    for i in corners:
        p=pts[i]; v=unit(p-pts[i-1]); w=unit(pts[(i+1)%len(pts)]-p)
        z=cross(v,w)
        if z == 0: continue  # exact reversal or straight continuation
        sign=-1 if z>0 else 1
        n1=sign*np.array([-v[1],v[0]]); n2=sign*np.array([-w[1],w[0]])
        a,b=p+radius*n1,p+radius*n2
        t=cross(b-a,w)/z; tip=a+t*v
        direction=unit(tip-p)
        polygons.append(clip([p,a,tip,b],p,direction,limit*radius))
    if not closed:
        for p,d in [(pts[0],unit(pts[0]-pts[1])),(pts[-1],unit(pts[-1]-pts[-2]))]:
            n=np.array([-d[1],d[0]])*radius
            if cap=='square': polygons.append([p+n,p+n+d*radius,p-n+d*radius,p-n])
            if cap=='round': disks.append((p,radius))
    return polygons,disks

def masks(polygons, disks, x, y):
    inside=np.zeros(x.shape,dtype=bool); outside=np.ones(x.shape,dtype=bool)
    for poly in polygons:
        # Convex intersection polygons can contain duplicate vertices.
        area=sum(cross(a,b) for a,b in zip(poly,poly[1:]+poly[:1]))
        if not area: continue
        distances=[]
        for a,b in zip(poly,poly[1:]+poly[:1]):
            v=b-a; length=np.linalg.norm(v)
            if length:
                distances.append((1 if area>0 else -1)*(v[0]*(y-a[1])-v[1]*(x-a[0]))/length)
        distance=np.minimum.reduce(distances)
        inside |= distance>1
        outside &= distance < -1
    for p,r in disks:
        distance=np.hypot(x-p[0],y-p[1])
        inside |= distance<r-1
        outside &= distance>r+1
    return inside,outside

records=[]; total_inside=total_outside=0
for c in cases:
    if not c['name'].startswith('clip-') or c['status']!='rendered': continue
    raw=Path(c['requestPath']).read_bytes();pixels=Path(c['pixelsPath']).read_bytes()
    assert hashlib.sha256(raw).hexdigest()==c['requestSha256']
    assert hashlib.sha256(pixels).hexdigest()==c['pixelsSha256']
    q=json.loads(raw); commands=q['paths'][0]['commands'];draw=q['draws'][0];style=draw['stroke']
    if any(cmd['kind'] in ['quadratic','cubic'] for cmd in commands): continue
    # These have a well-defined exact zero-segment cap contract exercised in
    # the legacy suite; this oracle checks the non-degenerate path geometry.
    if c['name'] in ['clip-zero-segment','clip-move-only']: continue
    v=q['viewport'];scale=v['scale']['numerator']/v['scale']['denominator']
    points=[np.array([(int(cmd['to'][a])+int(draw['origin'][a])-int(v['origin'][a]))/U*scale for a in ['x','y']]) for cmd in commands if 'to' in cmd]
    width=int(style['width'])/U*scale; radius=(width if width else 1)/2
    polys,disks=regions(points,commands[-1]['kind']=='close',radius,int(style['join']['limit'])/U,style['cap'])
    y,x=np.mgrid[0:v['height'],0:v['width']].astype(float);x+=.5;y+=.5
    inside,outside=masks(polys,disks,x,y)
    rgba=np.frombuffer(pixels,dtype=np.uint8).reshape(v['height'],v['width'],4)
    alpha=draw['color'][3];red=np.array([alpha,0,0,alpha],dtype=np.uint8)
    bad=(inside & np.any(rgba != red,axis=2)) | (outside & np.any(rgba!=0,axis=2))
    assert not np.any(bad),(c['name'],np.argwhere(bad)[:8].tolist())
    ni,no=int(inside.sum()),int(outside.sum());total_inside+=ni;total_outside+=no
    records.append({'name':c['name'],'requestSha256':c['requestSha256'],'pixelsSha256':c['pixelsSha256'],
                    'interiorPixels':ni,'exteriorPixels':no,'unclassifiedBoundaryPixels':int((~(inside|outside)).sum())})

samples=[]
lookup={c['name']:c for c in cases}
for name,x,y,expected in [('clip-v-16-24-2',96,44,[255,0,0,255]),
                         ('clip-v-16-24-2',96,25,[0,0,0,0]),
                         ('clip-v-16-24-1',96,55,[255,0,0,255])]:
    c=lookup[name];pixels=Path(c['pixelsPath']).read_bytes();offset=(192*y+x)*4
    assert list(pixels[offset:offset+4])==expected,(name,x,y)
    samples.append({'name':name,'pixel':[x,y],'expected':expected,'pixelsSha256':c['pixelsSha256']})
report={'format':'musteroffice.miter-clip-reference/1','cases':records,'interiorPixels':total_inside,
        'exteriorPixels':total_outside,'distinguishingPixels':samples,
        'scope':'Independent polygon-union interior/exterior checks with a declared 1px exclusion. No curve offset, AA boundary, SVG conformance or Office/WPS equivalence claim.'}
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'cases':len(records),'interiorPixels':total_inside,'exteriorPixels':total_outside,'distinguishingPixels':len(samples)}))
