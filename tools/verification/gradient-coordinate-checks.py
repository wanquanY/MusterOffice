"""Exact rational reference for device-to-gradient mapping and final float32 rounding."""
import hashlib,json,math,os,random,struct,subprocess
from fractions import Fraction as F
from pathlib import Path
root=Path('.codex-work/gradient-coordinates');root.mkdir(exist_ok=True)
common=['-std=c++20','-O3','-Wall','-Wextra','-Werror','-fno-exceptions','-fno-rtti','-ffp-contract=off',
 '-Icomponents/skia','components/skia/mo_gradient_coordinates.cpp','tools/verification/gradient-coordinate-probe.cpp']
commands=[]
def run(name,args,env=None):
    p=root/(name+'.log')
    with p.open('w') as log:r=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT)
    commands.append(dict(name=name,argv=args,exitCode=r.returncode,log=str(p)))
    assert r.returncode==0,p.read_text()
run('native-build',['clang++',*common,'-o',str(root/'native-probe')])
run('sanitizer-build',['clang++',*common,'-fsanitize=address,undefined','-fno-sanitize-recover=all','-o',str(root/'sanitizer-probe')])
env=dict(os.environ,EM_CONFIG=str(Path('.codex-work/emsdk/.emscripten').resolve()))
run('wasm-build',['.codex-work/emsdk/upstream/emscripten/em++',*common,'--no-entry',
 '-sMODULARIZE=1','-sEXPORT_ES6=1','-sENVIRONMENT=node,web','-sFILESYSTEM=0','-sALLOW_MEMORY_GROWTH=1',
 '-sDYNAMIC_EXECUTION=0','-sEXPORTED_FUNCTIONS=["_mo_coordinates_probe","_malloc","_free"]',
 '-sEXPORTED_RUNTIME_METHODS=["HEAPU32"]','-o',str(root/'probe.mjs')],env)
