"""Diagnostic contact sheet of actual opaque kernel RGBA; not a product renderer."""
import hashlib,json
from pathlib import Path
from PIL import Image,ImageDraw,__version__ as pillow_version
root=Path('.codex-work/playback-render');report=json.loads((root/'product.json').read_text())
items=[('grouped-0','Grouped  t=0'),('grouped-1','Grouped  t=1/3'),('grouped-4','Grouped  t=1'),('nested-2','Nested  t=1001/30000'),('nested-3','Nested  t=11/13'),('nested-9','Nested  t=2')]
sheet=Image.new('RGBA',(960,528),'#e5e7eb');draw=ImageDraw.Draw(sheet)
for i,(name,label)in enumerate(items):
 c=next(c for c in report['cases']if c['name']==name);b=Path(c['pixels']['path']).read_bytes();assert hashlib.sha256(b).hexdigest()==c['pixels']['sha256'];assert all(v==255 for v in b[3::4]);image=Image.frombytes('RGBA',(320,240),b);x=i%3*320;y=i//3*264;sheet.paste(image,(x,y+24));draw.text((x+8,y+5),label,fill='black')
path=root/'preview.png';sheet.save(path);b=path.read_bytes();(root/'preview.json').write_text(json.dumps(dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest(),pillow=pillow_version,sourceCases=[v[0]for v in items],role='Diagnostic montage of exact opaque output bytes with labels outside each frame.'),indent=2)+'\n')
