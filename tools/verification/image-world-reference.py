"""Exact-rational world-paint verification and a shared-scene test bridge.

The bridge is verification code, not a second native page renderer. Native
placement/local layout are inputs; this checks their composition and clipping.
"""
from fractions import Fraction as F
import hashlib
import itertools
import json
import math
from pathlib import Path
import sys

ROOT=Path('.codex-work/image-world');Q=1<<32

def entry(path):
    b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def read(r):
    assert entry(r['path'])==r
    return json.loads(Path(r['path']).read_text())
def fixed(v):return F(int(v),Q)
def point(p):return [fixed(p[k]) for k in ['x','y']]
def raw(v):
    v*=Q;assert v.denominator==1;return str(v.numerator)
def qp(p):return dict(zip(['x','y'],map(raw,p)))
def rect(r):return [fixed(r[k]) for k in ['left','top','right','bottom']]
def corners(values,errors):return itertools.product(*[(v-e,v+e) if e else (v,) for v,e in zip(values,errors)])
def map_point(m,t,p):return [m[i*2]*p[0]+m[i*2+1]*p[1]+t[i] for i in range(2)]
def inverse(m,t,p):
    x,y=p[0]-t[0],p[1]-t[1];d=m[0]*m[3]-m[1]*m[2]
    return [(m[3]*x-m[1]*y)/d,(m[0]*y-m[2]*x)/d]
IDENTITY=dict(linear=[str(Q),'0','0',str(Q)],translation=dict(x='0',y='0'))
def basis(plan):
    p=plan['placement']
    if p is None:return IDENTITY,[F(0)]*2,[F(0)]*4,[F(0)]*2
    a=p['affine'];anchor=[fixed(p['anchor'][k])-F(int(p['sourceOrigin'][k])) for k in ['x','y']]
    return a,anchor,list(map(fixed,p['uncertainty']['linear'])),point(p['uncertainty']['translation'])
def path(points):return dict(fillRule='nonzero',commands=[dict(kind='move' if i==0 else 'line',to=qp(p)) for i,p in enumerate(points)]+[dict(kind='close')])
def values4(r):return [[r[0],r[1]],[r[2],r[1]],[r[2],r[3]],[r[0],r[3]]]

fixtures=read(entry(ROOT/'fixtures.json'))
if '--pixels' not in sys.argv:
    records=[];comparisons=0;paints=0
    for c in fixtures['cases']:
        if not c['success']:continue
        output=read(c['output']);plans=output['result']['plans'];compiled=output['paint']['paints']
        assert [p['target'] for p in plans]==[p['target'] for p in compiled]
        scene=dict(paths=[],transforms=[],clips=[],instances=[]);upstream=F(0)
        order=sorted(range(len(plans)),key=lambda i:plans[i]['target']['kind']!='background')
        for i in order:
            plan,paint=plans[i],compiled[i];l=plan['layout'];e=l['uncertainty'];a,anchor,me,te=basis(plan)
            m=list(map(fixed,a['linear']));t=point(a['translation']);origin=[x-y for x,y in zip(point(l['origin']),anchor)]
            oe=point(e['origin']);step=point(l['pixelStep']);se=point(e['pixelStep']);b=paint['brush']
            be=b.get('uncertainty',dict(origin=dict(x='0',y='0'),xStep=dict(x='0',y='0'),yStep=dict(x='0',y='0'),sourceDomain=['0']*4))
            for row in range(2):
                actual=point(b['origin'])[row];bound=point(be['origin'])[row]
                for aa,bb,tt,x,y in corners([m[row*2],m[row*2+1],t[row],*origin],[me[row*2],me[row*2+1],te[row],*oe]):
                    assert abs(aa*x+bb*y+tt-actual)<=bound,c['name'];comparisons+=1
                for axis,key in enumerate(['xStep','yStep']):
                    actual=point(b[key])[row];bound=point(be[key])[row]
                    for aa,s in corners([m[row*2+axis],step[axis]],[me[row*2+axis],se[axis]]):
                        assert abs(aa*s-actual)<=bound,c['name'];comparisons+=1
            assert b['sourceDomain']==l['sourceRectangle'] and be['sourceDomain']==e['sourceRectangle']
            assert [b['tileX'],b['tileY']]==[l['tileX'],l['tileY']]
            clip=paint['fillClip'];fr=rect(l['fillRectangle']);fe=list(map(fixed,e['fillRectangle']))
            transform=len(scene['transforms']);scene['transforms'].append(dict(parent=None,affine=a))
            clip_id=None
            if clip:
                assert l['clipToFillRectangle'] and clip['affine']==a
                expected=path([[x-anchor[0],y-anchor[1]] for x,y in values4(fr)])
                assert clip['path']==expected
                ce=point(clip['upstreamError']);upstream=max(upstream,*ce)
                for xidx,yidx in [[0,1],[2,1],[2,3],[0,3]]:
                    p=[fr[xidx]-anchor[0],fr[yidx]-anchor[1]];pe=[fe[xidx],fe[yidx]];nominal=map_point(m,t,p)
                    for row in range(2):
                        for aa,bb,tt,x,y in corners([m[row*2],m[row*2+1],t[row],*p],[me[row*2],me[row*2+1],te[row],*pe]):
                            assert abs(aa*x+bb*y+tt-nominal[row])<=ce[row],c['name'];comparisons+=1
                clip_path=len(scene['paths']);scene['paths'].append(clip['path']);clip_id=len(scene['clips'])
                scene['clips'].append(dict(parent=None,path=clip_path,transform=transform))
            else:assert not l['clipToFillRectangle']
            size=plan['placement']['sourceSize'] if plan['placement'] else fixtures['pageSize']
            points=[[x-anchor[0],y-anchor[1]] for x,y in values4([F(0),F(0),F(int(size['width'])),F(int(size['height']))])]
            for p in points:
                for row in range(2):upstream=max(upstream,te[row]+me[row*2]*abs(p[0])+me[row*2+1]*abs(p[1]))
            idx=len(scene['paths']);scene['paths'].append(path(points));scene['instances'].append(dict(path=idx,transform=transform,clip=clip_id,brush=dict(kind='image',image=b)))
            paints+=1
        error=math.ceil(upstream*Q/fixtures['emuPerPixel']);assert 16777216-error>=256
        viewport=dict(width=64,height=48,origin=dict(x='0',y='0'),scale=dict(numerator=1,denominator=fixtures['emuPerPixel']),coordinateTolerance=str(16777216-error),background=[0,0,0,0])
        manifest=[dict(width=5,height=4,alpha='premultiplied',sha256=fixtures['pixels']['sha256'])]
        p=ROOT/'cases'/(c['name']+'.scene.json');p.write_text(json.dumps(dict(raster=dict(viewport=viewport,scene=scene),images=manifest)))
        records.append(dict(name=c['name'],sourceOutput=c['output'],request=entry(p),upstreamGeometryBoundRaw=error))
    report=dict(format='musteroffice.image-world-reference/1',fixtures=entry(ROOT/'fixtures.json'),paints=paints,exactComparisons=comparisons,cases=records)
    (ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(paints=paints,exactComparisons=comparisons,scenes=len(records))))
