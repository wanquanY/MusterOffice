"""80-digit independent path-space, polar ellipse and cubic error checks.

Guide outputs are exact binary64 inputs to this stage, not exact source math.
"""
import hashlib,json,zipfile,sys
from decimal import Decimal as D, ROUND_FLOOR
from pathlib import Path
from lxml import etree as E
from guide_decimal import PI,TURN,trig,angle
from mce_reference import project
PRESETS='--presets' in sys.argv
ROOT=Path('.codex-work/preset-expansion' if PRESETS else '.codex-work/native-paths');Q=D(2)**32;EPS=D('1e-40')
report=json.loads((ROOT/'parity.json').read_text());manifest={c['name']:c for c in json.loads((ROOT/'manifest.json').read_text())['cases']}
if PRESETS:
    evaluations={c['name']:c for c in report['cases'] if c['kind']=='geometry'}
    report={'cases':[{**c,'evaluationPath':evaluations[c['name']]['responsePath'],'evaluationSha256':evaluations[c['name']]['responseSha256']} for c in report['cases'] if c['kind']=='paths']}
sha=lambda b:hashlib.sha256(b).hexdigest()
def number(v):return D(v) if isinstance(v,int) else D.from_float(v)
def point(p):return [number(p['x']),number(p['y'])]
def add(a,b):return [a[i]+b[i] for i in range(2)]
def sub(a,b):return [a[i]-b[i] for i in range(2)]
def fixed(p):return [D(p[axis])/Q for axis in ['x','y']]
def parameter(theta,rx,ry):
    turns=(theta/TURN).to_integral_value(rounding=ROUND_FLOOR);phase=theta-turns*TURN
    c,s=trig(phase);value=angle(ry*c,rx*s)
    if value<0:value+=TURN
    return (turns*TURN+value)*2*PI/TURN
def radial(theta,rx,ry):
    c,s=trig(theta);denominator=((ry*c)**2+(rx*s)**2).sqrt()
    return [rx*ry*c/denominator,rx*ry*s/denominator]
def unit(radians):return trig(radians*TURN/(2*PI))
points=segments=samples=paths=compiled=unresolved=0;largest=D(0);checks=[]
def compare(actual,expected,scale,bounds,name):
    global points,largest
    values=fixed(actual)
    for axis in range(2):
        difference=abs(values[axis]-expected[axis]*scale[axis]);largest=max(largest,difference)
        assert difference<=bounds[axis]+EPS,(name,axis,str(difference),str(bounds[axis]))
    points+=1
