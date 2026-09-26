"""Owned synthetic containers and independent decoded pixel expectations."""
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


def chunk(name, data):
    name = name.encode()
    return struct.pack('>I', len(data)) + name + data + struct.pack('>I', zlib.crc32(name + data))


def png(w, h, raw, depth=8, color=6, extras=(), interlace=0):
    return (b'\x89PNG\r\n\x1a\n' + chunk('IHDR', struct.pack('>IIBBBBB', w, h, depth, color, 0, 0, interlace))
            + b''.join(chunk(k, v) for k, v in extras) + chunk('IDAT', zlib.compress(raw)) + chunk('IEND', b''))


def exif(origin, little=True):
    order = '<' if little else '>'
    return ((b'II' if little else b'MM') + struct.pack(order + 'HIH', 42, 8, 1)
            + struct.pack(order + 'HHIHHI', 0x112, 3, 1, origin, 0, 0))


def oriented(pixels, w, h, origin):
    width, height = (h, w) if origin >= 5 else (w, h)
    result = bytearray(len(pixels))
    for y in range(h):
        for x in range(w):
            dx, dy = [(x,y),(w-1-x,y),(w-1-x,h-1-y),(x,h-1-y),
                      (y,x),(h-1-y,x),(h-1-y,w-1-x),(y,w-1-x)][origin-1]
            result[(dy*width+dx)*4:(dy*width+dx+1)*4] = pixels[(y*w+x)*4:(y*w+x+1)*4]
    return bytes(result), width, height


def add(name, data, pixels=None, w=0, h=0, origin=1, code=None, tolerance=0, color=None):
    path = root / (name + '.bin')
    path.write_bytes(data)
    item = dict(name=name, path=str(path), sha256=hashlib.sha256(data).hexdigest(), code=code)
    if pixels is not None:
        pixels, width, height = oriented(pixels, w, h, origin)
        expected = root / (name + '.rgba')
        expected.write_bytes(pixels)
        item.update(expected=str(expected), width=width, height=height, orientation=origin,
                    tolerance=tolerance, color=color)
    cases.append(item)


rgba = bytes([255,0,0,255, 0,255,0,128, 10,20,30,0,
              0,0,255,64, 100,80,60,200, 230,170,80,255])
