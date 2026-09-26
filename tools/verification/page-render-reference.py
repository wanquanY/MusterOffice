"""Independent analytic author geometry vs actual float32 device paths.

The continuous cubic Hermite bound is checked analytically; sampled true arcs
also check placement and device lowering together. Raster coverage is separate.
"""
import hashlib,json,struct
from decimal import Decimal as D
from functools import lru_cache
from pathlib import Path
from page_reference_math import PI,U,cs,resolved,size,dims
from stroke_reference import verify_styles
root=Path('.codex-work/page-render')
def read(p):return json.loads(Path(p).read_text())
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
@lru_cache(None)
def f32(bits):return D.from_float(struct.unpack('<f',struct.pack('<I',bits))[0])
def added(a,b):return D.from_float(struct.unpack('<f',struct.pack('<f',float(f32(a)+f32(b))))[0])
def point(v):return [D(v['x']),D(v['y'])]
def local_geometry(o,n):
    g=o['content']['geometry'];w,h=[v/2 for v in dims(size(o))]
    if g['kind']=='path':
        return [(c['kind'],[[p[0]-w,p[1]-h] for p in [point(c[k]) for k in ['control','control1','control2','to'] if k in c]],None) for c in g['commands']]
    if g['kind']=='rectangle' or g['kind']=='roundRectangle' and int(g['radius'])==0:
        return [('move',[[-w,-h]],None),('line',[[w,-h]],None),('line',[[w,h]],None),('line',[[-w,h]],None),('close',[],None)]
    commands=[]
    def arc(center,radii,start):
        h=PI/(2*n);step=D(5400000)/n
        for j in range(n):
            a=D(start)+j*step;b=a+step;c0,s0=cs(a);c1,s1=cs(b)
            # Source derivatives for parameter t on this segment are h*r*(-sin,cos).
            p0=[center[0]+radii[0]*c0,center[1]+radii[1]*s0]
            p3=[center[0]+radii[0]*c1,center[1]+radii[1]*s1]
            p1=[p0[0]-h*radii[0]*s0/3,p0[1]+h*radii[1]*c0/3]
            p2=[p3[0]+h*radii[0]*s1/3,p3[1]-h*radii[1]*c1/3]
            commands.append(('cubic',[p1,p2,p3],(center,radii,a,step)))
    if g['kind']=='ellipse':
        commands.append(('move',[[w,D(0)]],None))
        for i in range(4):arc([D(0),D(0)],[w,h],i*5400000)
    else:
        r=D(g['radius']);a=w-r;b=h-r
        commands.append(('move',[[a,-h]],None));arc([a,-b],[r,r],16200000)
        commands.append(('line',[[w,b]],None));arc([a,b],[r,r],0)
        commands.append(('line',[[-a,h]],None));arc([-a,b],[r,r],5400000)
        commands.append(('line',[[-w,-b]],None));arc([-a,-b],[r,r],10800000)
    commands.append(('close',[],None));return commands

def expected_order(d,slide):
    s=d['slides'][slide];surfaces=[]
    if s['layout']:
        l=d['layouts'][s['layout']];surfaces+=[d['masters'][l['master']],l]
    surfaces.append(s);out=[]
    for surface in surfaces:
        stack=list(reversed(surface['objects']))
        while stack:
            id=stack.pop();o=d['objects'][id]
            if o['content']['kind']=='group':stack.extend(reversed(o['content']['children']))
            else:
                if o['appearance']['fill']['value']['kind']!='none':out.append((id,'fill'))
                if o['appearance']['stroke']['value']['kind']!='none':out.append((id,'stroke'))
    return out
