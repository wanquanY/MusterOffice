"""Independent Decimal oracle for chOff/zero-axis and native group placement.

Consumes fixture author frames, not the returned resolved transforms or Rust
intervals. Reconstructs each ancestor chain independently at 140-digit precision.
"""
import hashlib,json
from decimal import Decimal as D
from pathlib import Path
from page_reference_math import cs,U
ROOT=Path('.codex-work/source-placement')
read=lambda p:json.loads(Path(p).read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
counts={'objects':0,'coefficients':0,'controlPoints':0};records=[]
def expected(chain,last_group):
    center=[D(0),D(0)];scale=[D(1),D(1)];anchor=[D(0),D(0)];angle=0;flips=[False,False]
    for n,f in enumerate(chain):
        f=f or {};group=n<len(chain)-1 or last_group
        dst=list(map(D,f.get('ext',[0,0])));src=list(map(D,f.get('chExt',[0,0]))) if group else dst
        if group:src=[src[i] or dst[i] for i in range(2)]
        off=list(map(D,f.get('off',[0,0])));child=list(map(D,f.get('chOff',[0,0]))) if group else [D(0),D(0)]
        owncenter=[off[i]+dst[i]/2 for i in range(2)];rotation=f.get('rot',0)
        if n:
            delta=[(owncenter[i]-anchor[i])*scale[i]*(-1 if flips[i] else 1) for i in range(2)];c,s=cs(angle)
            center=[center[0]+c*delta[0]-s*delta[1],center[1]+s*delta[0]+c*delta[1]]
            sector=((rotation%21600000)+2700000)//5400000
            if sector%2:scale.reverse()
            angle+=(-1 if flips[0]!=flips[1] else 1)*rotation
        else:center=owncenter;angle=rotation
        scale=[scale[i]*(dst[i]/src[i] if src[i] else 1) for i in range(2)]
        flips=[flips[i]!=bool(f.get(k,0)) for i,k in enumerate(['flipH','flipV'])]
        anchor=[child[i]+src[i]/2 for i in range(2)]
    c,s=cs(angle);signed=[scale[i]*(-1 if flips[i] else 1) for i in range(2)]
    return anchor,[c*signed[0],-s*signed[1],s*signed[0],c*signed[1],*center],src,child
for case in read(ROOT/'manifest.json')['cases']:
    if case['error'] or case['unresolved']:continue
    r=read(ROOT/(case['name']+'.placement.response.json'))['placements']
    for got,want in zip(r['objects'],case['expected'],strict=True):
        p=got['outcome']['placement'];anchor,values,src,child=expected(want['frames'],want.get('group',False))
        assert got['nativeId']==want['id'] and got['depth']==len(want['frames'])-1
        assert [D(p['anchor'][a])/U for a in ['x','y']]==anchor,(case['name'],p['anchor'],anchor)
        assert [D(p['sourceSize'][a]) for a in ['width','height']]==src
        assert [D(p['sourceOrigin'][a]) for a in ['x','y']]==child
        actual=[D(v)/U for v in p['affine']['linear']]+[D(p['affine']['translation'][a])/U for a in ['x','y']]
        errors=[D(v)/U for v in p['uncertainty']['linear']]+[D(p['uncertainty']['translation'][a])/U for a in ['x','y']]
        for a,b,e in zip(actual,values,errors,strict=True):assert e>=0 and abs(a-b)<=e+D('1e-90'),(case['name'],want['id'],a,b,e)
        points=[child,[child[0]+src[0],child[1]],[child[0],child[1]+src[1]],[child[i]+src[i] for i in range(2)],anchor]
        for point in points:
            delta=[point[i]-anchor[i] for i in range(2)]
            for row in range(2):
                gotpoint=sum(actual[2*row+i]*delta[i] for i in range(2))+actual[4+row]
                refpoint=sum(values[2*row+i]*delta[i] for i in range(2))+values[4+row]
                bound=sum(errors[2*row+i]*abs(delta[i]) for i in range(2))+errors[4+row]
                assert abs(gotpoint-refpoint)<=bound+D('1e-85')
        counts['objects']+=1;counts['coefficients']+=6;counts['controlPoints']+=len(points)
    records.append({'name':case['name'],'responseSha256':sha(ROOT/(case['name']+'.placement.response.json'))})
report={'format':'musteroffice.source-placement-reference/1','arithmetic':'Decimal140; independent author frame chains, Chudnovsky pi; 1e-85 reference guard','counts':counts,'cases':records}
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(counts))
