"""Label real kernel pixels; no alternate shape renderer is used."""
import hashlib,json,textwrap
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
root=Path('.codex-work/preset-expansion')
cases=[c for c in json.loads((root/'parity.json').read_text())['cases'] if c['kind']=='raster']
assert len(cases)==187
image=Image.new('RGB',(1320,2210),'#f2f4f7');draw=ImageDraw.Draw(image);font=ImageFont.load_default(size=10)
for i,c in enumerate(cases):
    raw=Path(c['pixelsPath']).read_bytes();assert hashlib.sha256(raw).hexdigest()==c['pixelsSha256']
    assert len(raw)==96*96*4 and all(raw[j]==255 for j in range(3,len(raw),4))
    tile=Image.frombytes('RGBA',(96,96),raw);x=(i%11)*120+12;y=(i//11)*130+8;image.paste(tile,(x,y))
    for line,label in enumerate(textwrap.wrap(c['name'].removesuffix('-square'),width=19)):
        draw.text((x,y+99+line*11),label,font=font,fill='#172230')
path=root/'preview.png';image.save(path)
result={'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'sourcePixelSha256':[c['pixelsSha256'] for c in cases]}
(root/'preview.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'path':str(path),'verifiedTiles':len(cases)}))
