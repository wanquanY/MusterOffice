"""Prepared geometry classification and root intervals against exact reference."""
import json,os,struct,subprocess,random
from pathlib import Path
from fractions import Fraction as F
from elliptic_polynomial_reference import inside,poly,sturm,at,variations
root=Path('.codex-work/elliptic-fast/prepared');root.mkdir(exist_ok=True)
common=['-std=c++20','-O3','-Wall','-Wextra','-Werror','-fno-exceptions','-fno-rtti','-ffp-contract=off',
 '-Icomponents/skia','components/skia/mo_elliptic_field.cpp','components/skia/mo_elliptic_prepared.cpp','tools/verification/elliptic-prepared-probe.cpp']
commands=[]
def run(name,args,env=None):
    p=root/(name+'.log')
    with p.open('w') as log:r=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT)
    commands.append(dict(name=name,argv=args,exitCode=r.returncode,log=str(p)))
    assert r.returncode==0,p.read_text()
run('native-build',['clang++',*common,'-o',str(root/'native')])
run('sanitizer-build',['clang++',*common,'-fsanitize=address,undefined','-fno-sanitize-recover=all','-o',str(root/'sanitizer')])
env=dict(os.environ,EM_CONFIG=str(Path('.codex-work/emsdk/.emscripten').resolve()))
run('wasm-build',['.codex-work/emsdk/upstream/emscripten/em++',*common,'--no-entry',
 '-sMODULARIZE=1','-sEXPORT_ES6=1','-sENVIRONMENT=node,web','-sFILESYSTEM=0','-sALLOW_MEMORY_GROWTH=1',
 '-sDYNAMIC_EXECUTION=0','-sEXPORTED_FUNCTIONS=["_mo_elliptic_prepared_probe","_malloc","_free"]',
 '-sEXPORTED_RUNTIME_METHODS=["HEAPU32"]','-o',str(root/'probe.mjs')],env)
prior=json.loads(Path('.codex-work/elliptic-scalar/cases.json').read_text())
cases=[dict(name=c['name'],raw=struct.pack('<7I',*c['words'],c['budget']),required=c['required']) for c in prior]
r=random.Random(321)
for radius in [0,2**-149,2**-32,.125,.3,.5,1-2**-24,1,1+2**-23,2,32768]:
    for i in range(128):
        x,y=(r.uniform(-1,1),r.uniform(-1,1)) if i>7 else [(0,0),(1,0),(0,1),(-1,0),(0,-1),(1,1),(2**-149,0),(.5,0)][i]
        cases.append(dict(name=f'concentric-{radius}-{i}',raw=struct.pack('<6fI',x,y,0,0,radius,radius,512),required=None))
# Nested, near-certificate and deliberately non-nested anisotropic fields.
# Rounded endpoint neighbors stress exact inclusion and tiny first roots.
fields=[(0,0,.3,.8),(.2,-.15,.25,.6),(.6,-.2,0,0),(.4,0,.3,.3),
 (0,0,0,.6),(0,0,.6,0),(0,0,2**-149,.5),(2**-20,0,2**-40,.5),
 (.1,0,2**-40,.9),(.2,.2,.05,.95),(.25,.1,.8,.1),(.5,.5,0,0),
 (0,0,1-2**-24,.125),(0,0,1,1),(0,0,2,1.5),(.5,0,.5,.5),
 (.5-2**-25,0,.5,.5),(.5+2**-24,0,.5,.5),(.9,.2,.3,.8),
 (-.9715088489324577,-2.0813371009894306,.02605792474208458,1.8864963402571122)]
for k,f in enumerate(fields):
    for i in range(192):
        if i<128:x,y=r.uniform(-1,1),r.uniform(-1,1)
        else:
            points=[(0,0),(1,0),(-1,0),(0,1),(0,-1),(2**-149,0),(0,2**-149),(.5,.5),
                    (f[0]+f[2],f[1]),(f[0]-f[2],f[1]),(f[0],f[1]+f[3]),(f[0],f[1]-f[3])]
            x,y=points[(i-128)%len(points)]
            x=max(-1,min(1,x));y=max(-1,min(1,y))
        cases.append(dict(name=f'field-{k}-{i}',raw=struct.pack('<6fI',x,y,*f,512),required=None))
data=b''.join(c['raw'] for c in cases);(root/'input.bin').write_bytes(data)
def probe(name):
    r=subprocess.run([str(root/name)],input=struct.pack('<I',len(cases))+data,capture_output=True)
    assert r.returncode==0 and r.stderr==b'',(r.returncode,r.stderr)
    (root/(name+'.bin')).write_bytes(r.stdout);return r.stdout
native=probe('native');assert probe('sanitizer')==native
# The general solver's cached radius arithmetic must preserve its previous
# exact operation results. Compare all prior packed outputs, not only pixels.
scalar_common=[v for v in common if v not in ['components/skia/mo_elliptic_prepared.cpp','tools/verification/elliptic-prepared-probe.cpp']]
run('scalar-build',['clang++',*scalar_common,'tools/verification/elliptic-scalar-probe.cpp','-o',str(root/'scalar')])
scalar=probe('scalar')
previous=Path('.codex-work/elliptic-scalar/native.bin').read_bytes()
assert scalar[:len(previous)]==previous

results=[];statuses={}
for i,c in enumerate(cases):
    status,location,value,lo,hi,nodes,degree,fast=struct.unpack('<IIfddIII',native[40*i:40*(i+1)])
    v=struct.unpack('<6fI',c['raw']);values=v[:6]
    baseline_status=struct.unpack_from('<I',scalar,40*i)[0]
    if baseline_status==0:assert status==0,(c['name'],'lost scalar success')
    if c['required'] is not None:assert status==c['required'],(c['name'],status)
    statuses[str(status)]=statuses.get(str(status),0)+1
    if status==0:
        assert 0<=lo<=value<=hi<=1 and F(hi)-F(lo)<=F(1,2**23)
        if location==0:assert inside(values) and value==lo==hi==0,c['name']
        else:
            assert not inside(values),c['name'];p=poly(values);s=sturm(p);start=variations(s,F(0))
            if location==2:assert value==lo==hi==1 and start==variations(s,F(1)),c['name']
            else:
                assert location==1
                assert start-variations(s,F(lo))-(at(p,F(lo))==0)==0,c['name']
                assert start-variations(s,F(hi))>=1,c['name']
    results.append(dict(name=c['name'],status=status,location=location,nodes=nodes,fast=bool(fast)))
run('wasm-parity',['node','tools/verification/elliptic-fast-parity.mjs'])
(root/'verification.json').write_text(json.dumps(dict(format='musteroffice.elliptic-fast-prepared/1',priorScalarByteIdentical=len(previous)//40,commands=commands,cases=len(cases),statuses=statuses,results=results),indent=2)+'\n')
print(json.dumps(dict(cases=len(cases),statuses=statuses)))
