"""Exact rational geometry perturbations, independent of the interval verifier."""
import hashlib,json,struct,itertools
from pathlib import Path
from fractions import Fraction as F
root=Path('.codex-work/elliptic-source');Q=1<<32
entry=lambda p:dict(path=str(p),byteLength=Path(p).stat().st_size,sha256=hashlib.sha256(Path(p).read_bytes()).hexdigest())
report=json.loads((root/'runtime.json').read_text());records=[];total=0
for record in report['cases']:
    if not record['name'].startswith('parameter-'):continue
    q=json.loads(Path(record['request']['path']).read_text());meta=json.loads(Path(record['response']['path']).read_text())
    work=meta['info']['work']['ellipticGradients'];bound=F(work['coordinateErrorBound'])/Q
    assert work['encodedRootIntervalBound']=='512'
    g=q['draws'][0]['brush']['gradient']['geometry'];p=g['plane'];v=g['field'];u=p['uncertainty']
    values=[F(x)/Q for key in ['tileScale','innerCenter','innerRadii'] for x in v[key]]
    errors=[F(x)/Q for x in v['uncertainty']]
    matrix=[F(p[k][a])/Q for k,a in [('xStep','x'),('yStep','x'),('origin','x'),('xStep','y'),('yStep','y'),('origin','y')]]
    me=[F(u[k][a])/Q for k,a in [('xStep','x'),('yStep','x'),('origin','x'),('xStep','y'),('yStep','y'),('origin','y')]]
    raw=Path(record['frame']['path']).read_bytes();words=struct.unpack('<'+'I'*(len(raw)//4),raw)
    assert words[1]==12 and words[51]==4
    fp=lambda w:F(struct.unpack('<f',struct.pack('<I',w))[0])
    encoded=list(map(fp,words[64:70]));em=list(map(fp,words[56:62]))
    for i in range(6):assert abs(values[i]-encoded[i])+errors[i]<=F(work['parameterErrorBounds'][i])/Q
    def position(v,m,t,c,s):
        unit=[(1+((1-t)*v[i+2]+((1-t)*v[i+4]+t)*angle)/v[i])/2 for i,angle in enumerate([c,s])]
        return [m[r]*unit[0]+m[r+1]*unit[1]+m[r+2] for r in [0,3]]
    samples=0;maximum=F(0)
    angles=[(F(1),F(0)),(F(0),F(1)),(F(-1),F(0)),(F(0),F(-1)),(F(3,5),F(4,5)),(F(-3,5),F(4,5)),(F(3,5),F(-4,5)),(F(-3,5),F(-4,5))]
    # All 64 corners of six independent field uncertainties, with both matrix
    # box extremes and their opposites. Axis and rational-circle boundary points.
    for signs in itertools.product([-1,1],repeat=6):
        value=[a+s*e for a,e,s in zip(values,errors,signs)]
        for opposite in [-1,1]:
            transform=[a+opposite*s*e for a,e,s in zip(matrix,me,signs)]
            for t in [F(0),F(1,2),F(1)]:
                for c,s in angles:
                    a=position(value,transform,t,c,s);b=position(encoded,em,t,c,s)
                    error=max(abs(x-y) for x,y in zip(a,b));assert error<=bound,(record['name'],error,bound)
                    maximum=max(maximum,error);samples+=1
    total+=samples;records.append(dict(name=record['name'],request=record['request'],response=record['response'],frame=record['frame'],samples=samples,largestSampledDeviceError=float(maximum),reportedBound=float(bound)))
assert len(records)==8
out=dict(format='musteroffice.elliptic-parameter-reference/1',source=entry(root/'runtime.json'),cases=records,comparisons=total,scope='Exact-rational corresponding boundary points across uncertainty corners at t=0,1/2,1, including plane coefficient cross terms. These sampled geometry checks are not a scalar/color or coverage error claim.')
(root/'parameter-reference.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(dict(cases=len(records),comparisons=total)))
