"""Owned PNG/JPEG metadata corpus; geometry expectations use Python Fraction.

Pixel expectations reuse the independently specified samples of the codec
stage, not decoder output. No private image, system font or network input.
"""
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import zlib

root = Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=True)
encoder = sys.argv[2]
cases = []
png_pixels = bytes([255,0,0,255, 0,128,0,128, 0,0,0,0,
                    0,0,64,64, 78,63,47,200, 230,170,80,255])
raw = b'\0' + bytes([255,0,0,255, 0,255,0,128, 10,20,30,0])
raw += b'\0' + bytes([0,0,255,64, 100,80,60,200, 230,170,80,255])
jpeg_pixels = bytes([40,40,40,255])*64 + bytes([180,180,180,255])*64


def chunk(name, data):
    return struct.pack('>I',len(data)) + name + data + struct.pack('>I',zlib.crc32(name+data))


def exif(x=None, y=None, unit=None, orientation=1, little=True):
    order = '<' if little else '>'
    tags = [(0x112,3,orientation)]
    if x is not None: tags.append((0x11a,5,x))
    if y is not None: tags.append((0x11b,5,y))
    if unit is not None: tags.append((0x128,3,unit))
    table = (b'II' if little else b'MM') + struct.pack(order+'HIH',42,8,len(tags))
    data = b''
    for tag, kind, value in tags:
        field = struct.pack(order+'HH',value,0) if kind==3 else struct.pack(order+'I',14+12*len(tags)+len(data))
        if kind==5: data += struct.pack(order+'II',*value)
        table += struct.pack(order+'HHI',tag,kind,1)+field
    return table + bytes(4) + data


def declaration(source, x, y, unit):
    d = lambda v: None if v is None else dict(numerator=v[0],denominator=v[1])
    return dict(source=source,x=d(x),y=d(y),unit=unit)


def expectation(declarations, orientation):
    ratios, absolute = [], []
    zero = False
    for d in declarations:
        density = lambda v: Fraction(v['numerator'],v['denominator']) if v else Fraction(72)
        x,y = density(d['x']),density(d['y'])
        unit = d['unit'] or 'inch'  # Only EXIF permits omitted fields.
        if not x or not y:
            zero=True
            continue
        ratios.append(x/y)
        if unit != 'aspectRatio':
            emu = dict(inch=914400,centimetre=360000,metre=36000000)[unit]
            absolute.append((emu/x,emu/y))
    if len(set(ratios))>1 or len(set(absolute))>1: size=dict(status='conflicting')
    elif zero: size=dict(status='zeroDensity')
    elif absolute:
        x,y=absolute[0]
        if orientation>=5: x,y=y,x
        encode=lambda v: dict(numerator=str(v.numerator),denominator=v.denominator)
        size=dict(status='known',x=encode(x),y=encode(y))
    else: size=dict(status='unspecified')
    return dict(declarations=declarations,physicalPixelSize=size)


def add(name, encoded, pixels, w, h, orientation, declarations, code=None):
    path=root/(name+'.bin'); path.write_bytes(encoded)
    item=dict(name=name,path=str(path),sha256=hashlib.sha256(encoded).hexdigest(),code=code)
    if not code:
        width,height=(h,w) if orientation>=5 else (w,h)
        normalized=bytearray(len(pixels))
        for y in range(h):
            for x in range(w):
                dx,dy=[(x,y),(w-1-x,y),(w-1-x,h-1-y),(x,h-1-y),
                       (y,x),(h-1-y,x),(h-1-y,w-1-x),(y,w-1-x)][orientation-1]
                normalized[(dy*width+dx)*4:(dy*width+dx+1)*4]=pixels[(y*w+x)*4:(y*w+x+1)*4]
        p=root/(name+'.rgba');p.write_bytes(normalized)
        item.update(expected=str(p),width=width,height=height,orientation=orientation,
                    tolerance=0,resolution=expectation(declarations,orientation))
    cases.append(item)


def png(name, parts=(), orientation=1, code=None, late=False):
    b=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',3,2,8,6,0,0,0))
    declarations=[]
    if late: b+=chunk(b'IDAT',zlib.compress(raw))
    for tag,body,d in parts:
        if d: declarations.append(dict(d,sourceOffset=len(b)))
        b+=chunk(tag,body)
    if not late: b+=chunk(b'IDAT',zlib.compress(raw))
    b+=chunk(b'IEND',b'')
    add(name,b,png_pixels,3,2,orientation,declarations,code)


def phys(x,y,unit):
    return b'pHYs',struct.pack('>IIB',x,y,unit),declaration('pngPhysical',(x,1),(y,1),'metre' if unit else 'aspectRatio')


def exif_part(x=None,y=None,unit=None,orientation=1,little=True):
    return b'eXIf',exif(x,y,unit,orientation,little),declaration('exifIfd0',x,y,{None:None,2:'inch',3:'centimetre'}[unit])


png('png-plain')
for x,y,unit in [(3779,3779,1),(10000,20000,1),(0xffffffff,1,1),(2,1,0),(0,1,1)]:
    png(f'png-physical-{x}-{y}-{unit}',[phys(x,y,unit)])
