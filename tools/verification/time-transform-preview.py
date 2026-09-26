"""Diagnostic montage of actual opaque returned pixels; no substitute renderer."""
import hashlib,json
from pathlib import Path
from PIL import Image,ImageDraw,__version__
root=Path('.codex-work/time-transform');product=json.loads((root/'product.json').read_text())
names=[f'{group}-{i}/retained' for group in ['forward-ease','reverse-repeat','auto-reverse','clipped-reverse'] for i in [1,5,10]];images=[];sources=[]
for name in names:
 c=next(c for c in product['cases']if c['name']==name);b=Path(c['pixels']['path']).read_bytes();assert hashlib.sha256(b).hexdigest()==c['pixels']['sha256'];meta=json.loads(Path(c['response']['path']).read_text());r=meta['info']['page']['page']['scene']['raster'];w,h=r['width'],r['height'];assert len(b)==w*h*4 and all(x==255 for x in b[3::4]);images.append(Image.frombytes('RGBA',(w,h),b));sources.append(c['pixels'])
w=max(i.width for i in images);h=max(i.height for i in images);canvas=Image.new('RGB',(w*3,(h+24)*4),'#dedede');draw=ImageDraw.Draw(canvas)
for i,(name,img)in enumerate(zip(names,images)):
 x=i%3*w;y=i//3*(h+24);draw.text((x+5,y+5),name,fill='black');canvas.paste(img,(x,y+24))
path=root/'preview.png';canvas.save(path);b=path.read_bytes();(root/'preview.json').write_text(json.dumps(dict(pillow=__version__,sources=sources,image=dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())),indent=2)+'\n');print(path)
