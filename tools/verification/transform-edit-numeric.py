"""Diagnose a remaining rigid-transform precision gap; never label it a pass."""
import hashlib,json,struct
from pathlib import Path
root=Path('.codex-work/transform-edit')
def entry(p):
    b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def f32(v):return struct.unpack('<f',struct.pack('<f',v))[0]
def words(r):
    assert entry(r['path'])==r;b=Path(r['path']).read_bytes();return struct.unpack('<'+'I'*(len(b)//4),b)
def floats(r):
    b=Path(r['path']).read_bytes();return struct.unpack('<'+'f'*(len(b)//4),b)
report=json.loads((root/'render.json').read_text());case=next(c for c in report['cases'] if c['name']=='off-center-translate')
before=words(case['before']['frame']);after=words(case['frame'])
diffs=[i for i,(x,y) in enumerate(zip(before,after)) if x!=y]
# This owned one-object V12 frame: the only edits are paint and path origins.
assert len(before)==len(after) and diffs==[95,98,126,127]
b=floats(case['before']['frame']);a=floats(case['frame'])
assert b[93:99]==(300,0,25,0,150,50) and a[93:99]==(300,0,35,0,150,70)
samples=[]
for e in case['examples']:
    x,y=e['x']+0.5,e['y']+0.5
    def inverse(px,py,m):
        return [f32(f32(px*f32(1/m[0]))+f32(-m[2]/m[0])),
                f32(f32(py*f32(1/m[4]))+f32(-m[5]/m[4]))]
    old=inverse(x-10,y-20,b[93:99]);new=inverse(x,y,a[93:99])
    # Subtraction before scaling is exactly translation invariant for these
    # integer origins/half-pixel samples. This is a diagnostic, not a new shader.
    rebased_old=[f32(f32(x-10-25)*f32(1/300)),f32(f32(y-20-50)*f32(1/150))]
    rebased_new=[f32(f32(x-35)*f32(1/300)),f32(f32(y-70)*f32(1/150))]
    assert rebased_old==rebased_new
    samples.append(dict(pixel=e,expandedInverseBefore=old,expandedInverseAfter=new,
        expandedInverseDiffers=old!=new,originRelativeBefore=rebased_old,originRelativeAfter=rebased_new))
assert any(s['expandedInverseDiffers'] for s in samples)
result=dict(format='musteroffice.transform-edit-numeric-finding/1',status='unresolved-rendering-precision',
    renderReport=entry(root/'render.json'),frameBefore=case['before']['frame'],frameAfter=case['frame'],changedWordIndices=diffs,samples=samples,
    shaderSource=entry('components/skia/mo_gradient_plane.cpp'),
    interpretation='Source edit produced only the requested paint/path origin displacement; all field parameters stayed identical. Independently expanded float32 inverse mapping is position dependent. The shader uses a SkMatrix local inverse path; exact tracing and a production fix remain necessary.',
    notProven='This scalar diagnostic does not execute the Skia vector stage and does not claim to explain every channel discrepancy or validate a replacement renderer.')
(root/'numeric.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(status=result['status'],samples=len(samples),expandedInverseDifferences=sum(s['expandedInverseDiffers'] for s in samples))))
