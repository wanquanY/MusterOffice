"""Independent UCD reference, checked against runtime table and real Rust queries."""
import argparse,hashlib,json,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/itemization'));a=p.parse_args()
manifest=json.loads(Path('crates/mo-unicode/data/script-manifest.json').read_text());sources={}
for row in manifest['inputs']:
 raw=(a.directory/row['name']).read_bytes();assert len(raw)==row['byteLength'] and hashlib.sha256(raw).hexdigest()==row['sha256'];sources[row['name'].split('-')[0]]=raw.decode()
aliases={}
for line in sources['PropertyValueAliases'].splitlines():
 fields=[v.strip() for v in line.partition('#')[0].split(';')]
 if fields[0]=='sc':
  for value in fields[1:]:aliases[value]=fields[1]
tags=sorted(set(aliases.values()));ids={tag:i for i,tag in enumerate(tags)}
primary=bytearray([ids['Zzzz']])*0x110000
extensions=[None]*0x110000
for name,target in [('Scripts',primary),('ScriptExtensions',extensions)]:
 for line in sources[name].splitlines():
  fields=line.partition('#')[0].split(';')
  if len(fields)<2:continue
  extent=fields[0].strip().split('..');start,end=int(extent[0],16),int(extent[-1],16)
  value=ids[aliases[fields[1].strip()]] if name=='Scripts' else tuple(sorted(fields[1].split()))
  for cp in range(start,end+1):target[cp]=value
expected=[extensions[cp] or (tags[primary[cp]],) for cp in range(0x110000)]
data=Path('crates/mo-unicode/data/scripts.bin').read_bytes();assert data[:8]==b'MOSCR018';ns,nr,ne,nt=struct.unpack_from('<IIII',data,8);at=24
assert ns==len(tags) and data[at:at+ns*4].decode()==''.join(tags);at+=ns*4
actual=bytearray([ids['Zzzz']])*0x110000;last=-1
for i in range(nr):
 start,end,index=struct.unpack_from('<IIH',data,at+i*10);assert last<start<=end<0x110000 and index<ns;last=end;actual[start:end+1]=bytes([index])*(end-start+1)
assert actual==primary;at+=nr*10
extension_ranges=[struct.unpack_from('<IIH',data,at+i*10) for i in range(ne)];at+=ne*10
sets=[]
for i in range(nt):
 words=struct.unpack_from('<QQQQ',data,at+i*32);assert not any(words[bit//64]&(1<<(bit%64)) for bit in range(ns,256));sets.append(tuple(t for j,t in enumerate(tags) if words[j//64]&(1<<(j%64))))
assert len(data)==at+nt*32
actual_ext=[(tags[v],) for v in primary];last=-1
for start,end,index in extension_ranges:
 assert last<start<=end<0x110000 and index<nt;last=end;actual_ext[start:end+1]=[sets[index]]*(end-start+1)
assert actual_ext==expected
seen=bytearray(0x110000);ranges=0
for row in (a.directory/'native-properties.txt').read_text().splitlines():
 lo,hi,script,scx=row.split(';');lo,hi=int(lo,16),int(hi,16);value=tuple(scx.split());assert value==tuple(sorted(value));ranges+=1
 for cp in range(lo,hi+1):
  assert not 0xd800<=cp<=0xdfff and not seen[cp] and tags[primary[cp]]==script and expected[cp]==value,hex(cp);seen[cp]=1
assert sum(seen)==0x110000-0x800
representatives={}
for cp in range(0x110000):
 if 0xd800<=cp<=0xdfff:continue
 tag=tags[primary[cp]]
 if tag not in ('Zyyy','Zinh','Zzzz'):representatives.setdefault(tag,cp)
(a.directory/'script-representatives.json').write_text(json.dumps(representatives,sort_keys=True)+'\n')
result={'denseCodepointsCompared':0x110000,'nativeScalarsCompared':sum(seen),'nativePropertyRanges':ranges,'scripts':ns,'propertyRanges':nr,'extensionRanges':ne,'extensionSets':nt,'assignedExplicitScripts':len(representatives),'runtimeTableByteLength':len(data),'runtimeTableSha256':hashlib.sha256(data).hexdigest(),'nativePropertyDumpSha256':hashlib.sha256((a.directory/'native-properties.txt').read_bytes()).hexdigest()}
(a.directory/'script-properties.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