for little in [False,True]:
    for unit in [2,3]:
        for orientation in range(1,9):
            png(f'png-exif-{little}-{unit}-{orientation}',[exif_part((1200,4),(60,1),unit,orientation,little)],orientation)
for name,args in [('all',{}),('unit',dict(x=(300,1),y=(300,1))),
                  ('x',dict(y=(300,1),unit=2)),('y',dict(x=(300,1),unit=2))]:
    png('png-default-'+name,[exif_part(**args)])
png('png-exif-large',[exif_part((1,0xffffffff),(0xffffffff,0xffffffff),3)])
png('png-exif-zero',[exif_part((0,1),(300,1),2)])
for ppm in [10000,9999]:
    png(f'png-consensus-{ppm}',[phys(ppm,ppm,1),exif_part((508,2),(254,1),2)])
png('png-default-conflict',[phys(10000,10000,1),exif_part()])


def segment(marker,body): return bytes([255,marker])+struct.pack('>H',len(body)+2)+body


def jfif(x,y,unit): return b'JFIF\0\x01\x02'+struct.pack('>BHHBB',unit,x,y,0,0)


def jpeg(name, base, parts=(), orientation=1, late=False, code=None):
    # Encoder writes one APP0 directly after SOI. Replace it explicitly.
    assert base[:4]==b'\xff\xd8\xff\xe0'
    rest=base[4+struct.unpack('>H',base[4:6])[0]:]
    b=base[:2]; declarations=[]
    if late: b+=rest[:-2]
    for marker,body,d in parts:
        if d: declarations.append(dict(d,sourceOffset=len(b)))
        b+=segment(marker,body)
    b+=b'\xff\xd9' if late else rest
    add(name,b,jpeg_pixels,8,16,orientation,declarations,code)


for progressive in [False,True]:
    base=subprocess.check_output([encoder]+(['progressive'] if progressive else []))
    for unit in [0,1,2]:
        for x,y in [(100,100),(100,200),(0,1)]:
            jpeg(f'jpeg-jfif-{progressive}-{unit}-{x}-{y}',base,
                 [(0xe0,jfif(x,y,unit),declaration('jfif',(x,1),(y,1),['aspectRatio','inch','centimetre'][unit]))])
    for late in [False,True]:
        for little in [False,True]:
            d=declaration('exifIfd0',(508,2),(254,1),'inch')
            jpeg(f'jpeg-exif-{progressive}-{late}-{little}',base,
                 [(0xe0,jfif(100,100,2),declaration('jfif',(100,1),(100,1),'centimetre')),
                  (0xe1,b'Exif\0\0'+exif((508,2),(254,1),2,6,little),d)],6,late)
    jpeg(f'jpeg-default-conflict-{progressive}',base,
         [(0xe0,jfif(300,300,1),declaration('jfif',(300,1),(300,1),'inch')),
          (0xe1,b'Exif\0\0'+exif(),declaration('exifIfd0',None,None,None))])
    jpeg(f'jpeg-no-resolution-{progressive}',base)
base=subprocess.check_output([encoder])
for name,parts in [('duplicate',[(0xe0,jfif(100,100,1),None)]*2),
                   ('bad-unit',[(0xe0,jfif(100,100,3),None)]),
                   ('short',[(0xe0,b'JFIF\0',None)]),
                   ('thumbnail',[(0xe0,jfif(100,100,1)[:-2]+b'\x01\x01',None)])]:
    jpeg('jpeg-invalid-'+name,base,parts,code='INPUT_INVALID')
good=exif((300,1),(300,1),2)
bad_exif={}
for name,position,value in [('type',24,3),('count',26,2),('offset',30,255),('unit',54,1),('directory',4,255)]:
    b=bytearray(good);b[position]=value;bad_exif[name]=bytes(b)
bad_exif['denominator']=exif((300,0),(300,1),2)
bad_exif['truncated']=good[:-1]
# Duplicate the XResolution tag over YResolution; retain a valid TIFF layout.
b=bytearray(good);b[34:36]=struct.pack('<H',0x11a);bad_exif['duplicate']=bytes(b)
for name,b in bad_exif.items():
    png('png-invalid-exif-'+name,[(b'eXIf',b,None)],code='INPUT_INVALID')
    jpeg('jpeg-invalid-exif-'+name,base,[(0xe1,b'Exif\0\0'+b,None)],code='INPUT_INVALID')
for name,parts in [('length',[(b'pHYs',bytes(8),None)]),('unit',[(b'pHYs',struct.pack('>IIB',1,1,2),None)]),
                   ('duplicate',[phys(10000,10000,1)]*2)]:
    png('png-invalid-physical-'+name,parts,code='INPUT_INVALID')
png('png-invalid-physical-late',[phys(10000,10000,1)],code='INPUT_INVALID',late=True)
png('png-late-exif',[exif_part((300,1),(300,1),2)],late=True)
(root/'manifest.json').write_text(json.dumps(cases,indent=2)+'\n')
print(json.dumps(dict(cases=len(cases),success=sum('expected' in c for c in cases))))
