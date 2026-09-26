"""Compare actual pixels against exact first-root topology and Decimal colors."""
import hashlib,json,math,struct,zlib
from pathlib import Path
from decimal import Decimal as D,localcontext
from fractions import Fraction as F
from elliptic_polynomial_reference import inside,poly,sturm,at,variations
root=Path('.codex-work/elliptic-render')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def f32(v):return struct.unpack('<f',struct.pack('<f',v))[0]
def write_png(path,width,height,data):
    def chunk(kind,payload):return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload))
    rows=b''.join(b'\0'+data[y*width*4:(y+1)*width*4] for y in range(height))
    Path(path).write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b''))
def first(values):
    if inside(values):return 0.,0
    p=poly(values);seq=sturm(p);start=variations(seq,F(0));count=start-variations(seq,F(1))
    if not count:return 1.,0
    a,b=F(0),F(1)
    for _ in range(32):
        mid=(a+b)/2;left=start-variations(seq,mid)
        if left:
            if left==1 and at(p,mid)==0:return float(mid),count
            b=mid
        else:a=mid
    return float((a+b)/2),count
def tiled(x,mode):
    if mode==1:x=f32(x-math.floor(x))
    if mode==2:
        x=f32(x-f32(math.floor(f32(x*.5))*2));x=min(x,f32(2-x))
    return min(max(x,0),1)
report=json.loads((root/'components.json').read_text());results=[];excluded=[]
for c in report['cases']:
    if c['expectedStatus']:continue
    o=c['options'];w=o.get('width',128);h=o.get('height',64)
    assert entry(c['pixels']['path'])==c['pixels'];data=Path(c['pixels']['path']).read_bytes()
    png=root/'cases'/(c['name']+'.png');write_png(png,w,h,data)
    if o.get('office'):
        excluded.append(dict(name=c['name'],reason='Office gamma was verified previously; this independent reference covers the ordinary three-stop ramp.',png=entry(png)));continue
    a,b,tx,cx,d,ty=map(f32,o.get('matrix',[128,0,0,0,64,0]));det=a*d-b*cx
    inv=list(map(f32,[d/det,-b/det,(b*ty-d*tx)/det,-cx/det,a/det,(cx*tx-a*ty)/det]))
    scale=list(map(f32,o.get('scale',[.8,.6])));field=list(map(f32,o.get('field',[0,0,0,0])));tile=o.get('tile',[0,0])
    # Direct Decimal circle reference for complete images; exact Sturm topology
    # for sampled arbitrary ellipses. The case-independent oracle does not use
    # the production Bernstein coefficients, candidate or prepared-field code.
    circle=field[0]==field[1]==0 and field[2]==field[3]
    step=1 if circle else 4;checked=0;maximum=0;total=0;multiple=0
    for y in range(0,h,step):
        for x in range(0,w,step):
            u=f32(f32(f32(inv[0]*(x+.5))+f32(inv[1]*(y+.5)))+inv[2])
            v=f32(f32(f32(inv[3]*(x+.5))+f32(inv[4]*(y+.5)))+inv[5])
            q=[f32(f32(f32(tiled(k,m)*2)-1)*s) for k,m,s in zip([u,v],tile,scale)]
            if circle:
                with localcontext() as ctx:
                    ctx.prec=90;r=D.from_float(field[2]);distance=sum(D.from_float(n)**2 for n in q).sqrt()
                    t=D(0) if distance<=r else D(1) if r>=1 else min(D(1),max(D(0),(distance-r)/(1-r)))
                    scalar=float(t)
            else:
                scalar,count=first([*q,*field]);multiple+=count>1
            expected=32+192*scalar;i=(y*w+x)*4;pixel=data[i:i+4]
            assert pixel[3]==255 and pixel[0]==pixel[1]==pixel[2],(c['name'],x,y,pixel)
            error=abs(pixel[0]-expected);maximum=max(maximum,error);total+=error;checked+=1
            assert error<=1.01,(c['name'],x,y,q,field,scalar,pixel,error)
    results.append(dict(name=c['name'],pixels=checked,samplingStep=step,maxGrayError=maximum,meanGrayError=total/checked,
                        multipleRootPixels=multiple,png=entry(png)))
    print(json.dumps(results[-1]),flush=True)
result=dict(format='musteroffice.elliptic-render-reference/1',source=entry(root/'components.json'),cases=results,excluded=excluded,
    pixels=sum(v['pixels'] for v in results),multipleRootPixels=sum(v['multipleRootPixels'] for v in results),
    note='90-digit Decimal centered circles and exact rational Sturm first-root topology for sampled general pixels; <=1.01 gray steps against unrounded ideal ramp. Not Office/WPS acceptance.')
(root/'reference.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(pixels=result['pixels'],multipleRootPixels=result['multipleRootPixels'])))
