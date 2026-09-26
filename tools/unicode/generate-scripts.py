"""Generate fixed Unicode Script/Script_Extensions data; no host Unicode API."""
import argparse,hashlib,json,re,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--directory',type=Path,default=Path('.codex-work/itemization'));p.add_argument('--check',action='store_true');a=p.parse_args()
manifest=json.loads(Path('crates/mo-unicode/data/script-manifest.json').read_text());texts={}
for row in manifest['inputs']:
 b=(a.directory/row['name']).read_bytes();assert len(b)==row['byteLength'] and hashlib.sha256(b).hexdigest()==row['sha256'];texts[row['name'].split('-')[0]]=b.decode()
aliases={}
for raw in texts['PropertyValueAliases'].splitlines():
 f=[x.strip() for x in raw.partition('#')[0].split(';')]
 if f[0]=='sc':
  for value in f[1:]:assert aliases.get(value,f[1])==f[1];aliases[value]=f[1]
tags=sorted(set(aliases.values()));assert len(tags)<=256 and all(re.fullmatch('[A-Z][a-z]{3}',t) for t in tags)
ids={t:i for i,t in enumerate(tags)}
def records(text):
 for raw in text.splitlines():
  line=raw.partition('#')[0].strip()
  if not line:continue
  extent,value=[x.strip() for x in line.split(';')];span=extent.split('..');lo=int(span[0],16);hi=int(span[-1],16)
  assert 0<=lo<=hi<0x110000;yield lo,hi,value
primary=[ids['Zzzz']]*0x110000;covered=set()
for lo,hi,name in records(texts['Scripts']):
 assert not any(cp in covered for cp in range(lo,hi+1));covered.update(range(lo,hi+1));primary[lo:hi+1]=[ids[aliases[name]]]*(hi-lo+1)
ranges=[];start=0
for cp in range(1,0x110001):
 if cp==0x110000 or primary[cp]!=primary[start]:
  if primary[start]!=ids['Zzzz']:ranges.append((start,cp-1,primary[start]))
  start=cp
sets=[];extensions=[];previous=-1
for lo,hi,value in records(texts['ScriptExtensions']):
 assert lo>previous;previous=hi;values=tuple(sorted(ids[t] for t in value.split()));assert len(values)==len(set(values)) and values
 assert not any(tags[i] in ['Zyyy','Zinh','Zzzz'] for i in values)
 for cp in range(lo,hi+1):assert tags[primary[cp]] in ['Zyyy','Zinh'] or primary[cp] in values
 if values not in sets:sets.append(values)
 extensions.append((lo,hi,sets.index(values)))
data=b'MOSCR018'+struct.pack('<IIII',len(tags),len(ranges),len(extensions),len(sets))+''.join(tags).encode()
data+=b''.join(struct.pack('<IIH',*r) for r in ranges+extensions)
for values in sets:
 words=[0]*4
 for i in values:words[i//64]|=1<<(i%64)
 data+=struct.pack('<QQQQ',*words)
output=Path('crates/mo-unicode/data/scripts.bin')
if a.check:assert output.read_bytes()==data
else:output.write_bytes(data)
print(json.dumps({'scripts':len(tags),'ranges':len(ranges),'extensionRanges':len(extensions),'extensionSets':len(sets),'byteLength':len(data),'sha256':hashlib.sha256(data).hexdigest()}))
