"""Lossless PNG packaging of actual page RGBA; no substitute renderer."""
import hashlib,json,struct,zlib
from pathlib import Path
root=Path('.codex-work/elliptic-source');out=root/'previews';out.mkdir(exist_ok=True)
records=[]
for p in sorted((root/'native-final').glob('*.rgba')):
    q=json.loads(p.with_suffix('.request.json').read_text());w,h=q['viewport']['width'],q['viewport']['height'];pixels=p.read_bytes();assert len(pixels)==w*h*4
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
    rows=b''.join(b'\0'+pixels[y*w*4:(y+1)*w*4] for y in range(h))
    b=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')
    path=out/(p.stem+'.png');path.write_bytes(b);records.append(dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest()))
(root/'previews.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(dict(images=len(records))))
