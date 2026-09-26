"""Independent high-precision author coordinate oracle shared by audits.

This uses Decimal/Chudnovsky and source transforms, never production intervals.
"""
from decimal import Decimal as D, getcontext
from functools import lru_cache
getcontext().prec = 140
U = D(2) ** 32
def pi():
    # Chudnovsky series; independent of the production Machin-enclosed constant.
    m=1;l=13591409;x=1;k=6;s=D(l)
    for i in range(1,12):
        m=m*(k*k*k-16*k)//(i*i*i);l+=545140134;x*= -262537412640768000
        s+=D(m*l)/x;k+=12
    return 426880*D(10005).sqrt()/s
PI=pi()
@lru_cache(None)
def cs(angle):
    a=angle%21600000
    if a<0:a+=21600000
    if a%5400000==0:return [(D(1),D(0)),(D(0),D(1)),(D(-1),D(0)),(D(0),D(-1))][int(a//5400000)]
    x=D(a)*PI/10800000;c=D(1);ct=c;s=x;st=x
    for k in range(1,160):
        ct*= -x*x/D((2*k-1)*(2*k));st*= -x*x/D((2*k)*(2*k+1));c+=ct;s+=st
    return c,s
def size(o):
    content=o['content']
    if content['kind']=='group':return content['viewport']
    if content['kind']=='shape' and content['geometry']['kind']=='path':return content['geometry']['viewport']
    return o['transform']['size']
def dims(s):return [D(s['width']),D(s['height'])]
def resolved(chain):
    # Rebuild the object's entire root-to-leaf author chain, without production
    # matrices, fixed-point intervals, cache, or traversal-state reuse.
    center=[D(0),D(0)];extent=[D(1),D(1)];source=[D(1),D(1)]
    angle=0;flip=[False,False];previous=False
    for o in chain:
        t=o['transform'];dst=dims(t['size']);src=dims(size(o))
        own_center=[D(t['origin'][axis])+dst[i]/2 for i,axis in enumerate(['x','y'])]
        if previous:
            v=[(own_center[i]-source[i]/2)*extent[i]/source[i]*(-1 if flip[i] else 1) for i in range(2)]
            c,s=cs(angle)
            center=[center[0]+c*v[0]-s*v[1],center[1]+s*v[0]+c*v[1]]
            # Partition the turn into alternating sectors centered on horizontal
            # and vertical; use full angle units rather than integer degrees.
            quarter=((t['rotation']%21600000)+2700000)//5400000
            parent_ratios=[extent[i]/source[i] for i in range(2)]
            if quarter%2:parent_ratios.reverse()
            extent=[dst[i]*parent_ratios[i] for i in range(2)]
            angle+=(-1 if flip[0]!=flip[1] else 1)*t['rotation']
        else:
            center=own_center;extent=dst;angle=t['rotation']
        flip=[flip[i]!=t[['flipHorizontal','flipVertical'][i]] for i in range(2)]
        source=src;previous=True
    # A zero source axis only occurs on non-group leaf shapes in this model.
    # Their point set is collapsed; the contract retains the unscaled own axis.
    if any(v==0 for v in source) and len(chain)>1:
        # Rebuild just the ancestors to obtain the scale of a zero-sized leaf.
        _, parent_scale, _=resolved(chain[:-1])
        parent_scale=[abs(v) for v in parent_scale]
        if ((chain[-1]['transform']['rotation']%21600000)+2700000)//5400000%2:
            parent_scale.reverse()
    else:parent_scale=[D(1),D(1)]
    scale=[(extent[i]/source[i] if source[i] else parent_scale[i])*(-1 if flip[i] else 1) for i in range(2)]
    return center,scale,angle
