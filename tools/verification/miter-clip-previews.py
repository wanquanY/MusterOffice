"""Compose diagnostic views from verified renderer pixels, without redrawing."""
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw

root=Path('.codex-work/miter-clip')
lookup={c['name']:c for c in json.loads(Path('.codex-work/path-raster/parity.json').read_text())['cases']}
names=['clip-v-16-24-1','clip-v-16-24-1.25','clip-v-16-24-2','clip-v-16-24-4','clip-v-16-24-16',
       'clip-rectangle','clip-short','clip-reversal','clip-quadratic-0','clip-cubic-1']
canvas=Image.new('RGB',(1040,456),'#ffffff');labels=ImageDraw.Draw(canvas);sources=[]
for i,name in enumerate(names):
    c=lookup[name];b=Path(c['pixelsPath']).read_bytes()
    assert hashlib.sha256(b).hexdigest()==c['pixelsSha256']
    # These probes are straight red over transparent; recover straight channels
    # before normal alpha compositing so AA premultiplication is not applied twice.
    pixels=bytearray(b)
    for j in range(0,len(pixels),4):
        if pixels[j+3]:
            for k in range(3):pixels[j+k]=round(pixels[j+k]*255/pixels[j+3])
    image=Image.frombytes('RGBA',(192,192),bytes(pixels));x=(i%5)*208;y=(i//5)*228
    canvas.paste(image,(x+8,y+24),image)
    labels.text((x+8,y+5),name.replace('clip-',''),fill='#111111')
    sources.append({'name':name,'pixelsSha256':c['pixelsSha256']})
path=root/'preview.png';canvas.save(path)
entry={'path':str(path),'byteLength':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
(root/'previews.json').write_text(json.dumps({'image':entry,'sources':sources},indent=2)+'\n')
print(json.dumps(entry))
