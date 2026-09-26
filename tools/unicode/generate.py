"""Compile pinned Unicode 18 property facts into nonoverlapping little-endian ranges.
No upstream implementation code is copied. The accompanying Unicode notice is retained.
"""
import argparse,array,hashlib,json,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/unicode'));p.add_argument('--check',action='store_true');a=p.parse_args()
manifest=json.loads(Path('crates/mo-unicode/data/manifest.json').read_text())
inputs={}
for item in manifest['inputs']:
    data=(a.directory/item['name']).read_bytes();assert len(data)==item['byteLength'] and hashlib.sha256(data).hexdigest()==item['sha256']
    inputs[item['name']]=data.decode('utf8')
gcb={name:i for i,name in enumerate(['Other','CR','LF','Control','Extend','ZWJ','Regional_Indicator','Prepend','SpacingMark','L','V','T','LV','LVT'])}
incb={'None':0,'Consonant':1,'Extend':2,'Linker':3}
values=array.array('H',[0])*0x110000
def rows(name):
    for line in inputs[name].splitlines():
        body=line.split('#',1)[0].strip()
        if not body:continue
        fields=[s.strip() for s in body.split(';')];bounds=fields[0].split('..');start=int(bounds[0],16);end=int(bounds[-1],16)
        assert 0<=start<=end<0x110000
        yield start,end,fields[1:]
def assign(start,end,value,mask):
    for cp in range(start,end+1):
        assert values[cp]&mask==0,hex(cp)
        values[cp]|=value
for start,end,fields in rows('GraphemeBreakProperty-18.0.0.txt'):assign(start,end,gcb[fields[0]],15)
for start,end,fields in rows('DerivedCoreProperties-18.0.0.txt'):
    if fields[0]=='InCB':assign(start,end,incb[fields[1]]<<4,48)
    elif fields[0]=='Default_Ignorable_Code_Point':assign(start,end,128,128)
for start,end,fields in rows('emoji-data-18.0.0.txt'):
    if fields[0]=='Extended_Pictographic':assign(start,end,64,64)
for start,end,fields in rows('PropList-18.0.0.txt'):
    if fields[0]=='Variation_Selector':assign(start,end,256,256)
records=[];start=0
for cp in range(1,0x110001):
    if cp==0x110000 or values[cp]!=values[start]:
        if values[start]:records.append((start,cp-1,values[start]))
        start=cp
data=b'MOUCD018'+struct.pack('<I',len(records))+b''.join(struct.pack('<IIH',*r) for r in records)
target=Path('crates/mo-unicode/data/properties.bin')
if a.check:assert target.read_bytes()==data,'generated Unicode table changed'
else:target.write_bytes(data)
print(json.dumps({'ranges':len(records),'byteLength':len(data),'sha256':hashlib.sha256(data).hexdigest(),'unicodeVersion':manifest['unicodeVersion']}))
