"""Independent Decimal geometry from original XML, including analytic curve extrema."""
import hashlib,json,zipfile
from pathlib import Path
from decimal import Decimal as D,localcontext
from lxml import etree as X
root=Path('.codex-work/radial-observation');out=root/'reference';out.mkdir(exist_ok=True)
ns=dict(a='http://schemas.openxmlformats.org/drawingml/2006/main',p='http://schemas.openxmlformats.org/presentationml/2006/main')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def pct(s):return D(s[:-1])/100 if s.endswith('%') else D(s)/100000
parser=X.XMLParser(resolve_entities=False,no_network=True)
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd',parser))
files=sorted((root/'native').glob('*.pptx'))+[Path(c['source']['path']) for c in json.loads((root/'sources.json').read_text())['cases']];assert len(files)==40
parts=0;records=[];excluded=[];checks=0

def curve(points):
    results=list(points[::len(points)-1]);degree=len(points)-1
    for axis in [0,1]:
        p=[v[axis] for v in points];roots=[]
        if degree==2:
            a=p[0]-2*p[1]+p[2]
            if a:roots=[(p[0]-p[1])/a]
        elif degree==3:
            a=-p[0]+3*p[1]-3*p[2]+p[3];b=2*(p[0]-2*p[1]+p[2]);c=p[1]-p[0]
            if a:
                disc=b*b-4*a*c
                if disc>=0:roots=[(-b-disc.sqrt())/(2*a),(-b+disc.sqrt())/(2*a)]
            elif b:roots=[-c/b]
        for t in roots:
            if 0<t<1:
                w=[(1-t)**2,2*t*(1-t),t*t] if degree==2 else [(1-t)**3,3*t*(1-t)**2,3*t*t*(1-t),t**3]
                results.append(tuple(sum(v[i]*c for v,c in zip(points,w)) for i in [0,1]))
    return results

def geometry(sp):
    props=sp.find('p:spPr',ns);extent=props.find('a:xfrm/a:ext',ns);w,h=D(extent.get('cx')),D(extent.get('cy'))
    preset=props.find('a:prstGeom',ns)
    if preset is not None:
        if preset.get('prst') not in ['rect','ellipse']:return None
        return [D(0),D(0),w,h]
    points=[]
    for path in props.findall('a:custGeom/a:pathLst/a:path',ns):
        scale=[w/D(path.get('w')) if path.get('w') else D(1),h/D(path.get('h')) if path.get('h') else D(1)];current=start=None
        for cmd in path:
            name=X.QName(cmd).localname
            values=[(D(p.get('x'))*scale[0],D(p.get('y'))*scale[1]) for p in cmd.findall('a:pt',ns)]
            if name=='moveTo':current=start=values[0];continue
            if name=='close':values=[start]
            assert current is not None and name in ['lnTo','quadBezTo','cubicBezTo','close']
            points.extend(curve([current]+values) if len(values)>1 else [current,*values]);current=values[-1]
            if name=='close':current=start=None
    return [min(p[0] for p in points),min(p[1] for p in points),max(p[0] for p in points),max(p[1] for p in points)]

def fill(sp):
    props=sp.find('p:spPr',ns);g=props.find('a:gradFill',ns)
    if g is not None:return g
    assert props.find('a:grpFill',ns) is not None
    for parent in sp.iterancestors():
        g=parent.find('p:grpSpPr/a:gradFill',ns)
        if g is not None:return g
    raise AssertionError('unresolved owned fill')

