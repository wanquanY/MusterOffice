"""Build and verify the actual standalone scalar kernel; not a page renderer.

Independent exact rational power-polynomial/Sturm reference. No NumPy, Skia,
source-layout output or shader implementation supplies expected answers.
"""
import hashlib
import json
import math
import os
from pathlib import Path
import random
import struct
import subprocess
from fractions import Fraction as F

ROOT=Path('.codex-work/elliptic-scalar')
ROOT.mkdir(exist_ok=True)
def entry(p):
    p=Path(p); b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def f32(v): return struct.unpack('<f',struct.pack('<f',v))[0]
def bits(v): return struct.unpack('<I',struct.pack('<f',v))[0]
commands=[]
def run(name,args,env=None):
    p=ROOT/(name+'.log')
    with p.open('w') as log: r=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,env=env)
    commands.append(dict(name=name,argv=args,exitCode=r.returncode,log=entry(p)))
    if r.returncode: raise RuntimeError(name+': '+p.read_text()[-3000:])

common=['-std=c++20','-O3','-Wall','-Wextra','-Werror','-fno-exceptions','-fno-rtti',
        '-ffp-contract=off','-Icomponents/skia','components/skia/mo_elliptic_field.cpp',
        'tools/verification/elliptic-scalar-probe.cpp']
run('native-build',['clang++',*common,'-o',str(ROOT/'native-probe')])
run('sanitizer-build',['clang++',*common,'-fsanitize=address,undefined','-fno-sanitize-recover=all',
                      '-g1','-fno-omit-frame-pointer','-o',str(ROOT/'sanitizer-probe')])
env=os.environ.copy(); env['EM_CONFIG']=str(Path('.codex-work/emsdk/.emscripten').resolve())
run('wasm-build',['.codex-work/emsdk/upstream/emscripten/em++',*common,'--no-entry',
    '-sMODULARIZE=1','-sEXPORT_ES6=1','-sENVIRONMENT=node,web','-sFILESYSTEM=0',
    '-sALLOW_MEMORY_GROWTH=1','-sMAXIMUM_MEMORY=67108864','-sSTACK_SIZE=1048576',
    '-sDYNAMIC_EXECUTION=0','-sEXPORTED_FUNCTIONS=["_mo_elliptic_probe","_mo_elliptic_interval_probe","_malloc","_free"]',
    '-sEXPORTED_RUNTIME_METHODS=["HEAPU32"]','-o',str(ROOT/'probe.mjs')],env)

cases=[]
def case(name,values,budget=512,required=None):
    cases.append(dict(name=name,values=[f32(v) for v in values],budget=budget,required=required))
# x,y,cx,cy,sx,sy. These names do not claim Office behavior.
for name,values in [
    ('center',[0,0,0,0,0,0]),('point',[.25,.5,0,0,0,0]),
    ('offset-point',[.1,.2,.6,-.2,0,0]),('external-point',[0,0,2,0,0,0]),
    ('earlier-point-root',[1,0,2,0,0,0]),
    ('inner-ellipse',[.1,.1,0,0,.3,.8]),('ellipse',[.5,.3,0,0,.3,.8]),
    ('line-on',[0,.7,0,0,0,.6]),('line-off',[.01,.7,0,0,0,.6]),
    ('line-horizontal',[.7,0,0,0,.6,0]),('inner-line',[0,.25,0,0,0,.6]),
    ('whole-circle',[1,0,0,0,1,1]),('expanded',[.7,.4,0,0,2,1.5]),
    ('root-isolation',[-.875,-.375,-1,-2,.03125,2]),
    ('multiple-crossings',[-.9187984697040693,-.32614082270717,
        -.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122]),
    ('outside',[1,1,0,0,0,0]),('endpoint-tangent',[1,0,1,2,0,0]),
    ('dyadic-tangent',[.96875,.375,1.25,0,0,0]),
    ('tiny-axis',[.01,.3,0,0,2**-149,.6]),
    ('huge-focus',[0,0,32768,-32768,.5,.75]),
]: case(name,values,required=0)
case('non-dyadic-tangent',[.875,.5,1.25,0,0,0],required=0)
case('circle-non-dyadic-tangent',[.875,.5,.9375,0,.25,.25],required=0)
case('inner-boundary-ambiguous',[0,2**-149,32768,0,32768,1],required=2)
case('tangent-near-miss',[.875,f32(.5)+2**-24,1.25,0,0,0],required=0)
case('tangent-near-hit',[.875,f32(.5)-2**-25,1.25,0,0,0],required=0)
for name,values in [('negative-radius',[0,0,0,0,-1,1]),('outside-domain',[2,0,0,0,0,0]),
                    ('large-center',[0,0,65536,0,0,0]),('nan',[float('nan'),0,0,0,0,0]),
                    ('infinity',[0,0,float('inf'),0,0,0])]:
    case(name,values,required=1)
