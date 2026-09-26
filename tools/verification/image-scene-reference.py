"""Check scene graphs with exact arithmetic and pixels with a separate sampler."""
from fractions import Fraction as F
import json
from pathlib import Path
from jsonschema import Draft202012Validator
from image_reference import point, premul, sample, entry

ROOT=Path('.codex-work/image-scene')
report=json.loads((ROOT/'parity.json').read_text())
def read(record):
    assert entry(Path(record['path']))==record;return Path(record['path']).read_bytes()
def mapped(p,node):
    a,b,c,d=[F(int(v),1<<32) for v in node['affine']['linear']]
    x,y=p;tx,ty=point(node['affine']['translation'])
    return [a*x+b*y+tx,c*x+d*y+ty]
def world(p,index,nodes):
    result=point(p);hops=0
    while index is not None:
        hops+=1;assert hops<=64
        node=nodes[index];result=mapped(result,node);index=node['parent']
    return result
schemas={n:Draft202012Validator(json.loads(Path('contracts/generated/image-scene-'+n+'.schema.json').read_text())) for n in ['request','response']}
cases=[];vertices=0;compared=0
for c in report['cases']:
    q=json.loads(read(c['request']));response=json.loads(read(c['response']))
    schemas['request'].validate(q);schemas['response'].validate(response)
    if not c['success']:continue
    raw=json.loads(read(c['originalRequest']));scene=q['raster']['scene'];v=q['raster']['viewport']
    for key in ['width','height','scale','coordinateTolerance','background']:assert v[key]==raw['raster']['viewport'][key]
    vx,vy=point(v['origin']);ox,oy=point(raw['raster']['viewport']['origin']);delta=[vx-ox,vy-oy]
    boxes=[]
    assert len(scene['instances'])==len(raw['raster']['draws'])
    for instance,draw in zip(scene['instances'],raw['raster']['draws']):
        commands=scene['paths'][instance['path']]['commands'];original=raw['raster']['paths'][draw['path']]['commands']
        assert len(commands)==len(original)==5
        points=[];dx,dy=point(draw['origin'])
        for cmd,expected in zip(commands,original):
            assert cmd['kind']==expected['kind']
            if cmd['kind']=='close':continue
            assert cmd['kind'] in ['move','line']
            actual=world(cmd['to'],instance['transform'],scene['transforms']);x,y=point(expected['to'])
            assert actual==[x+dx+delta[0],y+dy+delta[1]],c['name'];points.append(actual);vertices+=1
        x0,x1=min(p[0] for p in points),max(p[0] for p in points)
        y0,y1=min(p[1] for p in points),max(p[1] for p in points)
        assert set(map(tuple,points))=={(x0,y0),(x1,y0),(x1,y1),(x0,y1)}
        for x,y in points:assert ((x-vx)*F(v['scale']['numerator'],v['scale']['denominator'])).denominator==((y-vy)*F(v['scale']['numerator'],v['scale']['denominator'])).denominator==1
        boxes.append((x0,y0,x1,y1))
        paint=instance['brush'];prior=draw['brush'];assert paint['kind']==prior['kind']
        if paint['kind']=='image':
            for key in ['resource','xStep','yStep','tileX','tileY','sampling']:assert paint['image'][key]==prior['image'][key]
            assert point(paint['image']['origin'])==[p+d for p,d in zip(point(prior['image']['origin']),delta)]
        elif paint['kind']=='solid':assert paint==prior
    pixels=read(c['pixels']);data=read(c['images']);resources=[];offset=0
    for resource in q['images']:
        size=resource['width']*resource['height']*4;resources.append(data[offset:offset+size]);offset+=size
    assert offset==len(data)
    scale=F(v['scale']['numerator'],v['scale']['denominator']);count=0
    for py in range(v['height']):
        for px in range(v['width']):
            dest=premul(v['background'],'straight');x,y=vx+F(2*px+1,2)/scale,vy+F(2*py+1,2)/scale
            for instance,(x0,y0,x1,y1) in zip(scene['instances'],boxes):
                if not (x0<=x<x1 and y0<=y<y1):continue
                paint=instance['brush']
                if paint['kind']=='gradient':
                    assert all(s['srgb']==[0,0,0,0] for s in paint['gradient']['stops']);continue
                if paint['kind']=='solid':source=premul(paint['rgba'],'straight')
                else:
                    b=paint['image'];a,c1=point(b['xStep']);b1,d=point(b['yStep']);tx,ty=point(b['origin']);det=a*d-b1*c1
                    sx,sy=((x-tx)*d-(y-ty)*b1)/det,(a*(y-ty)-c1*(x-tx))/det
                    source=sample(q['images'][b['resource']],resources[b['resource']],b,sx,sy)
                dest=[source[k]+dest[k]*(1-source[3]/255) for k in range(4)]
            start=4*(py*v['width']+px);expected=[max(0,min(255,round(z))) for z in dest]
            assert max(abs(a-b) for a,b in zip(pixels[start:start+4],expected))<=1,(c['name'],px,py)
            count+=1
    assert int(response['info']['scene']['work']['combinedCoordinateErrorBound'])<=int(v['coordinateTolerance'])
    assert int(response['info']['images']['coordinateErrorBound'])<=int(v['coordinateTolerance'])
    cases.append(dict(name=c['name'],comparedPixels=count));compared+=count
for c in report['invalid']:
    schemas['response'].validate(json.loads(read(c['response'])))
    if not c['duplicateMember']:assert not schemas['request'].is_valid(json.loads(read(c['request'])))
counts=dict(cases=len(cases),exactWorldVertices=vertices,comparedPixels=compared,channelTolerance=1)
(ROOT/'reference.json').write_text(json.dumps(dict(format='musteroffice.image-scene-reference/1',counts=counts,cases=cases),indent=2)+'\n')
print(json.dumps(counts))