records=[];counts={'pages':0,'paintInstances':0,'controlCoordinates':0,'trueArcSamples':0,'analyticRemainders':0};maximum=D(0)
for case in read(root/'parity.json')['cases']:
    if case['status']!='rendered':continue
    for k in ['request','plan','response','frame','pixels']:assert sha(case[k+'Path'])==case[k+'Sha256']
    q=read(case['requestPath']);d=q['page']['document'];plan=read(case['planPath'])['plan'];r=read(case['responsePath'])['info'];v=q['viewport']
    assert [(p['object'],p['paint']) for p in plan['paintSources'] if p['object'] is not None]==expected_order(d,q['page']['slide'])
    data=Path(case['framePath']).read_bytes();words=struct.unpack('<'+'I'*(len(data)//4),data);offset=10;paths=[]
    for _ in range(words[5]):
        fill,count=words[offset:offset+2];offset+=2;assert fill==0;commands=[]
        for _ in range(count):
            op=words[offset];n={1:1,2:1,3:2,4:3,5:0}[op]
            commands.append((op,[(words[offset+1+2*k],words[offset+2+2*k]) for k in range(n)]));offset+=7
        paths.append(commands)
    scene=plan['raster']['scene']
    offset,paints,stroke_summary=verify_styles(scene['instances'],v['scale'],words,offset,r['scene']['raster']['work'])
    author_miter_error=D(0)
    for source in plan['paintSources']:
        instance=scene['instances'][source['instance']]
        assert (source['paint']=='stroke')==(instance.get('stroke') is not None)
        if source['paint']=='stroke':
            declared=d['objects'][source['object']]['appearance']['stroke']['value'];evaluated=instance['stroke']
            assert D(evaluated['width'])==D(declared['width'])*U
            assert evaluated['cap']=={'flat':'butt','round':'round','square':'square'}[declared['cap']]
            assert declared['join']['kind']==evaluated['join']['kind']
            if declared['join']['kind']=='miter':
                error=abs(D(evaluated['join']['limit'])-D(declared['join']['limit'])*U/100000)
                assert error<=D('0.5');author_miter_error=max(author_miter_error,error)
    assert author_miter_error<=D(plan['info']['authorMiterLimitErrorBound'])<author_miter_error+1

    scale=D(v['scale']['numerator'])/v['scale']['denominator'];bound=D(r['combinedCoordinateErrorBound'])/U
    assert len(plan['paintSources'])==words[6];largest=D(0)
    for source,expected_paint in zip(plan['paintSources'],paints,strict=True):
        index,dx,dy,color,paint,gradient=words[offset:offset+6];offset+=6;assert paint==expected_paint and gradient==0;actual=paths[index]
        if source['object'] is None:
            w,h=dims(d['pageSize']);expect=[('move',[[D(0),D(0)]],None),('line',[[w,D(0)]],None),('line',[[w,h]],None),('line',[[D(0),h]],None),('close',[],None)]
            center=[D(0),D(0)];lin=[D(1),D(0),D(0),D(1)]
        else:
            o=d['objects'][source['object']];chain=[o];parent=o['parent']
            while parent['kind']=='group':g=d['objects'][parent['id']];chain.insert(0,g);parent=g['parent']
            center,extent,angle=resolved(chain);c,s=cs(angle);lin=[c*extent[0],-s*extent[1],s*extent[0],c*extent[1]]
            n=sum(1 for op,_ in actual if op==4)//4
            expect=local_geometry(o,n);counts['paintInstances']+=1
            g=o['content']['geometry']
            if g['kind']=='ellipse' or g['kind']=='roundRectangle' and int(g['radius'])>0:
                radii=[v/2 for v in dims(size(o))] if g['kind']=='ellipse' else [D(g['radius'])]*2
                remainder=[radius*(PI/(2*n))**4/384 for radius in radii]
                transformed=max(sum(abs(lin[2*row+i])*remainder[i] for i in range(2)) for row in range(2))*scale
                assert transformed<=D(plan['info']['geometryCoordinateErrorBound'])/U+D('1e-90')
                counts['analyticRemainders']+=1
        def transform(p):return [(sum(lin[2*row+i]*p[i] for i in range(2))+center[row])*scale for row in range(2)]
        assert len(expect)==len(actual),(case['name'],source)
        previous=None
        for (kind,controls,arc),(op,bits) in zip(expect,actual,strict=True):
            assert op=={'move':1,'line':2,'quadratic':3,'cubic':4,'close':5}[kind]
            got=[[added(p[0],dx),added(p[1],dy)] for p in bits]
            for p,a in zip(controls,got,strict=True):
                expected=transform(p)
                for i in range(2):largest=max(largest,abs(a[i]-expected[i]));counts['controlCoordinates']+=1
            if arc:
                ctr,radii,start,step=arc;assert previous is not None
                for t in [D(0),D('0.125'),D('0.25'),D('0.5'),D('0.75'),D('0.875'),D(1)]:
                    c,s=cs(start+step*t);exact=transform([ctr[0]+radii[0]*c,ctr[1]+radii[1]*s]);u=1-t
                    actual_point=[u**3*previous[i]+3*u*u*t*got[0][i]+3*u*t*t*got[1][i]+t**3*got[2][i] for i in range(2)]
                    largest=max(largest,max(abs(actual_point[i]-exact[i]) for i in range(2)));counts['trueArcSamples']+=1
            if got:previous=got[-1]
    assert offset==len(words)
    assert largest<=bound+D('1e-90') and bound<=D(v['coordinateTolerance'])/U,(case['name'],largest,bound)
    maximum=max(maximum,largest);counts['pages']+=1
    records.append({'name':case['name'],'planSha256':case['planSha256'],'frameSha256':case['frameSha256'],'maximumObservedCoordinateDeviationPixels':str(largest),'reportedBoundPixels':str(bound),'stroke':stroke_summary,'authorMiterExactErrorRawQ32':str(author_miter_error)})
report={'format':'musteroffice.page-render-reference/1','counts':counts,'maximumObservedCoordinateDeviationPixels':str(maximum),'arithmetic':'Decimal 140 digits, Chudnovsky pi, direct source transforms and analytic curves; actual IEEE binary32 paths and translations; reference guard 1e-90','cases':records,'scope':'Coordinates and continuous Hermite remainder, not antialias coverage, full PPT fidelity or Office acceptance.'}
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(counts))