else:
    runtime=read(entry(ROOT/'parity.json'));count=0
    pixels=Path(fixtures['pixels']['path']).read_bytes();records=[]
    for c in runtime['cases']:
        source=read(c['sourceOutput']);plans=source['result']['plans'];actual=Path(c['pixels']['path']).read_bytes();assert entry(c['pixels']['path'])==c['pixels']
        order=sorted(range(len(plans)),key=lambda i:plans[i]['target']['kind']!='background');checked=0
        for y,x in itertools.product(range(48),range(64)):
            sample=[F(2*x+1,2)*fixtures['emuPerPixel'],F(2*y+1,2)*fixtures['emuPerPixel']];safe=True;expected=bytes(4)
            for i in order:
                plan=plans[i];l=plan['layout'];a,anchor,_,_=basis(plan);m=list(map(fixed,a['linear']));t=point(a['translation'])
                local=[u+v for u,v in zip(inverse(m,t,sample),anchor)]
                size=plan['placement']['sourceSize'] if plan['placement'] else fixtures['pageSize']
                boxes=[[F(0),F(0),F(int(size['width'])),F(int(size['height']))]]
                if l['clipToFillRectangle']:boxes.append(rect(l['fillRectangle']))
                visible=True
                for box in boxes:
                    pts=[map_point(m,t,[px-anchor[0],py-anchor[1]]) for px,py in values4(box)]
                    for j in range(4):
                        aa,bb=pts[j],pts[(j+1)%4];dx,dy=bb[0]-aa[0],bb[1]-aa[1]
                        distance=abs(float(dx*(sample[1]-aa[1])-dy*(sample[0]-aa[0])))/math.hypot(float(dx),float(dy))/fixtures['emuPerPixel']
                        if distance<2:safe=False
                    visible &= box[0]<local[0]<box[2] and box[1]<local[1]<box[3]
                if not visible:continue
                uv=[(v-o)/s for v,o,s in zip(local,point(l['origin']),point(l['pixelStep']))];d=rect(l['sourceRectangle']);tex=[]
                for axis,tile in enumerate([l['tileX'],l['tileY']]):
                    lo,hi=d[axis],d[axis+2];v=uv[axis];period=hi-lo
                    if tile=='clamp':v=max(lo,min(hi-F(1,10**8),v))
                    elif tile=='repeat':v=lo+(v-lo)%period
                    else:
                        z=(v-lo)%(2*period);v=lo+(z if z<=period else 2*period-z);v=min(hi-F(1,10**8),v)
                    if min(v%1,1-v%1)<F(1,10**5):safe=False
                    tex.append(math.floor(v))
                xx,yy=tex;assert 0<=xx<5 and 0<=yy<4
                expected=pixels[(yy*5+xx)*4:(yy*5+xx+1)*4]
            if safe:
                assert actual[(y*64+x)*4:(y*64+x+1)*4]==expected,(c['name'],x,y,list(actual[(y*64+x)*4:(y*64+x+1)*4]),list(expected))
                checked+=1
        assert checked>=300,(c['name'],checked);count+=checked;records.append(dict(name=c['name'],interiorPixels=checked))
    report=dict(format='musteroffice.image-world-interior-pixels/1',runtime=entry(ROOT/'parity.json'),interiorPixels=count,scope='Nearest sampling and composition away from shape/clip/texel boundaries; not arbitrary AA or Office fidelity.',cases=records)
    (ROOT/'pixels-reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(scenes=len(records),interiorPixels=count)))
