"""Independent analytic spatial/color oracle at pixel centers of owned rectangles.

Reads requested world geometry and working colors, not rendered colors. Uses
Python double math, published sRGB transfer equations and exact Fraction rebasing.
This is not an Office/WPS rendering oracle or a general path coverage oracle.
"""
import bisect, hashlib, json, math, struct
from fractions import Fraction as F
from pathlib import Path

ROOT=Path('.codex-work/gradient-raster');U=1<<32
read=lambda p:json.loads(Path(p).read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
def linear(v):
    return math.copysign(abs(v)/12.92 if abs(v)<=0.04045 else ((abs(v)+0.055)/1.055)**2.4,v)
def encoded(v):
    return math.copysign(12.92*abs(v) if abs(v)<=0.0031308 else 1.055*abs(v)**(1/2.4)-0.055,v)
def color(g,t):
    mode=g['tile']
    if mode=='decal' and (t<0 or t>1):return [0.0]*4
    if mode=='repeat':t=t-math.floor(t)
    elif mode=='mirror':
        period=t-2*math.floor(t/2);t=period if period<=1 else 2-period
    else:t=max(0,min(1,t))
    stops=g['stops'];i=bisect.bisect_right([s['position'] for s in stops],t)
    if i==0:a=b=stops[0];f=0
    elif i==len(stops):a=b=stops[-1];f=0
    else:a,b=stops[i-1:i+1];f=(t-a['position'])/(b['position']-a['position'])
    a,b=list(a['srgb']),list(b['srgb'])
    if g['interpolation']=='linearSrgb':a[:3]=map(linear,a[:3]);b[:3]=map(linear,b[:3])
    if g['alpha']=='premultiplied':a[:3]=[x*a[3] for x in a[:3]];b[:3]=[x*b[3] for x in b[:3]]
    out=[x*(1-f)+y*f for x,y in zip(a,b)]
    if g['alpha']=='premultiplied':out[:3]=[v/out[3] if out[3] else 0 for v in out[:3]]
    if g['interpolation']=='linearSrgb':out[:3]=map(encoded,out[:3])
    return [max(0,min(1,v))*out[3] for v in out[:3]]+[out[3]]
def f32(bits):return F(struct.unpack('<f',struct.pack('<I',bits))[0])
def nearest(value):
    if value==0:return 0
    sign=0x80000000 if value<0 else 0;v=abs(value)
    guess=struct.unpack('<I',struct.pack('<f',float(v)))[0]
    return min(range(max(0,guess-1),guess+2),key=lambda b:(abs(f32(b)-v),b&1))|sign

counts={'cases':0,'pixels':0,'channels':0,'wireGradientCoordinates':0,'wireStopValues':0}
maximum=0;records=[]
for c in read(ROOT/'parity.json')['cases']:
    if not c['oracle']:continue
    data={k:Path(c[k+'Path']).read_bytes() for k in ['request','response','frame','pixels']}
    for k,b in data.items():assert sha(b)==c[k+'Sha256']
    q,r=map(json.loads,[data['request'],data['response']]);v=q['viewport'];g=q['draws'][0]['brush']['gradient'];geom=g['geometry']
    factor=F(v['scale']['numerator'],v['scale']['denominator']*U)
    def point(p):return [F(int(p[a])-int(v['origin'][a]))*factor for a in ['x','y']]
    if geom['kind']=='linear':
        p0,p1=point(geom['start']),point(geom['end']);coords=p0+p1
    else:coords=point(geom['center'])+[F(int(geom['radius']))*factor,F(0)]
    words=struct.unpack('<'+'I'*(len(data['frame'])//4),data['frame']);start=10
    assert words[1]==4 and words[8:10]==(0,1)
    for _ in range(words[5]):start+=2+7*words[start+1]
    assert words[start:start+5]==(int(geom['kind']=='radial'),['clamp','repeat','mirror','decal'].index(g['tile']),int(g['interpolation']=='linearSrgb'),int(g['alpha']=='premultiplied'),len(g['stops']))
    assert words[start+5:start+9]==tuple(nearest(x) for x in coords)
    counts['wireGradientCoordinates']+=4
    for i,s in enumerate(g['stops']):
        assert words[start+9+5*i:start+14+5*i]==tuple(nearest(F(x)) for x in [s['position']]+s['srgb'])
        counts['wireStopValues']+=5
    max_coord=max(abs(f32(words[start+5+i])-x)*U for i,x in enumerate(coords))
    assert max_coord<=int(r['info']['work']['gradientCoordinateErrorBound'])<=max_coord+2
    value_error=max(abs(float(f32(nearest(F(x))))-x) for s in g['stops'] for x in [s['position']]+s['srgb'])
    assert value_error==r['info']['work']['gradientValueErrorBound']
    origin=point(q['draws'][0]['origin']);points=q['paths'][0]['commands']
    bounds=[min(F(int(c['to'][a]))*factor for c in points if 'to' in c)+origin[i] for i,a in enumerate(['x','y'])]+[max(F(int(c['to'][a]))*factor for c in points if 'to' in c)+origin[i] for i,a in enumerate(['x','y'])]
    local_max=0
    for y in range(v['height']):
        for x in range(v['width']):
            px,py=x+0.5,y+0.5
            if not (bounds[0]<=px<bounds[2] and bounds[1]<=py<bounds[3]):out=[0.0]*4
            else:
                if geom['kind']=='linear':
                    dx,dy=[float(b-a) for a,b in zip(p0,p1)];t=((px-float(p0[0]))*dx+(py-float(p0[1]))*dy)/(dx*dx+dy*dy)
                else:t=math.hypot(px-float(coords[0]),py-float(coords[1]))/float(coords[2])
                out=color(g,t)
            bg=[z/255 for z in v['background']];bg[:3]=[z*bg[3] for z in bg[:3]]
            expected=[round(max(0,min(1,out[i]+bg[i]*(1-out[3])))*255) for i in range(4)]
            actual=data['pixels'][(y*v['width']+x)*4:(y*v['width']+x+1)*4]
            error=max(abs(a-b) for a,b in zip(actual,expected));local_max=max(local_max,error)
            assert error<=1,(c['name'],x,y,list(actual),expected,error)
            counts['pixels']+=1;counts['channels']+=4
    maximum=max(maximum,local_max);counts['cases']+=1
    records.append({'name':c['name'],'pixelsSha256':c['pixelsSha256'],'requestSha256':c['requestSha256'],'maximumChannelDifference':local_max})
report={'format':'musteroffice.gradient-raster-reference/1','counts':counts,'toleranceRgba8':1,'maximumChannelDifference':maximum,'cases':records,'scope':__doc__.strip()}
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'counts':counts,'maximumChannelDifference':maximum}))