for record in report['cases']:
    if record['name'] not in manifest:continue
    c=manifest[record['name']];raw=Path(record['responsePath']).read_bytes();evaluated=Path(record['evaluationPath']).read_bytes()
    assert sha(raw)==record['responseSha256'] and sha(evaluated)==record['evaluationSha256']
    assert sha(Path(c['path']).read_bytes())==c['sha256']==record['sourceSha256']
    result=json.loads(raw)['paths']['objects'][0]['outcome'];source=json.loads(evaluated)['geometry']['objects'][0]['outcome']
    if source['status']=='unresolved':
        assert result['status']=='unresolvedGeometry' and result['reason']==source['reason'];checks.append({'name':c['name'],'sourceSha256':c['sha256'],'responseSha256':record['responseSha256'],'geometryUnresolved':source['reason']});unresolved+=1;continue
    if result['status']!='compiled':
        assert result['status']=='unresolvedPath';checks.append({'name':c['name'],'issue':result['issue'],'sourceSha256':c['sha256'],'responseSha256':record['responseSha256']});unresolved+=1;continue
    g=source['geometry'];extent=g['extent']['value'];assert result['extent']==g['extent'];assert len(result['paths'])==len(g['paths'])
    for p,original in zip(result['paths'],g['paths']):
        name=c['name']+'/'+str(p['origin']);assert p['origin']==original['origin']
        for key in ['fill','stroke','extrusionOk']:assert p[key]==original[key]
        scale=[D(extent[axis])/D(original[axis]) if original[axis] is not None else D(1) for axis in ['width','height']]
        numeric=fixed(p['numericErrorBound']);curve=fixed(p['curveErrorBound']);combined=fixed(p['coordinateErrorBound'])
        assert combined==add(numeric,curve) and min(numeric+curve)>=0
        assert max(combined)<=D(c.get('tolerance','4294967296'))/Q
        current=[D(0),D(0)];start=current[:];needs_move=True;cursor=0;arc_segments=0
        assert len(p['sourceMap'])==len(original['commands'])
        for cmd,span in zip(original['commands'],p['sourceMap']):
            assert span['origin']==cmd['origin'] and span['firstCommand']==cursor
            kind=cmd['kind']
            if needs_move and kind!='move':
                actual=p['commands'][cursor];assert actual['kind']=='move';compare(actual['to'],current,scale,numeric,name);cursor+=1;start=current[:];needs_move=False
            if kind in ['move','line','quadratic','cubic']:
                actual=p['commands'][cursor];assert actual['kind']==kind
                for key in ['to','control','control1','control2']:
                    if key in cmd:compare(actual[key],point(cmd[key]),scale,numeric,name)
                current=point(cmd['to']);cursor+=1
                if kind=='move':start=current[:];needs_move=False
            elif kind=='close':
                assert p['commands'][cursor]['kind']=='close';cursor+=1;current=start[:];needs_move=True
            elif kind=='arc':
                rx,ry,theta,sweep=[number(cmd[k]) for k in ['widthRadius','heightRadius','startAngle','sweepAngle']]
                if sweep!=0 and (rx!=0 or ry!=0):
                    count=span['firstCommand']+span['commandCount']-cursor;assert count>0 and count&(count-1)==0
                    assert rx>0 and ry>0
                    a=parameter(theta,rx,ry);b=parameter(theta+sweep,rx,ry);h=(b-a)/count
                    center=sub(current,radial(theta,rx,ry));remainder=[rx*scale[0]*h**4/384,ry*scale[1]*h**4/384]
                    assert all(remainder[i]<=curve[i]+EPS for i in range(2)),name
                    for j in range(count):
                        c0,s0=unit(a+j*h);c1,s1=unit(a+(j+1)*h)
                        p0=add(center,[rx*c0,ry*s0]);p3=add(center,[rx*c1,ry*s1])
                        controls=[add(center,[rx*(c0-h*s0/3),ry*(s0+h*c0/3)]),add(center,[rx*(c1+h*s1/3),ry*(s1-h*c1/3)])]
                        actual=p['commands'][cursor];assert actual['kind']=='cubic'
                        for key,value in zip(['control1','control2','to'],[*controls,p3]):compare(actual[key],value,scale,numeric,name)
                        # Independent samples of the output cubic against the
                        # exact ellipse, using the returned combined bound.
                        actual0=fixed(p['commands'][cursor-1]['to']) if cursor>0 and 'to' in p['commands'][cursor-1] else [p0[i]*scale[i] for i in range(2)]
                        actual_points=[actual0,*[fixed(actual[k]) for k in ['control1','control2','to']]]
                        for t in [D(1)/4,D(1)/2,D(3)/4]:
                            c2,s2=unit(a+(j+t)*h);exact=add(center,[rx*c2,ry*s2]);weights=[(1-t)**3,3*t*(1-t)**2,3*t*t*(1-t),t**3]
                            for axis in range(2):
                                position=sum(weights[k]*actual_points[k][axis] for k in range(4));assert abs(position-exact[axis]*scale[axis])<=combined[axis]+EPS,(name,'sample',axis)
                            samples+=1
                        cursor+=1;segments+=1;arc_segments+=1
                    current=add(center,radial(theta+sweep,rx,ry))
            else:raise AssertionError(kind)
            assert cursor==span['firstCommand']+span['commandCount'],name
        assert cursor==len(p['commands']) and arc_segments==p['arcSegments'];paths+=1
    compiled+=1;checks.append({'name':c['name'],'sourceSha256':c['sha256'],'responseSha256':record['responseSha256'],'compiledPaths':len(result['paths'])})
XSD=Path('.codex-work/ecma376/xsd');schema=E.XMLSchema(E.parse(str(XSD/'pml.xsd'),E.XMLParser(resolve_entities=False,no_network=True)));xsd=0
for c in manifest.values():
    if not c.get('owned'):continue
    with zipfile.ZipFile(c['path']) as z:
        for part in z.namelist():
            if part.endswith('.xml') and any(part.startswith(prefix) for prefix in ['ppt/slides/slide','ppt/slideLayouts/slideLayout','ppt/slideMasters/slideMaster']):schema.assertValid(project(z.read(part))[0]);xsd+=1
result={'format':'musteroffice.native-path-independent/1','compiled':compiled,'unresolved':unresolved,'paths':paths,'controlPoints':points,'arcSegments':segments,'ellipseSamples':samples,'largestControlCoordinateDifference':str(largest),'ownedXsdParts':xsd,'precisionDigits':80,'cases':checks,
        'scope':'Coordinate and cubic remainder checks relative to exact binary64 evaluator outputs; not exact source guide math, raster pixels, paint semantics or Office/WPS acceptance.'}
(ROOT/('paths-independent.json' if PRESETS else 'independent.json')).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k!='cases'}))