def reference(box,g):
    ext=[box[2]-box[0],box[3]-box[1]];tile=g.find('a:tileRect',ns)
    edges=[D(0) if tile is None else pct(tile.get(k,'0')) for k in ['l','t','r','b']]
    tile=[box[i]+(1 if i<2 else -1)*ext[i%2]*edges[i] for i in range(4)]
    w,h=tile[2]-tile[0],tile[3]-tile[1];radius=(w*w+h*h).sqrt()/2
    center=[(tile[0]+tile[2])/2,(tile[1]+tile[3])/2];origin=[c-radius for c in center];diameter=2*radius
    f=g.find('a:path',ns);assert f.get('path')=='circle';f=f.find('a:fillToRect',ns)
    margins=[D('.5') if f is None else pct(f.get(k,'50000')) for k in ['l','t','r','b']]
    inner=[];radii=[];focus=[];scale=[]
    for i in [0,1]:
        length=tile[i+2]-tile[i];lo=tile[i]+length*margins[i];hi=tile[i+2]-length*margins[i+2];size=hi-lo
        factor=size/length;point=lo+size*(lo-tile[i])/(length-size) if size>0 and length!=size else lo
        inner.append(factor*center[i]+(1-factor)*point);radii.append(radius*factor);scale.append(factor);focus.append(point)
    return dict(pathBounds=box,tileRectangle=tile,outerCenter=center,outerRadius=[radius],innerCenter=inner,innerRadii=radii,focusPoint=focus,focusScale=scale)

with localcontext() as ctx:
    ctx.prec=160
    for source in files:
        with zipfile.ZipFile(source) as z:
            for n in z.namelist():
                if not n.endswith('.xml'):continue
                e=X.fromstring(z.read(n),parser);tag=X.QName(e)
                if tag.namespace==ns['p'] and tag.localname in ['sld','sldMaster','sldLayout','presentation']:schema.assertValid(e);parts+=1
            page=X.fromstring(z.read('ppt/slides/slide1.xml'),parser);presentation=X.fromstring(z.read('ppt/presentation.xml'),parser)
        plan_path=source.with_suffix('.plan.json') if source.parent==root/'native' else root/'runtime'/('observed-'+source.stem.removeprefix('radial-')+'.response.json')
        plans=json.loads(plan_path.read_text());plans=plans['plans'] if isinstance(plans,dict) else plans
        for plan in plans:
            target=plan['target']
            if target['kind']=='background':
                size=presentation.find('p:sldSz',ns);box=[D(0),D(0),D(size.get('cx')),D(size.get('cy'))];g=page.find('p:cSld/p:bg/p:bgPr/a:gradFill',ns)
            else:
                matches=[sp for sp in page.findall('.//p:sp',ns) if sp.find('p:nvSpPr/p:cNvPr',ns).get('id')==str(target['nativeId'])];assert len(matches)==1
                sp=matches[0];box=geometry(sp)
                if box is None:excluded.append(dict(name=source.stem,target=target,reason='Preset path equations are outside this independent XML oracle. Covered by current Native/WASM queries.'));continue
                g=fill(sp)
            expected=reference(box,g);actual=plan['layout'];count=0
            for field,values in expected.items():
                assert len(values)==len(actual[field]['values'])
                for i,value in enumerate(values):
                    observed=D(actual[field]['values'][i])/D(2**32);error=D(actual[field]['errors'][i])/D(2**32)
                    assert error>=0 and abs(observed-value)<=error+D('1e-120'),(source.stem,field,i,str(observed),str(value),str(error))
                    count+=1
            checks+=count;p=out/(source.stem+'-'+str(target.get('nativeId','background'))+'.json');p.write_text(json.dumps({k:[str(v) for v in values] for k,values in expected.items()},indent=2)+'\n')
            records.append(dict(name=source.stem,target=target,plan=entry(plan_path),reference=entry(p),comparisons=count))
report=dict(format='musteroffice.radial-anchor-reference/1',officialXsdParts=parts,sourceFiles=[entry(p) for p in files],decimalPrecision=160,comparisons=checks,cases=records,excluded=excluded,scope='Original XML and independent derivative roots for quadratic/cubic extrema; 160-digit Decimal geometry compared to each returned Q32 error enclosure. No compiled scene or native plan value is used as an expected geometric input. Rotations remain separate placement. Does not validate final radial pixels or Office/WPS behavior.')
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(parts=parts,cases=len(records),excluded=len(excluded),comparisons=checks)))
