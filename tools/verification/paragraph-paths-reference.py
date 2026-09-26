"""Independent Fraction placement and 120-digit analytic Bezier extrema.
Bounds are compared to the supplied Q32 paths, not raster/effect ink bounds.
"""
from pathlib import Path
from decimal import Decimal as D, localcontext
from fractions import Fraction as F
import hashlib,json
from line_geometry_math import reference
root=Path('.codex-work/paragraph-paths');U=1<<32;eps=D('1e-80')
count={'layouts':0,'glyphs':0,'paths':0,'segments':0,'coordinates':0};maximum=D(0)
def point(p):return [D(p[k]) for k in ['x','y']]
def unite(a,b):
    if a is None:return b
    if b is None:return a
    return [min(a[0],b[0]),min(a[1],b[1]),max(a[2],b[2]),max(a[3],b[3])]
def curve(points):
    count['segments']+=1
    values=[[],[]]
    for k in range(2):
        p=[v[k] for v in points];ts=[D(0),D(1)]
        if len(p)==3:
            a=p[0]-2*p[1]+p[2]
            if a:ts.append((p[0]-p[1])/a)
        elif len(p)==4:
            a=3*(-p[0]+3*p[1]-3*p[2]+p[3]);b=2*(3*p[0]-6*p[1]+3*p[2]);c=3*(p[1]-p[0])
            if a:
                discriminant=b*b-4*a*c
                if discriminant>=0:
                    d=discriminant.sqrt();ts.extend([(-b-d)/(2*a),(-b+d)/(2*a)])
            elif b:ts.append(-c/b)
        for t in ts:
            if 0<=t<=1:
                # Independent high-precision polynomial evaluation.
                if len(p)==2:y=(1-t)*p[0]+t*p[1]
                elif len(p)==3:y=(1-t)**2*p[0]+2*(1-t)*t*p[1]+t*t*p[2]
                else:y=(1-t)**3*p[0]+3*(1-t)**2*t*p[1]+3*(1-t)*t*t*p[2]+t**3*p[3]
                values[k].append(y)
    return [min(values[0]),min(values[1]),max(values[0]),max(values[1])]
def path_bounds(commands):
    current=start=None;b=None
    for c in commands:
        if c['kind']=='move':current=start=point(c['to']);continue
        if c['kind']=='close':points=[current,start]
        else:
            points=[current]
            for name in ['control','control1','control2']:
                if name in c:points.append(point(c[name]))
            points.append(point(c['to']))
        b=unite(b,curve(points));current=points[-1]
    return b
def verify(actual,expected,tolerance):
    global maximum
    if expected is None:assert actual is None;return
    a=point(actual['min'])+point(actual['max'])
    excess=[expected[0]-a[0],expected[1]-a[1],a[2]-expected[2],a[3]-expected[3]]
    assert all(-eps<=v<=tolerance+eps for v in excess),(a,expected,tolerance,excess)
    maximum=max(maximum,*excess);count['coordinates']+=4
cases=[]
with localcontext() as ctx:
    ctx.prec=120
    for case in json.loads((root/'parity.json').read_text())['cases']:
        files={k:Path(case[k+'Path']).read_bytes() for k in ['request','response']}
        for k,v in files.items():assert hashlib.sha256(v).hexdigest()==case[k+'Sha256']
        r=json.loads(files['response'])
        if r['status']!='evaluated' or r['result']['scene'] is None:continue
        q=json.loads(files['request']);r=r['result'];scene=r['scene'];tolerance=D(q['boundsTolerance'])
        layout=q['layout'];gq={'shaping':{'paragraph':layout['paragraph']},'styles':layout['styles'],'strutStyle':layout['strutStyle'],'spacing':layout['spacing']}
        fixed=reference(gq,r['layout']['geometry'],True);positions=[]
        for line,l in enumerate(fixed['lines']):positions.extend(dict(g,line=line) for g in l['glyphs'])
        assert len(positions)==len(scene['glyphs']);exact_paths=[]
        for p in scene['paths']:
            b=path_bounds(p['commands']);verify(p['bounds'],b,tolerance);exact_paths.append(b);count['paths']+=1
        all_bounds=None
        for actual,expected in zip(scene['glyphs'],positions):
            assert actual['line']==expected['line'] and actual['source']==expected['source'] and actual['glyph']==expected['glyph']
            for k in ['x','y']:assert F(int(actual['origin'][k]),U)==expected[k]
            b=exact_paths[actual['path']]
            if b is not None:
                dx,dy=point(actual['origin']);all_bounds=unite(all_bounds,[b[0]+dx,b[1]+dy,b[2]+dx,b[3]+dy])
            count['glyphs']+=1;count['coordinates']+=2
        verify(scene['bounds'],all_bounds,tolerance);count['layouts']+=1
        cases.append({'name':case['name'],'requestSha256':case['requestSha256'],'responseSha256':case['responseSha256'],'glyphs':len(positions),'paths':len(exact_paths)})
report={'format':'musteroffice.paragraph-paths-reference/1',**count,'maximumObservedExcessRawQ32':str(maximum),'analyticPrecisionDigits':120,'comparisonEpsilonRawQ32':'1e-80','cases':cases,'scope':'Independent Fraction line placement and analytic polynomial extrema of emitted quantized paths; no independent shaping, rasterization, fill/stroke/effect ink or target app visual acceptance.'}
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(count))