def bits(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def number(b):return struct.unpack('<f',struct.pack('<I',b))[0]
def f32(f):return number(bits(f))
def rounded(n):
    negative=n<0;n=abs(n);lo,hi=0,0x3f800000
    while lo<hi:
        mid=(lo+hi+1)//2
        if F(number(mid))<=n:lo=mid
        else:hi=mid-1
    if lo!=0x3f800000:
        midpoint=(F(number(lo))+F(number(lo+1)))/2
        if n>midpoint or n==midpoint and lo%2:lo+=1
    return lo|0x80000000 if negative and lo else lo
def reference(w):
    m=list(map(number,w[:6]));sx,sy,x,y=map(number,w[9:]);tx,ty,center=w[6:9]
    if center>1 or tx>2 or ty>2 or any(not math.isfinite(v) or abs(v)>32768 for v in m+[x,y]) or any(not math.isfinite(v) or not 0<=v<=1 for v in [sx,sy]):return None
    a,b,ox,c,d,oy=map(F,m);det=a*d-b*c
    if det==0:return None
    u=((F(x)-ox)*d-(F(y)-oy)*b)/det;v=((F(y)-oy)*a-(F(x)-ox)*c)/det
    result=[]
    for value,mode,scale in [(u,tx,sx),(v,ty,sy)]:
        if mode==0:value=max(F(0),min(F(1),value))
        elif mode==1:value%=1
        else:value%=2;value=min(value,2-value)
        if center:value=(2*value-1)*F(scale)
        result.append(rounded(value))
    return result
cases=[]
def add(name,m=(300,0,25,0,150,50),tile=(2,2),center=True,scale=(.89442718,.4472136),point=(332.5,140.5)):
    cases.append(dict(name=name,words=[*map(bits,m),*tile,int(center),*map(bits,scale),*map(bits,point)]))
# Rigid transformations map corresponding device samples to exactly the same
# local point; fractional origins and both diagonal signs are included.
pairs=[]
for x,y in [(322.5,120.5),(86.5,141.5),(49.5,173.5),(284.5,173.5),(25,50),(175,125),(0,0),(400,300)]:
    start=len(cases);add('rigid-base',point=(x,y))
    for name,m,point in [
        ('translate',(300,0,35,0,150,70),(x+10,y+20)),
        ('flip-h',(-300,0,325,0,150,50),(350-x,y)),
        ('flip-v',(300,0,25,0,-150,200),(x,250-y)),
        ('quarter',(0,-150,250,300,0,-25),(300-y,x-50))]:
        pairs.append((start,len(cases)));add(name,m=m,point=point)
for mode_x in range(3):
 for mode_y in range(3):
  for center in [False,True]:
   for k in range(-32,33):
    add('boundaries',m=(1,0,0,0,1,0),tile=(mode_x,mode_y),center=center,scale=(1,.3),point=(k/4,-k/8))
for k in range(1,65,2):
 for sign in [-1,1]:
  for center in [False,True]:
   add('midpoint',m=(1,0,sign*k*2**-25,0,1,0),tile=(0,0),center=center,scale=(1,1),point=(.5,.5))
   add('subnormal-midpoint',m=(2,0,0,0,2,0),tile=(0,0),center=center,scale=(2**-149,2**-148),point=(k*2**-149,.5))
r=random.Random(936)
def random_float():
    # Full allowed binary32 exponent range, including signed subnormal values.
    exponent=r.randrange(0,143);mantissa=r.randrange(1<<23)
    if exponent==142:mantissa=0
    return number((r.randrange(2)<<31)|(exponent<<23)|mantissa)
for i in range(4000):
    m=[random_float() for _ in range(6)] if i<2000 else [f32(r.uniform(-800,800)) for _ in range(6)]
    add('random-'+str(i),m=m,tile=(r.randrange(3),r.randrange(3)),center=bool(r.randrange(2)),
        scale=(r.choice([0,1,2**-149,f32(r.random())]),r.choice([0,1,2**-148,f32(r.random())])),
        point=(random_float(),random_float()) if i<2000 else (f32(r.uniform(-32000,32000)),f32(r.uniform(-32000,32000))))
for i in range(2000):
    sx,sy,ox,oy=[random_float() for _ in range(4)]
    if i>=1000:sx,sy,ox,oy=[f32(r.uniform(-800,800)) for _ in range(4)]
    m=(sx,0,ox,0,sy,oy) if i%2 else (0,sy,ox,sx,0,oy)
    add('axes-'+str(i),m=m,tile=(r.randrange(3),r.randrange(3)),center=bool(r.randrange(2)),
        scale=(r.choice([0,1,2**-149,f32(r.random())]),r.choice([0,1,2**-148,f32(r.random())])),
        point=(random_float(),random_float()) if i<1000 else (f32(r.uniform(-32000,32000)),f32(r.uniform(-32000,32000))))
for value in [0,2**-149,1,32768]:
    add('singular',m=(value,value,0,value,value,0),scale=(0,0))
for location in list(range(6))+[9,10,11,12]:
    for v in [float('nan'),float('inf'),float('-inf'),65536]:
        add('invalid');cases[-1]['words'][location]=bits(v)
for field in [6,7,8]:add('invalid-enum');cases[-1]['words'][field]=255
raw=struct.pack('<I',len(cases))+b''.join(struct.pack('<13I',*c['words']) for c in cases)
(root/'input.bin').write_bytes(raw)
def execute(name):
    p=subprocess.run([str(root/name)],input=raw,capture_output=True,env=dict(os.environ,ASAN_OPTIONS='detect_leaks=0',UBSAN_OPTIONS='halt_on_error=1'))
    assert p.returncode==0 and p.stderr==b'',(name,p.returncode,p.stderr)
    (root/(name.replace('-probe','')+'.bin')).write_bytes(p.stdout);return p.stdout
native=execute('native-probe');assert execute('sanitizer-probe')==native
assert len(native)==len(cases)*16;fallback=success=invalid=0
for i,c in enumerate(cases):
    valid,x,y,exact=struct.unpack_from('<4I',native,i*16);expected=reference(c['words'])
    assert valid==int(expected is not None),(i,c,valid,expected)
    if valid:assert [x,y]==expected,(i,c,[hex(x),hex(y)],[hex(v) for v in expected]);success+=1
    else:invalid+=1
    fallback+=exact;c['result']=dict(valid=bool(valid),x=x,y=y,exactFallback=bool(exact))
for a,b in pairs:
    assert cases[a]['result']['valid'] and cases[b]['result']['valid']
    assert [cases[a]['result'][k] for k in ['x','y']]==[cases[b]['result'][k] for k in ['x','y']],(a,b)
run('wasm-parity',['node','tools/verification/gradient-coordinate-parity.mjs'])
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
report=dict(format='musteroffice.gradient-coordinate-reference/1',commands=commands,cases=cases,
    counts=dict(total=len(cases),success=success,invalid=invalid,exactFallback=fallback,rigidPairs=len(pairs)),
    artifacts=[entry(root/n) for n in ['native-probe','sanitizer-probe','probe.mjs','probe.wasm','input.bin','native.bin','sanitizer.bin','wasm.bin']],
    scope='Standalone correctly rounded mapping against Python Fraction reference; production shader integration is separately required.')
(root/'coordinates.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report['counts']))
