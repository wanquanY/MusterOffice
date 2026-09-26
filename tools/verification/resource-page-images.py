"""Generate original tiny RGBA PNG inputs for source page composition tests."""
import json, struct, zlib, hashlib
from pathlib import Path
root=Path('fixtures/presentations/resource-page')
def chunk(t,b): return struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b))
def png(p):
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0'+p[:8]+b'\0'+p[8:]))+chunk(b'IEND',b'')
records=[]
for name, values in [('transparent',[255,0,0,255, 0,0,0,0, 0,0,255,255, 255,255,0,255]),('cyan',[0,255,255,255]*4)]:
 data=png(bytes(values));path=root/(name+'.png');path.write_bytes(data)
 records.append(dict(path=str(path),sha256=hashlib.sha256(data).hexdigest(),byteLength=len(data),width=2,height=2,rgba=values))
(root/'images.json').write_text(json.dumps(dict(ownership='Original programmatically generated test patterns; no external artwork.',generator='tools/verification/resource-page-images.py',images=records),indent=2)+'\n')