premul = bytes(c if i % 4 == 3 else (c * rgba[i//4*4+3]+127)//255 for i,c in enumerate(rgba))
raw = b'\0' + rgba[:12] + b'\0' + rgba[12:]
base = png(3, 2, raw)
for o in range(1, 9):
    for little in [False, True]:
        add(f'png-origin-{o}-{little}', png(3,2,raw,extras=[('eXIf',exif(o,little))]), premul,3,2,o)
add('png-plain',base,premul,3,2)
add('png-palette',png(3,1,b'\0\x00\x01\x02',color=3,extras=[('PLTE',bytes([255,0,0,0,255,0,0,0,255])),('tRNS',bytes([255,128,0]))]),bytes([255,0,0,255,0,128,0,128,0,0,0,0]),3,1)
add('png-gray2',png(4,1,b'\0\x1b',depth=2,color=0),bytes(sum(([v,v,v,255] for v in [0,85,170,255]),[])),4,1)
add('png-gray16',png(3,1,b'\0\x00\x00\x80\x80\xff\xff',depth=16,color=0),bytes([0,0,0,255,128,128,128,255,255,255,255,255]),3,1)
adam = bytearray()
for x0,y0,dx,dy in [(0,0,8,8),(4,0,8,8),(0,4,4,8),(2,0,4,4),(0,2,2,4),(1,0,2,2),(0,1,1,2)]:
    for y in range(y0,2,dy):
        row=b''.join(rgba[(y*3+x)*4:(y*3+x+1)*4] for x in range(x0,3,dx))
        if row: adam += b'\0'+row
add('png-adam7',png(3,2,adam,interlace=1),premul,3,2)
gray = bytes([40,40,40,255])*64 + bytes([180,180,180,255])*64
for progressive in [False,True]:
    jpg = subprocess.check_output([encoder] + (['progressive'] if progressive else []))
    for o in range(1,9):
        tag = b'Exif\0\0'+exif(o)
        data = jpg[:2] + b'\xff\xe1' + struct.pack('>H',len(tag)+2) + tag + jpg[2:]
        add(f'jpeg-{progressive}-origin-{o}',data,gray,8,16,o)
    add(f'jpeg-{progressive}-truncated',jpg[:-2],code='INPUT_INVALID')
    add(f'jpeg-{progressive}-extra',jpg+b'junk',code='INPUT_INVALID')
    mpf=b'MPF\0test'
    add(f'jpeg-{progressive}-mpf',jpg[:2]+b'\xff\xe2'+struct.pack('>H',len(mpf)+2)+mpf+jpg[2:],code='UNSUPPORTED')
add('png-gamma-linear',png(3,1,b'\0\x40\x80\xc0',color=0,extras=[('gAMA',struct.pack('>I',100000))]),
    bytes(sum(([round((1.055*(v/255)**(1/2.4)-0.055)*255)]*3+[255] for v in [64,128,192]),[])),3,1,tolerance=1,color='pngColor')


def icc_linear(gray=False):
    def fixed(v): return struct.pack('>i',round(v*65536))
    def xyz(v): return b'XYZ '+bytes(4)+b''.join(fixed(n) for n in v)
    curve=b'curv'+bytes(4)+struct.pack('>I',1)+struct.pack('>H',256)+bytes(2)
    tags=[(b'wtpt',xyz([0.9642,1,0.8249]))]
    if gray: tags.append((b'kTRC',curve))
    else:
        for channel, values in [(b'r',[0.4360747,0.2225045,0.0139322]),
                                (b'g',[0.3850649,0.7168786,0.0971045]),
                                (b'b',[0.1430804,0.0606169,0.7141733])]:
            tags += [(channel+b'XYZ',xyz(values)),(channel+b'TRC',curve)]
    header=bytearray(128)
    header[8:12]=bytes([2,0x10,0,0]);header[12:24]=b'mntr'+(b'GRAY' if gray else b'RGB ')+b'XYZ '
    header[24:36]=struct.pack('>6H',2026,9,25,0,0,0);header[36:40]=b'acsp'
    header[68:80]=b''.join(fixed(v) for v in [0.9642,1,0.8249])
    offset=132+len(tags)*12; table=struct.pack('>I',len(tags)); payload=b''
    for name,value in tags:
        table+=name+struct.pack('>II',offset,len(value));payload+=value;offset+=len(value)
    header[:4]=struct.pack('>I',offset)
    return bytes(header)+table+payload


linear=icc_linear()
linear_raw=bytes([64,128,192,255,128,64,32,128])
expected=[]
for i,v in enumerate(linear_raw):
    a=linear_raw[i//4*4+3]
    expected.append(a if i%4==3 else round((1.055*(v/255)**(1/2.4)-0.055)*a))
add('png-icc-linear-rgb',png(2,1,b'\0'+linear_raw,extras=[('iCCP',b'linear\0\0'+zlib.compress(linear))]),bytes(expected),2,1,tolerance=1,color='icc')
gray_profile=icc_linear(True)
gray_expected=bytes(sum(([round((1.055*(v/255)**(1/2.4)-0.055)*255)]*3+[255] for v in [40]*64+[180]*64),[]))
jpg=subprocess.check_output([encoder])
pieces=[gray_profile[:100],gray_profile[100:]]
segments=[]
for i,part in enumerate(pieces):
    payload=b'ICC_PROFILE\0'+bytes([i+1,2])+part
    segments.append(b'\xff\xe2'+struct.pack('>H',len(payload)+2)+payload)
for order in [[0,1],[1,0]]:
    add('jpeg-icc-order-'+''.join(map(str,order)),jpg[:2]+b''.join(segments[i] for i in order)+jpg[2:],gray_expected,8,16,tolerance=1,color='icc')
add('jpeg-icc-missing',jpg[:2]+segments[0]+jpg[2:],code='INPUT_INVALID')
add('jpeg-icc-duplicate',jpg[:2]+segments[0]*2+segments[1]+jpg[2:],code='INPUT_INVALID')
add('png-crc',base[:-1]+bytes([base[-1]^1]),code='INPUT_INVALID')
add('png-extra',base+b'junk',code='INPUT_INVALID')
add('png-apng',png(3,2,raw,extras=[('acTL',struct.pack('>II',2,0))]),code='UNSUPPORTED')
add('png-huge-dimension',png(8193,1,b''),code='LIMIT_EXCEEDED')
add('png-huge-pixels',png(8192,8192,b''),code='LIMIT_EXCEEDED')
add('png-bad-exif',png(3,2,raw,extras=[('eXIf',exif(9))]),code='INPUT_INVALID')
add('png-bad-icc',png(3,2,raw,extras=[('iCCP',b'bad\0\0'+zlib.compress(b'bad'))]),code='INPUT_INVALID')
add('png-large-icc',png(3,2,raw,extras=[('iCCP',b'large\0\0'+zlib.compress(bytes(1024*1024+1)))]),code='LIMIT_EXCEEDED')
compressed=zlib.compress(raw)
header=base[:33]
for suffix,data in [('short-deflate',compressed[:-2]),('bad-adler',compressed[:-1]+bytes([compressed[-1]^1])),
                    ('invalid-filter',zlib.compress(b'\x05'+raw[1:])),
                    ('extra-pixels',zlib.compress(raw+raw))]:
    add('png-'+suffix,header+chunk('IDAT',data)+chunk('IEND',b''),code='INPUT_INVALID')
sos=jpg.index(b'\xff\xda');scan_start=sos+2+struct.unpack('>H',jpg[sos+2:sos+4])[0]
add('jpeg-empty-scan',jpg[:scan_start]+b'\xff\xd9',code='INPUT_INVALID')
for i in range(1,len(base)):
    add(f'png-prefix-{i}',base[:i],code='UNSUPPORTED' if i < 8 else 'INPUT_INVALID')
(root/'manifest.json').write_text(json.dumps(cases,indent=2)+'\n')
print(json.dumps({'cases':len(cases),'success':sum('expected' in c for c in cases)}))