case('zero-budget',[.5,.4,0,0,0,0],0,3)
case('one-node-multiple',[-.875,-.375,-1,-2,.03125,2],1,3)
# Exact binary32 test domain, deterministic distributions including subnormals.
r=random.Random(0xE111C)
for i in range(800):
    c=[r.uniform(-3,3) for _ in range(2)]
    s=[0 if r.random()<.08 else 2**r.uniform(-8,3) for _ in range(2)]
    case('random-'+str(i),[r.uniform(-1,1),r.uniform(-1,1),*c,*s])
for i in range(120):
    def wide(): return math.ldexp(r.uniform(.5,1),r.randint(-149,15))
    case('wide-'+str(i),[r.uniform(-1,1),r.uniform(-1,1),wide()*r.choice([-1,1]),
                        wide()*r.choice([-1,1]),wide(),wide()])
# Both nextafter directions around an exact double root, and a fixed grid of
# centered, offset and expanded foci, exercise mode changes and degeneracy.
for i in range(-20,21):
    y=struct.unpack('<f',struct.pack('<I',bits(.5)+i))[0]
    case('near-tangent-'+str(i),[.875,y,1.25,0,0,0])
for i,field in enumerate([(0,0,0,0),(.5,-.25,0,0),(0,0,.3,.8),(-1,-2,.03125,2),
                          (0,0,0,.6),(0,0,1,1),(0,0,2,1.5)]):
    for x in [-.875,-.5,-.125,0,.125,.5,.875]:
        for y in [-.75,-.375,0,.375,.75]: case(f'grid-{i}-{x}-{y}',[x,y,*field])

def trim(p):
    while len(p)>1 and p[-1]==0: p.pop()
    return p
def add(a,b): return trim([(a[i] if i<len(a) else F(0))+(b[i] if i<len(b) else F(0)) for i in range(max(len(a),len(b)))])
def neg(a): return [-v for v in a]
def mul(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b): out[i+j]+=x*y
    return trim(out)
def square(a): return mul(a,a)
def poly(values):
    x,y,cx,cy,sx,sy=map(F,values)
    ax=square([x-cx,cx]); ay=square([y-cy,cy])
    bx=square([sx,1-sx]); by=square([sy,1-sy])
    p=add(add(mul(ax,by),mul(ay,bx)),neg(mul(bx,by)))
    while len(p)>1 and p[0]==0: p=p[1:]
    return p
def inside(values):
    x,y,cx,cy,sx,sy=map(F,values)
    if (sx==0 and x!=cx) or (sy==0 and y!=cy): return False
    return (F(0) if sx==0 else ((x-cx)/sx)**2)+(F(0) if sy==0 else ((y-cy)/sy)**2)<=1
def remainder(a,b):
    a=a.copy()
    while len(a)>=len(b) and a!=[0]:
        n=len(a)-len(b); k=a[-1]/b[-1]
        for i,v in enumerate(b): a[n+i]-=k*v
        trim(a)
    return a
def sturm(p):
    seq=[p,trim([i*p[i] for i in range(1,len(p))]) or [F(0)]]
    if seq[-1]==[0]: return seq[:-1]
    while True:
        r=neg(remainder(seq[-2],seq[-1]))
        if r==[0]: return seq
        scale=abs(r[-1]); seq.append([v/scale for v in r])
def at(p,x):
    out=F(0)
    for v in reversed(p): out=out*x+v
    return out
def variations(seq,x):
    s=[1 if v>0 else -1 for p in seq if (v:=at(p,x))!=0]
    return sum(a!=b for a,b in zip(s,s[1:]))

raw=b''.join(struct.pack('<6fI',*v['values'],v['budget']) for v in cases)
(ROOT/'cases.bin').write_bytes(raw)
labels=[dict(name=v['name'],words=[bits(x) for x in v['values']],budget=v['budget'],required=v['required']) for v in cases]
(ROOT/'cases.json').write_text(json.dumps(labels,indent=2)+'\n')
def probe(name,data,n,mode=None):
    args=[str(ROOT/(name+'-probe'))]+([mode] if mode else [])
    r=subprocess.run(args,input=struct.pack('<I',n)+data,capture_output=True)
    (ROOT/(name+('-'+mode if mode else '')+'.log')).write_bytes(r.stderr)
    assert r.returncode==0,(name,r.returncode,r.stderr.decode())
    assert not r.stderr,r.stderr.decode()
    p=ROOT/(name+('-'+mode if mode else '')+'.bin'); p.write_bytes(r.stdout)
    return r.stdout
