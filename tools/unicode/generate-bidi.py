"""Compile exact UCD18 bidi properties; no dependency's built-in Unicode tables.
Canonical bracket equivalence derives from UnicodeData, not host normalization.
"""
import argparse,array,hashlib,json,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/bidi'));p.add_argument('--check',action='store_true');a=p.parse_args()
manifest=json.loads(Path('crates/mo-unicode/data/bidi-manifest.json').read_text());inputs={}
for item in manifest['inputs']:
    raw=(a.directory/item['name']).read_bytes();assert len(raw)==item['byteLength'] and hashlib.sha256(raw).hexdigest()==item['sha256']
    inputs[item['name'].split('-')[0]]=raw.decode('utf8')
classes=['L','R','AL','EN','ES','ET','AN','CS','NSM','BN','B','S','WS','ON','LRE','LRO','RLE','RLO','PDF','LRI','RLI','FSI','PDI']
aliases={'Left_To_Right':'L','Right_To_Left':'R','Arabic_Letter':'AL','European_Terminator':'ET'}
values=array.array('B',[0])*0x110000
def fields(line):return [x.strip() for x in line.split('#',1)[0].split(';')]
def bounds(s):
    v=s.split('..');lo,hi=int(v[0],16),int(v[-1],16);assert 0<=lo<=hi<0x110000;return lo,hi
for line in inputs['DerivedBidiClass'].splitlines():
    if '@missing:' not in line:continue
    span,value=fields(line.split('@missing:',1)[1]);lo,hi=bounds(span);code=classes.index(aliases.get(value,value));values[lo:hi+1]=array.array('B',[code])*(hi+1-lo)
seen=set()
for line in inputs['DerivedBidiClass'].splitlines():
    f=fields(line)
    if not f[0]:continue
    lo,hi=bounds(f[0]);code=classes.index(f[1])
    for cp in range(lo,hi+1):assert cp not in seen;seen.add(cp);values[cp]=code
decompositions={};mirrored=set()
for line in inputs['UnicodeData'].splitlines():
    f=line.split(';');cp=int(f[0],16)
    if f[9]=='Y':
        assert not f[1].endswith((', First>',', Last>')),'mirrored ranges need explicit expansion'
        mirrored.add(cp);values[cp]|=32
    if f[5] and not f[5].startswith('<'):decompositions[cp]=[int(v,16) for v in f[5].split()]
def canonical(cp):
    visited=set()
    while cp in decompositions:
        assert cp not in visited;visited.add(cp);value=decompositions[cp];assert len(value)==1;cp=value[0]
    return cp
brackets=[];bracket_map={}
for line in inputs['BidiBrackets'].splitlines():
    f=fields(line)
    if not f[0]:continue
    cp,pair=int(f[0],16),int(f[1],16);assert f[2] in ['o','c'];is_open=f[2]=='o';assert cp not in bracket_map
    assert values[cp]&31==classes.index('ON') and cp in mirrored
    bracket_map[cp]=(pair,is_open);brackets.append((cp,pair,canonical(cp if is_open else pair),is_open))
for cp,(pair,is_open) in bracket_map.items():assert bracket_map[pair]==(cp,not is_open)
mirrors=[]
for line in inputs['BidiMirroring'].splitlines():
    f=fields(line)
    if not f[0]:continue
    cp,pair=int(f[0],16),int(f[1],16);assert cp in mirrored;mirrors.append((cp,pair))
brackets.sort();mirrors.sort();assert len(dict(mirrors))==len(mirrors)
ranges=[];start=0
for cp in range(1,0x110001):
    if cp==0x110000 or values[cp]!=values[start]:
        if values[start]:ranges.append((start,cp-1,values[start]))
        start=cp
raw=b'MOBDI018'+struct.pack('<III',len(ranges),len(brackets),len(mirrors))+b''.join(struct.pack('<IIB',*r) for r in ranges)+b''.join(struct.pack('<IIIB',*b) for b in brackets)+b''.join(struct.pack('<II',*m) for m in mirrors)
target=Path('crates/mo-unicode/data/bidi.bin')
if a.check:assert target.read_bytes()==raw
else:target.write_bytes(raw)
print(json.dumps({'ranges':len(ranges),'brackets':len(brackets),'mirroringMappings':len(mirrors),'mirroredCodepoints':len(mirrored),'byteLength':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}))
