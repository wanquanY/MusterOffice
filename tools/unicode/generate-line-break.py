"""Compile pinned UCD18 line-break and auxiliary properties, without host tables."""
import argparse,array,hashlib,json,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/line-break'));p.add_argument('--check',action='store_true');a=p.parse_args()
manifest=json.loads(Path('crates/mo-unicode/data/line-break-manifest.json').read_text());inputs={}
for item in manifest['inputs']:
    raw=(a.directory/item['name']).read_bytes();assert len(raw)==item['byteLength'] and hashlib.sha256(raw).hexdigest()==item['sha256']
    inputs[item['name'].split('-')[0]]=raw.decode('utf8')
classes=['XX','AI','AK','AL','AP','AS','B2','BA','BB','BK','CB','CJ','CL','CM','CP','CR','EB','EM','EX','GL','H2','H3','HH','HL','HY','ID','IN','IS','JL','JT','JV','LF','NL','NS','NU','OP','PO','PR','QU','RI','SA','SG','SP','SY','VF','VI','WJ','ZW','ZWJ']
assert len(classes)<64
def entries(name):
    for line in inputs[name].splitlines():
        line=line.split('#')[0].strip()
        if not line:continue
        span,value=map(str.strip,line.split(';'));v=span.split('..');lo,hi=int(v[0],16),int(v[-1],16)
        assert 0<=lo<=hi<0x110000
        yield lo,hi,value
values=array.array('H',[512])*0x110000 # XX and General_Category=Cn
for name in ['LineBreak','DerivedGeneralCategory','EastAsianWidth']:
    seen=bytearray(0x110000)
    for lo,hi,value in entries(name):
        assert not any(seen[lo:hi+1]);seen[lo:hi+1]=bytes([1])*(hi-lo+1)
        if name=='LineBreak':mask,bits=63,classes.index(value)
        elif name=='DerivedGeneralCategory':mask,bits=960,{'Mn':64,'Mc':64,'Pi':128,'Pf':256,'Cn':512}.get(value,0)
        else:mask,bits=1024,1024 if value in ['F','W','H'] else 0
        for cp in range(lo,hi+1):values[cp]=(values[cp]&~mask)|bits
ranges=[];start=0
for cp in range(1,0x110001):
    if cp==0x110000 or values[cp]!=values[start]:
        if values[start]!=512:ranges.append((start,cp-1,values[start]))
        start=cp
raw=b'MOLBR018'+struct.pack('<I',len(ranges))+b''.join(struct.pack('<IIH',*r) for r in ranges)
target=Path('crates/mo-unicode/data/line-break.bin')
if a.check:assert target.read_bytes()==raw
else:target.write_bytes(raw)
print(json.dumps({'classes':classes,'ranges':len(ranges),'byteLength':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}))
