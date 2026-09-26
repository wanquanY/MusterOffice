"""Display verified kernel pixels over a checkerboard; labels are diagnostic only."""
import hashlib,json
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
root=Path('.codex-work/gradient-raster')
records={c['name']:c for c in json.loads((root/'parity.json').read_text())['cases']}
names=['linear-clamp-srgb-straight','linear-clamp-srgb-premultiplied',
       'linear-clamp-linearSrgb-straight','linear-clamp-linearSrgb-premultiplied',
       'radial-clamp-srgb-straight','radial-repeat-srgb-straight',
       'radial-mirror-srgb-straight','radial-decal-srgb-straight']
image=Image.new('RGB',(1040,620),(245,246,249));draw=ImageDraw.Draw(image);font=ImageFont.load_default(size=16)
draw.text((24,18),'MusterOffice | actual kernel pixels | Native = WASM',fill=(20,25,35),font=font)
for i,name in enumerate(names):
    c=records[name];raw=Path(c['pixelsPath']).read_bytes();assert hashlib.sha256(raw).hexdigest()==c['pixelsSha256']
    q=json.loads(Path(c['requestPath']).read_text());w,h=q['viewport']['width'],q['viewport']['height'];pixels=bytearray(raw)
    for k in range(0,len(pixels),4):
        alpha=pixels[k+3]
        for channel in range(3):pixels[k+channel]=min(255,(pixels[k+channel]*255+alpha//2)//alpha) if alpha else 0
    actual=Image.frombytes('RGBA',(w,h),bytes(pixels)).resize((232,232),Image.Resampling.NEAREST)
    tile=Image.new('RGBA',actual.size,'white');td=ImageDraw.Draw(tile)
    for y in range(0,232,16):
        for x in range(0,232,16):
            if (x//16+y//16)%2:td.rectangle((x,y,x+15,y+15),fill=(225,228,234))
    tile.alpha_composite(actual);x,y=24+(i%4)*256,60+(i//4)*276;image.paste(tile.convert('RGB'),(x,y))
    label=name.replace('linear-clamp-','').replace('radial-','').replace('-srgb-straight','').replace('linearSrgb','linear sRGB')
    draw.text((x,y+240),label,fill=(20,25,35),font=font)
image.save(root/'preview.png')
print(json.dumps({'path':str(root/'preview.png'),'sha256':hashlib.sha256((root/'preview.png').read_bytes()).hexdigest(),'sourcePixelSha256':[records[n]['pixelsSha256'] for n in names]}))
