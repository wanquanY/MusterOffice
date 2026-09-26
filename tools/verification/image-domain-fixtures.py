"""Independent Fraction reference for bounded source-domain image sampling."""
from fractions import Fraction as F
import copy
import hashlib
import itertools
import json
import math
from pathlib import Path
import sys

root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=True)
Q=1<<32
W,H=24,20
width,height=5,4
pixels=[]
for y in range(height):
    for x in range(width):
        a=[255,128,0,192][(x+y)%4]
        rgb=[220 if x==0 else x*35,20+y*45,30+(x+y)*25]
        pixels.extend([(v*a+127)//255 for v in rgb]+[a])
data=bytes(pixels);bundle=root/'image.bin';bundle.write_bytes(data)
def entry(path):
    b=path.read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def fixed(v):
    v=F(v)*Q
    assert v.denominator==1
    return str(v.numerator)
def point(x,y): return dict(x=fixed(x),y=fixed(y))
def texel(x,y,d):
    if d[0]>=0 and d[2]<=width:x=min(max(x,0),width-1)
    if d[1]>=0 and d[3]<=height:y=min(max(y,0),height-1)
    if not (0<=x<width and 0<=y<height):return [F(0)]*4
    return list(map(F,pixels[(y*width+x)*4:(y*width+x+1)*4]))
def mix(a,b,t): return [x*(1-t)+y*t for x,y in zip(a,b)]
def linear(x,y,d):
    ix,iy=math.floor(x-F(1,2)),math.floor(y-F(1,2))
    fx,fy=x-F(1,2)-ix,y-F(1,2)-iy
    return mix(mix(texel(ix,iy,d),texel(ix+1,iy,d),fx),mix(texel(ix,iy+1,d),texel(ix+1,iy+1,d),fx),fy)
def axis(c,lo,hi,mode,sampling):
    length=hi-lo
    if mode=='repeat':c=(c-lo)%length+lo
    elif mode=='mirror':
        p=(c-lo)%(length*2);c=lo+min(p,length*2-p)
    if sampling=='linear':
        half=min(F(1,2),length/2);a,b=lo+half,hi-half
    else:a,b=F(math.floor(lo))+F(1,2),F(math.ceil(hi))-F(1,2)
    q=min(max(c,a),b)
    return c,q,a,b
def sample(x,y,d,tx,ty,sampling):
    x,qx,l,r=axis(x,d[0],d[2],tx,sampling)
    y,qy,t,b=axis(y,d[1],d[3],ty,sampling)
    if sampling=='nearest':
        if tx=='decal' and not d[0]<x<=d[2]:return [0]*4
        if ty=='decal' and not d[1]<y<=d[3]:return [0]*4
        color=texel(math.ceil(qx)-1,math.ceil(qy)-1,d)
    else:
        ex,ey=x-qx,y-qy;color=linear(qx,qy,d)
        opposite_x=l if ex>0 else r;opposite_y=t if ey>0 else b
        if tx=='repeat':color=mix(color,linear(opposite_x,qy,d),abs(ex))
        if ty=='repeat':
            other=linear(qx,opposite_y,d)
            if tx=='repeat':other=mix(other,linear(opposite_x,opposite_y,d),abs(ex))
            color=mix(color,other,abs(ey))
        if tx=='decal':color=[v*max(F(0),1-abs(ex)) for v in color]
        if ty=='decal':color=[v*max(F(0),1-abs(ey)) for v in color]
    return [math.floor(v+F(1,2)) for v in color]
def expected(d,tx,ty,sampling,m):
    a,b,c,e,ox,oy=map(F,m);det=a*e-b*c
    result=bytearray()
    for y in range(H):
        for x in range(W):
            px,py=F(2*x+1,2)-ox,F(2*y+1,2)-oy
            u,v=(e*px-b*py)/det,(a*py-c*px)/det
            result.extend(sample(u,v,d,tx,ty,sampling))
    return result
cases=[]
def add(name,d,tx,ty,sampling,m=(2,0,0,2,5,4),mutation=None):
    d=list(map(F,d));a,b,c,e,ox,oy=m
    image=dict(resource=0,origin=point(ox,oy),xStep=point(a,c),yStep=point(b,e),tileX=tx,tileY=ty,sampling=sampling,
               sourceDomain=dict(zip(['left','top','right','bottom'],map(fixed,d))))
    if mutation:mutation(image)
    path=dict(fillRule='nonzero',commands=[dict(kind='move',to=point(0,0))]+[dict(kind='line',to=point(x,y)) for x,y in [(W,0),(W,H),(0,H)]]+[dict(kind='close')])
    viewport=dict(width=W,height=H,origin=point(0,0),scale=dict(numerator=1,denominator=1),coordinateTolerance=str(1<<24),background=[0]*4)
    resources=[dict(width=width,height=height,alpha='premultiplied',sha256=entry(bundle)['sha256'])]
    draw=dict(path=0,origin=point(0,0),brush=dict(kind='image',image=image))
    request=dict(raster=dict(viewport=viewport,paths=[path],draws=[draw]),images=resources)
    p=root/(name+'.json');p.write_text(json.dumps(request))
    # Mirrored scene geometry covers the same viewport. Paint stays in world
    # coordinates, so source-domain coordinates must not be transformed twice.
    scene=dict(paths=[path],transforms=[dict(parent=None,affine=dict(linear=list(map(fixed,[-1,0,0,1])),translation=point(W,0)))],
               instances=[dict(path=0,transform=0,brush=draw['brush'])])
    s=root/(name+'.scene.json');s.write_text(json.dumps(dict(raster=dict(viewport=viewport,scene=scene),images=resources)))
    record=dict(name=name,request=entry(p),sceneRequest=entry(s),images=entry(bundle),success=mutation is None)
    if not mutation:
        out=root/(name+'.rgba');out.write_bytes(expected(d,tx,ty,sampling,m));record['expectedPixels']=entry(out)
    cases.append(record)

domains=[(1,0,4,3),(F(1,4),F(1,2),F(15,4),F(5,2)),(F(5,4),F(1,4),F(7,4),F(11,4)),
         (-F(1,2),-F(1,4),F(11,2),F(9,2)),(-4,-4,-1,-1),(-F(1,8),0,F(3,8),1),
         (F(1,8),F(1,8),F(3,8),F(3,8)),(F(37,8),F(29,8),F(39,8),F(31,8))]
tiles=['clamp','repeat','mirror','decal']
for index,d in enumerate(domains):
    for tx,ty,s in itertools.product(tiles,tiles,['nearest','linear']):add(f'domain-{index}-{tx}-{ty}-{s}',d,tx,ty,s)
matrices=[(-2,0,0,2,15,4),(0,-2,2,0,14,3),(2,1,0,2,3,2)]
for i,m in enumerate(matrices):
    for tx,s in itertools.product(tiles,['nearest','linear']):add(f'affine-{i}-{tx}-{s}',domains[1],tx,tx,s,m)
bad=[('empty',lambda b:b['sourceDomain'].update(right=b['sourceDomain']['left'])),
     ('reversed',lambda b:b['sourceDomain'].update(bottom=fixed(-10))),
     ('tiny',lambda b:b['sourceDomain'].update(left='0',right='1')),
     ('range',lambda b:b['sourceDomain'].update(right=fixed(32769))),
     ('projected-range',lambda b:b['sourceDomain'].update(right=fixed(20000))),
     ('unknown',lambda b:b['sourceDomain'].update(arbitrary=True)),
     ('missing',lambda b:b['sourceDomain'].pop('right')),
     ('numeric',lambda b:b['sourceDomain'].update(left=1)),
     ('noncanonical',lambda b:b['sourceDomain'].update(left='-0'))]
for name,mutation in bad:add('invalid-'+name,domains[0],'repeat','mirror','linear',mutation=mutation)
# Independent constant-field invariant, including all four image corners.
# Premultiplied translucent color also must not acquire additional transparency.
pixels=[80,30,100,128]*(width*height)
bundle=root/'uniform.bin';bundle.write_bytes(bytes(pixels))
for corner,d in enumerate([(F(1,8),F(1,8),F(3,8),F(3,8)),(F(37,8),F(1,8),F(39,8),F(3,8)),
                           (F(1,8),F(29,8),F(3,8),F(31,8)),(F(37,8),F(29,8),F(39,8),F(31,8))]):
    for tile,s in itertools.product(['clamp','repeat','mirror'],['nearest','linear']):
        name=f'constant-corner-{corner}-{tile}-{s}'
        add(name,d,tile,tile,s)
        assert (root/(name+'.rgba')).read_bytes()==bytes([80,30,100,128])*(W*H)
(root/'manifest.json').write_text(json.dumps(dict(cases=cases),indent=2)+'\n')
print(json.dumps(dict(cases=len(cases),success=sum(c['success'] for c in cases),referencePixels=sum(c['success'] for c in cases)*W*H)))
