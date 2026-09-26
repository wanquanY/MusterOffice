"""Independent dense reference vs compiled table and actual Rust scalar queries."""
import hashlib,json,struct
from pathlib import Path
root=Path('.codex-work/line-break');sources={}
for r in json.loads(Path('crates/mo-unicode/data/line-break-manifest.json').read_text())['inputs']:
 raw=(root/r['name']).read_bytes();assert len(raw)==r['byteLength'] and hashlib.sha256(raw).hexdigest()==r['sha256'];sources[r['name'].split('-')[0]]=raw.decode()
values=['XX']*0x110000;flags=bytearray([8])*0x110000
for name in ['LineBreak','DerivedGeneralCategory','EastAsianWidth']:
 for raw in sources[name].splitlines():
  line=raw.partition('#')[0].strip()
  if not line:continue
  extent,prop=map(str.strip,line.split(';'));bounds=extent.split('..');lo,hi=int(bounds[0],16),int(bounds[-1],16)
  if name=='LineBreak':values[lo:hi+1]=[prop]*(hi-lo+1)
  elif name=='DerivedGeneralCategory':flags[lo:hi+1]=bytes([{'Mn':1,'Mc':1,'Pi':2,'Pf':4,'Cn':8}.get(prop,0)])*(hi-lo+1)
  elif prop in ['F','W','H']:
   for cp in range(lo,hi+1):flags[cp]|=16
classes=['XX','AI','AK','AL','AP','AS','B2','BA','BB','BK','CB','CJ','CL','CM','CP','CR','EB','EM','EX','GL','H2','H3','HH','HL','HY','ID','IN','IS','JL','JT','JV','LF','NL','NS','NU','OP','PO','PR','QU','RI','SA','SG','SP','SY','VF','VI','WJ','ZW','ZWJ']
data=Path('crates/mo-unicode/data/line-break.bin').read_bytes();assert data[:8]==b'MOLBR018';nr=struct.unpack_from('<I',data,8)[0];assert len(data)==12+nr*10
actual=['XX']*0x110000;actual_flags=bytearray([8])*0x110000;last=-1;probes={0,0x10ffff}
for index in range(nr):
 lo,hi,value=struct.unpack_from('<IIH',data,12+index*10);assert last<lo<=hi<0x110000 and value&63<len(classes) and value<2048;last=hi
 actual[lo:hi+1]=[classes[value&63]]*(hi-lo+1);actual_flags[lo:hi+1]=bytes([value>>6])*(hi-lo+1)
 probes.update([lo,hi,max(0,lo-1),min(0x10ffff,hi+1)])
assert actual==values and actual_flags==flags
seen=bytearray(0x110000);native_ranges=0
for row in (root/'native-properties.txt').read_text().splitlines():
 lo,hi,value,bits=row.split(';');lo,hi=int(lo,16),int(hi,16);bits=int(bits);native_ranges+=1
 for cp in range(lo,hi+1):
  assert not 0xd800<=cp<=0xdfff and not seen[cp] and values[cp]==value and flags[cp]==bits,hex(cp);seen[cp]=1
assert sum(seen)==0x110000-0x800
probes=sorted(cp for cp in probes if not 0xd800<=cp<=0xdfff)
queries=[{'codepoint':cp,'properties':{'class':values[cp],'combiningMark':bool(flags[cp]&1),'initialPunctuation':bool(flags[cp]&2),'finalPunctuation':bool(flags[cp]&4),'unassigned':bool(flags[cp]&8),'eastAsian':bool(flags[cp]&16)}} for cp in probes]
(root/'property-probes.json').write_text(json.dumps(queries)+'\n')
result={'denseCodepointsCompared':0x110000,'nativeScalarsCompared':sum(seen),'nativePropertyRanges':native_ranges,'classes':len(classes),'propertyRanges':nr,'propertyProbes':len(probes),'runtimeTableByteLength':len(data),'runtimeTableSha256':hashlib.sha256(data).hexdigest(),'nativePropertyDumpSha256':hashlib.sha256((root/'native-properties.txt').read_bytes()).hexdigest()}
(root/'properties.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
