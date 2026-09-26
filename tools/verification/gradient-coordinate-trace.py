"""Trace the old real Skia inverse and the replacement numeric mapper."""
import hashlib,json,struct,subprocess
from fractions import Fraction as F
from pathlib import Path
root=Path('.codex-work/gradient-coordinates')
old=Path('.codex-work/elliptic-fast/component')
skia=old/'source-native/skia-8d6d37b063afe87fd361de55359fb4cb6b6f443c'
argv=['clang++','-std=c++20','-O3','-fno-exceptions','-fno-rtti','-ffp-contract=off','-DSK_DISABLE_TRACING',
      '-I',str(skia),'tools/verification/gradient-coordinate-skia-trace.cpp',str(skia/'out/mo/libskia.a'),'-o',str(root/'skia-trace')]
with (root/'trace-build.log').open('w') as log:subprocess.run(argv,stdout=log,stderr=subprocess.STDOUT,check=True)
p=subprocess.run([str(root/'skia-trace')],capture_output=True,check=True);assert not p.stderr
(root/'old-pipeline.txt').write_bytes(p.stdout)
def bits(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def value(bits):return struct.unpack('<f',struct.pack('<I',bits))[0]
def nearest(q):
    # All traced ratios are positive normal numbers. The nearby candidate is
    # chosen with Python arithmetic, then exact rational distances select it.
    candidate=bits(float(q));choices=range(candidate-1,candidate+2)
    return min(choices,key=lambda v:(abs(F(value(v))-q),v%2))
lines=[line.split() for line in p.stdout.decode().splitlines()];assert len(lines)==8
inputs=[]
for x,y,moved,_,_ in lines:
    x,y,moved=map(int,[x,y,moved]);m=[300,0,25+10*moved,0,150,50+20*moved]
    inputs.append([*map(bits,m),0,0,0,bits(1),bits(1),bits(x+.5+10*moved),bits(y+.5+20*moved)])
raw=struct.pack('<I',len(inputs))+b''.join(struct.pack('<13I',*i) for i in inputs)
p=subprocess.run([str(root/'native-probe')],input=raw,capture_output=True,check=True);assert not p.stderr
assert len(p.stdout)==16*len(inputs);records=[]
for i in range(0,len(lines),2):
    x,y=map(int,lines[i][:2]);before=list(map(lambda s:int(s,16),lines[i][3:]));after=list(map(lambda s:int(s,16),lines[i+1][3:]))
    assert before!=after
    expected=[nearest((F(2*x+1,2)-25)/300),nearest((F(2*y+1,2)-50)/150)]
    pairs=[struct.unpack_from('<4I',p.stdout,j*16) for j in [i,i+1]]
    for valid,u,v,_ in pairs:assert valid and [u,v]==expected
    records.append(dict(pixel=[x,y],oldBeforeBits=before,oldAfterBits=after,newBefore=list(pairs[0]),newAfter=list(pairs[1]),exactRoundedBits=expected))
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
(root/'trace.json').write_text(json.dumps(dict(format='musteroffice.gradient-coordinate-trace/1',buildCommand=argv,cases=records,
    oldSkiaLibrary=entry(skia/'out/mo/libskia.a'),oldBuild=entry(old/'native-build.json'),
    artifacts=[entry(root/n) for n in ['skia-trace','old-pipeline.txt','native-probe']],
    scope='Actual old Skia seed/inverse/callback pipeline reproduces all four translation-coordinate differences. Standalone new mapper matches exact rational rounding before/after; complete shader and pixel integration are tested separately.'),indent=2)+'\n')
print(json.dumps(dict(actualSkiaDifferences=len(records),newMapperExactPairs=len(records))))