native=probe('native',raw,len(cases)); sanitizer=probe('sanitizer',raw,len(cases))
assert native==sanitizer
assert len(native)==len(cases)*40
results=[]; counts={}; fast=0
for i,c in enumerate(cases):
    status,location,value,lo,hi,nodes,degree,quick=struct.unpack('<IIfddIII',native[i*40:(i+1)*40])
    if c['required'] is not None: assert status==c['required'],(c,status,location,value,nodes)
    assert nodes<=min(c['budget'],512)
    counts[str(status)]=counts.get(str(status),0)+1; fast+=quick
    mathematical=None
    if status==0:
        assert 0<=lo<=value<=hi<=1 and F(hi)-F(lo)<=F(1,2**23),(c,lo,value,hi)
        if location==0:
            assert inside(c['values']) and (value,lo,hi)==(0,0,0),c
        else:
            assert not inside(c['values']),c
            p=poly(c['values']); seq=sturm(p); start=variations(seq,F(0))
            roots=start-variations(seq,F(1)); mathematical=dict(degree=len(p)-1,roots=roots)
            if c['name']=='multiple-crossings': assert roots==3,roots
            if location==2: assert roots==0 and value==lo==hi==1,(c,roots)
            else:
                assert location==1
                before=start-variations(seq,F(lo))-(1 if at(p,F(lo))==0 else 0)
                through=start-variations(seq,F(hi))
                assert before==0 and through>=1,(c,lo,hi,before,through)
    results.append(dict(name=c['name'],status=status,location=location,value=value,
                        lower=lo,upper=hi,nodes=nodes,degree=degree,certifiedFastPath=bool(quick),reference=mathematical))

# Check error-free arithmetic separately with exact rational values, including
# cancellation, exponents far apart, and the whole algorithm's exponent range.
pairs=[(0.,0.,0.),(1.,-1.,.5),(1.,2**-149,2**-80),
       (2**60,2**-600,1-2**-52),(1+2**-52,1-2**-52,2**-100)]
for i in range(4000):
    pairs.append((*[math.ldexp(r.uniform(-1,1),r.randint(-450,35)) for _ in range(2)],
                  math.ldexp(r.uniform(.5,1),r.randint(-100,0))))
interval_raw=b''.join(struct.pack('<ddd',a,b,t) for a,b,t in pairs)
(ROOT/'interval-cases.bin').write_bytes(interval_raw)
iv=probe('native',interval_raw,len(pairs),'interval')
assert iv==probe('sanitizer',interval_raw,len(pairs),'interval')
for i,(a,b,t) in enumerate(pairs):
    low,high,plow,phigh,mlow,mhigh=struct.unpack('<dddddd',iv[i*48:(i+1)*48])
    assert F(low)<=F(a)+F(b)<=F(high),(a,b,low,high)
    assert F(plow)<=F(a)*F(b)<=F(phigh),(a,b,plow,phigh)
    assert F(mlow)<=(1-F(t))*F(a)+F(t)*F(b)<=F(mhigh),(a,b,t,mlow,mhigh)

run('wasm-parity',['node','tools/verification/elliptic-scalar-parity.mjs'])
report=dict(format='musteroffice.elliptic-scalar-verification/1',commands=commands,
    cases=len(cases),statuses=counts,certifiedFastPaths=fast,maxVisitedNodes=max(v['nodes'] for v in results),
    exactIntervalComparisons=len(pairs)*3,results=results,
    artifacts=[entry(ROOT/n) for n in ['native-probe','sanitizer-probe','probe.mjs','probe.wasm',
      'cases.bin','cases.json','native.bin','sanitizer.bin','interval-cases.bin','native-interval.bin',
      'sanitizer-interval.bin','wasm.bin','wasm-interval.bin']],
    limitations=['This verifies scalar geometry only; source-page/raster integration is not implemented.',
      'Success encloses exact binary32-input first roots; source parameter error and pixel color error are outside this bound.',
      'Unresolved tangencies and ill-conditioned inputs diagnose precision or budget; no completion claim for full gradients.'])
(ROOT/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:report[k] for k in ['cases','statuses','certifiedFastPaths','maxVisitedNodes','exactIntervalComparisons']}))
