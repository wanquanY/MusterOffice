"""Inspect actual native raster frames; no replacement shape renderer."""
import hashlib,json
from pathlib import Path
from PIL import Image,ImageDraw
ROOT=Path('.codex-work/source-placement')
r=json.loads((ROOT/'parity.json').read_text());cases=[c for c in r['cases'] if c['kind']=='raster'];preview=Image.new('RGB',(1280,4*205),'#e5e5e5');draw=ImageDraw.Draw(preview)
records=[]
for i,c in enumerate(cases):
    raw=Path(c['pixelsPath']).read_bytes();assert hashlib.sha256(raw).hexdigest()==c['pixelsSha256'];im=Image.frombytes('RGBA',(800,450),raw).convert('RGB');x=i%4*320;y=i//4*205
    preview.paste(im.resize((320,180)),(x,y));draw.text((x+3,y+183),c['name'],fill='black')
    red=[(x,y) for y in range(450) for x in range(800) if im.getpixel((x,y))==(219,35,75)]
    records.append({'name':c['name'],'pixelsSha256':c['pixelsSha256'],'opaqueRedBounds':[min(x for x,y in red),min(y for x,y in red),max(x for x,y in red)+1,max(y for x,y in red)+1] if red else None})
path=ROOT/'preview.png';preview.save(path)
(ROOT/'preview.json').write_text(json.dumps({'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'frames':records},indent=2)+'\n');print(len(records))
