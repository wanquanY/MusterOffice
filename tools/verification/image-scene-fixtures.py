"""Equivalent original image scenes with independently inverted affine graphs."""
from copy import deepcopy
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path

ROOT = Path('.codex-work/image-scene/cases');ROOT.mkdir(parents=True,exist_ok=True)
PREVIOUS = Path('docs/reviews/evidence/2026-09-25-image-brush-completed-verification.json')
assert hashlib.sha256(PREVIOUS.read_bytes()).hexdigest() == '4f407ae1696efeb49337e113f0a399576f33f44f5994bc621733883f0c86c31a'
old = json.loads(PREVIOUS.read_text())['reports']['parity']['evidence']['cases']
U=1<<32
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def read(r):
    assert entry(r['path'])==r;return json.loads(Path(r['path']).read_text())
def point(x,y):
    assert F(x).denominator==F(y).denominator==1
    return dict(x=str(int(x)),y=str(int(y)))
def inverse(p,m,t):
    a,b,c,d=m;x,y=F(int(p['x'])-t[0]),F(int(p['y'])-t[1]);det=a*d-b*c
    return point((d*x-b*y)/det,(-c*x+a*y)/det)
def transformed(source,variant):
    q=deepcopy(source);r=q['raster'];delta=(1<<100,-(1<<100)) if variant=='huge' else (0,0)
    outer={'nested':(0,-1,1,0),'huge':(0,-1,1,0),'scaled':(2,0,0,4),'reflected':(-1,1,0,1)}[variant]
    inner=(1,1,0,1);ot=(7*U,-3*U);it=(-2*U,5*U)
    def inv(p):return inverse(inverse(p,outer,ot),inner,it)
    paths=deepcopy(r['paths'])
    for path in paths:
        for cmd in path['commands']:
            for k in ['to','control','control1','control2']:
                if k in cmd:cmd[k]=inv(cmd[k])
    transforms=[];instances=[]
    for draw in r['draws']:
        origin=draw['origin'];root=len(transforms)
        transforms.extend([
            dict(parent=None,affine=dict(linear=[str(v*U) for v in outer],translation=point(int(origin['x'])+ot[0]+delta[0],int(origin['y'])+ot[1]+delta[1]))),
            dict(parent=root,affine=dict(linear=[str(v*U) for v in inner],translation=point(*it))),
        ])
        brush=deepcopy(draw['brush'])
        points=[]
        if brush['kind']=='image':points=[brush['image']['origin']]
        elif brush['kind']=='gradient':
            geometry=brush['gradient']['geometry'];points=[geometry[k] for k in ['start','end','center'] if k in geometry]
        for p in points:p.update(point(int(p['x'])+delta[0],int(p['y'])+delta[1]))
        instances.append(dict(path=draw['path'],transform=root+1,brush=brush,**({'stroke':draw['stroke']} if 'stroke' in draw else {})))
    origin=r['viewport']['origin'];origin.update(point(int(origin['x'])+delta[0],int(origin['y'])+delta[1]))
    q['raster']=dict(viewport=r['viewport'],scene=dict(paths=paths,transforms=transforms,instances=instances))
    return q
def save(name,q,c,success):
    file=ROOT/(name+'.json');file.write_text(json.dumps(q))
    r=dict(name=name,request=entry(file),images=c['images'],originalRequest=c['request'],success=success)
    if success and c.get('pixels'):r['expectedPixels']=c['pixels']
    return r
cases=[]
base=next(c for c in old if c['name']=='premultiplied-nearest-clamp')
mixed=read(base['request']);draw=mixed['raster']['draws'][0]
under=deepcopy(draw);under['brush']=dict(kind='solid',rgba=[20,40,80,200])
over=deepcopy(draw);over['brush']=dict(kind='solid',rgba=[30,100,10,80])
mixed['raster']['draws']=[under,draw,over]
mixed_path=ROOT/'mixed-paints-world.json';mixed_path.write_text(json.dumps(mixed))
old.append(dict(name='mixed-paints',request=entry(mixed_path),images=base['images'],success=True))
for c in old:
    source=read(c['request'])
    success=c.get('success','pixels' in c)
    variants=['nested','scaled','reflected','huge'] if success else ['nested']
    for variant in variants:
        q=transformed(source,variant)
        cases.append(save(c['name']+'-'+variant,q,c,success))
base=next(c for c in old if c['name']=='premultiplied-nearest-clamp')
for name in ['path-reference','transform-reference','parent-order','depth','path-order','huge-paint-origin','scene-instances']:
    q=transformed(read(base['request']),'nested');scene=q['raster']['scene']
    if name=='path-reference':scene['instances'][0]['path']=100
    if name=='transform-reference':scene['instances'][0]['transform']=100
    if name=='parent-order':scene['transforms'][0]['parent']=0
    if name=='depth':
        for i in range(2,65):scene['transforms'].append(dict(parent=i-1,affine=dict(linear=[str(U),'0','0',str(U)],translation=point(0,0))))
        scene['instances'][0]['transform']=64
    if name=='path-order':scene['paths'][0]['commands'][0]['kind']='line'
    if name=='huge-paint-origin':scene['instances'][0]['brush']['image']['origin']=point(1<<120,0)
    if name=='scene-instances':scene['instances']*=65537
    cases.append(save('invalid-scene-'+name,q,base,False))
manifest=dict(format='musteroffice.image-scene-fixtures/1',previousEvidence=entry(PREVIOUS),cases=cases)
(ROOT/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(dict(success=124,failures=16,cases=len(cases))))
