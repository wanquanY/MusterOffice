"""Independent end-to-end affine oracle: no matrix composition or Q32 rounding
is reused. Apply every source transform as exact Fractions, then compare actual
float32 device controls against the reported combined error budget.
"""
import hashlib
import json
import struct
from fractions import Fraction as F
from functools import lru_cache
from pathlib import Path
from stroke_reference import verify_styles

root=Path('.codex-work/scene-raster');U=1<<32
counts={'scenes':0,'instances':0,'controlCoordinates':0,'exactTransformApplications':0}
maximum=F(0)
def sha(b):return hashlib.sha256(b).hexdigest()
@lru_cache(maxsize=131072)
def exact(bits):return F(struct.unpack('<f',struct.pack('<I',bits))[0])
@lru_cache(maxsize=131072)
def nearest(value):
    if value==0:return F(0)
    sign=-1 if value<0 else 1;value=abs(value)
    approx=struct.unpack('<I',struct.pack('<f',float(value)))[0]
    best=min(range(max(0,approx-1),approx+2),key=lambda b:(abs(exact(b)-value),b&1))
    return exact(best)*sign
def coords(c):return [(int(c[k]['x']),int(c[k]['y'])) for k in ['control','control1','control2','to'] if k in c]
records=[]
for c in json.loads((root/'parity.json').read_text())['cases']:
    if c['status']!='rendered':continue
    data={k:Path(c[k+'Path']).read_bytes() for k in ['request','response','frame','pixels']}
    for k,b in data.items():assert sha(b)==c[k+'Sha256']
    q=json.loads(data['request']);r=json.loads(data['response'])['info'];s=q['scene'];v=q['viewport']
    w=struct.unpack('<'+'I'*(len(data['frame'])//4),data['frame']);offset=10;paths=[]
    for _ in range(w[5]):
        fill,count=w[offset:offset+2];offset+=2;commands=[]
        for _ in range(count):
            op=w[offset];n={1:1,2:1,3:2,4:3,5:0}[op]
            commands.append((op,[(w[offset+1+2*k],w[offset+2+2*k]) for k in range(n)]));offset+=7
        paths.append((fill,commands))
    offset,paints,stroke_summary=verify_styles(s['instances'],v['scale'],w,offset,r['raster']['work'])
    assert w[6]==len(s['instances']);depths=[]
    for i,t in enumerate(s['transforms']):
        parent=t['parent'];assert parent is None or parent<i
        depths.append(1 if parent is None else depths[parent]+1)
    @lru_cache(maxsize=131072)
    def transformed(x,y,node):
        p=(F(x),F(y))
        while node is not None:
            t=s['transforms'][node];a=[F(int(v),U) for v in t['affine']['linear']];dx=int(t['affine']['translation']['x']);dy=int(t['affine']['translation']['y'])
            p=(a[0]*p[0]+a[1]*p[1]+dx,a[2]*p[0]+a[3]*p[1]+dy);node=t['parent']
        return tuple((p[k]-int(v['origin'][axis]))*F(v['scale']['numerator'],v['scale']['denominator']*U) for k,axis in enumerate(['x','y']))
    max_error=F(0);point_count=0;work=0
    for instance,expected_paint in zip(s['instances'],paints,strict=True):
        index,dx,dy,color,paint,gradient=w[offset:offset+6];offset+=6;assert paint==expected_paint and gradient==0
        source=s['paths'][instance['path']];fill,commands=paths[index]
        assert fill==int(source['fillRule']=='evenodd');assert len(commands)==len(source['commands'])
        assert color==int.from_bytes(bytes(instance['brush']['rgba']),'little')
        depth=1 if instance['transform'] is None else depths[instance['transform']]
        for command,(op,pairs) in zip(source['commands'],commands):
            expected_op={'move':1,'line':2,'quadratic':3,'cubic':4,'close':5}[command['kind']];assert op==expected_op
            points=coords(command);assert len(points)==len(pairs)
            for (x,y),pair in zip(points,pairs):
                expected=transformed(x,y,instance['transform'])
                for k,offset_bits in enumerate([dx,dy]):
                    actual=nearest(exact(pair[k])+exact(offset_bits));error=abs(actual-expected[k])*U
                    max_error=max(max_error,error);counts['controlCoordinates']+=1
                point_count+=1;work+=depth
        counts['instances']+=1
    assert offset==len(w)
    bound=int(r['work']['combinedCoordinateErrorBound'])
    assert max_error<=bound<=int(v['coordinateTolerance']),(c['name'],max_error,bound)
    assert r['work']['evaluatedPoints']==point_count and r['work']['pointTransformWork']==work
    assert r['work']['maximumDepth']==max(depths,default=0)
    assert r['work']['sourceCommands']==sum(len(p['commands']) for p in s['paths'])
    assert r['work']['compiledPaths']==len(paths) and r['work']['compiledCommands']==sum(len(p[1]) for p in paths)
    counts['scenes']+=1;counts['exactTransformApplications']+=work;maximum=max(maximum,max_error)
    records.append({'name':c['name'],'frameSha256':c['frameSha256'],'maximumExactErrorRawQ32Pixels':str(max_error),'reportedBoundRawQ32Pixels':str(bound),'stroke':stroke_summary})
report={'format':'musteroffice.scene-raster-reference/1','counts':counts,'maximumExactErrorRawQ32Pixels':str(maximum),'cases':records,
        'scope':'Exact unrounded source affine chains vs actual float32 device controls; no independent raster coverage or Office/WPS visual acceptance.'}
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(counts))
