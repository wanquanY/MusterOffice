"""Encode actual premultiplied pixel output for visual inspection, not rendering."""
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw

root=Path('.codex-work/stroke')
names=['stroke-cap-butt','stroke-cap-round','stroke-cap-square',
       'stroke-join-miter','stroke-join-round','stroke-join-bevel',
       'stroke-contour-open','stroke-contour-closed','stroke-fill-order-reset',
       'stroke-curve-0','stroke-curve-9','stroke-curve-28']
report=json.loads(Path('.codex-work/path-raster/parity.json').read_text())
cases={c['name']:c for c in report['cases']}
sheet=Image.new('RGB',(3*264,4*290),'white');labels=ImageDraw.Draw(sheet)
sources=[]
for i,name in enumerate(names):
    c=cases[name];raw=Path(c['pixelsPath']).read_bytes()
    assert hashlib.sha256(raw).hexdigest()==c['pixelsSha256']
    q=json.loads(Path(c['requestPath']).read_text());assert q['viewport']['width']==q['viewport']['height']==64
    # Composite premultiplied output directly onto white. Do not multiply twice.
    rgb=bytes(raw[j+k]+255-raw[j+3] for j in range(0,len(raw),4) for k in range(3))
    tile=Image.frombytes('RGB',(64,64),rgb).resize((256,256),Image.Resampling.NEAREST)
    x=(i%3)*264;y=(i//3)*290;sheet.paste(tile,(x,y+22));labels.text((x+4,y+5),name,fill='black')
    sources.append({'name':name,'pixelsSha256':c['pixelsSha256']})
path=root/'stroke-contact-sheet.png';sheet.save(path)
out={'scope':'4x nearest-neighbor display of actual CPU pixels on white; not a reference rasterizer.',
     'image':{'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()},'sources':sources}
(root/'previews.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out['image']))
