"""Compare current pixels with independent analytic/frozen source references."""
import hashlib,json
from pathlib import Path
from fractions import Fraction as F
root=Path('.codex-work/gradient-coordinates')
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):
    assert entry(r['path'])==r;return Path(r['path']).read_bytes()
components=json.loads((root/'components.json').read_text())
replay=json.loads((root/'replay.json').read_text())
records=[]
def compare(name,current,expected,indices=None,inputs=None):
    a=load(current);b=expected
    if indices is None:indices=range(len(a)//4)
    assert len(b)==len(indices)*4
    maximum=different=0
    for n,index in enumerate(indices):
        delta=max(abs(a[index*4+k]-b[n*4+k]) for k in range(4))
        assert delta<=1,(name,index,a[index*4:index*4+4],b[n*4:n*4+4],delta)
        maximum=max(maximum,delta);different+=delta!=0
    records.append(dict(name=name,pixels=current,verifiedPixels=len(indices),maximumChannelDifference=maximum,
                        differingPixels=different,**(inputs or {})))
def tile(x,mode):
    if mode=='clamp':return max(F(0),min(F(1),x))
    if mode=='repeat':return x%1
    return 1-abs(x%2-1)
def byte(x):return int(x*255+F(1,2))
# Rational coordinates and ramp, independently reconstructed from public Q32
# requests. No generated float32 frame or implementation matrix inverse used.
old=json.loads(Path('.codex-work/gradient-field/parity.json').read_text())
for c in old['cases'][:27]:
    q=json.loads(load(c['request']));g=q['draws'][0]['brush']['gradient'];p=g['geometry']['plane']
    point=lambda p: [F(int(p[k]),2**32) for k in ['x','y']]
    ox,oy=point(p['origin']);a,c0=point(p['xStep']);b,d=point(p['yStep']);det=a*d-b*c0
    pixels=bytearray()
    for y in range(24):
        for x in range(32):
            dx,dy=F(2*x+1,2)-ox,F(2*y+1,2)-oy
            u,v=tile((d*dx-b*dy)/det,p['tileX']),tile((a*dy-c0*dx)/det,p['tileY'])
            t=(u+v)/2;pixels.extend([byte(1-t),0,byte(t),255])
    for key in ['frame','sceneFrame']:
        current=next(r for r in components['cases'] if r['name']=='gradient-field/'+c['name']+'/'+key)
        assert current['frame']==c[key]
        compare(current['name'],current['currentPixels'],pixels,inputs=dict(request=c['request']))
# Reuse immutable source-derived independent references (not old renderer output).
# Check source/frame identities before comparing the current renderer's pixels.
rect_source=json.loads(Path('.codex-work/rect-gradient/source-parity.json').read_text())
reference_reports=[]
for kind,path in [('rect','.codex-work/rect-gradient/reference.json'),('elliptic','.codex-work/elliptic-source/reference.json')]:
    reference_reports.append(entry(path));old=json.loads(Path(path).read_text())
    for c in old['cases']:
        if kind=='rect':
            source=next(r for r in rect_source['cases'] if r['name']=='new/'+c['name'])
            current=next(r for r in components['cases'] if r['name']=='source/new/'+c['name'])
            assert source['frame']==current['frame'];pixels=current['currentPixels']
        else:
            source=next(r for r in replay['cases'] if r['kind']=='source-parity' and r['name']=='new/'+c['name'])
            pixels=source['pixels']
        assert source['source']['sha256']==c['source']['sha256'];load(c['source'])
        indices=[y*400+x for y in range(0,300,16) for x in range(0,400,16)] if kind=='elliptic' else None
        compare(kind+'/'+c['name'],pixels,load(c['reference']),indices,dict(source=c['source'],reference=c['reference']))
group=[next(c for c in components['cases'] if c['name']=='source/new/'+name) for name in ['group-inherited','group-inherited-control']]
assert load(group[0]['currentPixels'])==load(group[1]['currentPixels'])
report=dict(format='musteroffice.gradient-coordinate-reference/1',cases=records,
    verifiedPixels=sum(r['verifiedPixels'] for r in records),maximumChannelDifference=max(r['maximumChannelDifference'] for r in records),
    groupInheritedControlEqual=True,previousReferenceReports=reference_reports,
    scope='Exact rational public linear planes plus frozen independently computed raw XML rectangular/elliptic references. Original <=1 RGBA8 color quantization criterion retained. Coordinate rounding and rigid transform exactness have separate zero-tolerance checks. Not target application acceptance or a unified color/coverage error proof.')
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(cases=len(records),pixels=report['verifiedPixels'],maximum=report['maximumChannelDifference'])))
