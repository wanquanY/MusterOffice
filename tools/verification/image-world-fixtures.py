"""Owned PPTX probes for source-bound world image paint; no application assets."""
import hashlib
import io
import itertools
import json
from pathlib import Path
import struct
import subprocess
import zlib
import zipfile
from lxml import etree as X

ROOT = Path('.codex-work/image-world')
CASES = ROOT/'cases'
CASES.mkdir(parents=True, exist_ok=True)
P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
NS = dict(p=P, a=A)
def entry(path):
    b = Path(path).read_bytes()
    return dict(path=str(path), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())
def child(parent, tag, **attrs):
    prefix, name = tag.split(':')
    return X.SubElement(parent, '{'+NS[prefix]+'}'+name, **{k:str(v) for k,v in attrs.items()})
old = json.loads(Path('.codex-work/image-layout/reference.json').read_text())
base = old['cases'][0]
assert entry(base['source']['path']) == base['source']
with zipfile.ZipFile(base['source']['path']) as z:
    original = {n:z.read(n) for n in z.namelist()}
pixels = bytes(v for y in range(4) for x in range(5) for v in [30+x*45, 40+y*55, 50+((x+y)%3)*60, 255])
def chunk(name, data):
    return struct.pack('>I',len(data))+name+data+struct.pack('>I',zlib.crc32(name+data))
png = b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',5,4,8,6,0,0,0))
png += chunk(b'IDAT',zlib.compress(b''.join(b'\0'+pixels[y*20:(y+1)*20] for y in range(4))))+chunk(b'IEND',b'')
(ROOT/'pixels.rgba').write_bytes(pixels)
schema = X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
configs = [dict(angle=a, grouped=g, mode=m, rotate=True, backgroundOnly=False)
           for a,g,m in itertools.product([0,45,90,135,270,350],[False,True],['stretch','tile'])]
configs += [dict(angle=45, grouped=True, mode=m, rotate=False, backgroundOnly=b)
            for b,m in itertools.product([False,True],['stretch','tile'])]
records = []
for i,c in enumerate(configs):
    parts = dict(original)
    slide = X.fromstring(parts['ppt/slides/slide1.xml'])
    pres = X.fromstring(parts['ppt/presentation.xml'])
    size = pres.find('p:sldSz',NS); size.set('cx','1280000'); size.set('cy','960000')
    fills = slide.findall('.//p:blipFill',NS)+slide.findall('.//a:blipFill',NS)
    for fill in fills:
        fill.set('rotWithShape',str(int(c['rotate']))); fill.set('dpi','45')
        for e in list(fill):
            if X.QName(e).localname in ['srcRect','stretch','tile']: fill.remove(e)
        child(fill,'a:srcRect',l='20000',t='25000',r='20000',b='0')
        if c['mode']=='stretch':
            child(child(fill,'a:stretch'),'a:fillRect',l='12.3456789123%',t='12500',r='-10%',b='0')
        else:
            child(fill,'a:tile',algn='ctr',flip='xy',sx='75%',sy='125000',tx='-6666.6666666pt',ty='0.2cm')
    tree = slide.find('p:cSld/p:spTree',NS)
    shapes = [e for e in tree if X.QName(e).localname in ['pic','sp']]
    for k,shape in enumerate(shapes):
        xfrm = shape.find('p:spPr/a:xfrm',NS)
        xfrm.set('rot',str(c['angle']*60000)); xfrm.set('flipH',str(i%2)); xfrm.set('flipV',str((i//2)%2))
        if c['grouped']:
            tree.remove(shape); parent = tree
            for depth in [0,1]:
                g = child(parent,'p:grpSp')
                nv = child(g,'p:nvGrpSpPr'); child(nv,'p:cNvPr',id=100+k*10+depth,name='Owned group'); child(nv,'p:cNvGrpSpPr'); child(nv,'p:nvPr')
                t = child(child(g,'p:grpSpPr'),'a:xfrm',rot=(30 if depth==0 else -15)*60000,flipH=depth)
                child(t,'a:off',x=150 if depth==0 else 25,y=200 if depth==0 else 50)
                child(t,'a:ext',cx=2400 if depth==0 else 2000,cy=1800)
                child(t,'a:chOff',x=-150 if depth==0 else 50,y=75 if depth==0 else -90)
                child(t,'a:chExt',cx=3000 if depth==0 else 2500,cy=2100 if depth==0 else 1900)
                parent=g
            parent.append(shape)
    # Keep real sldSz above the standard's one-inch minimum.
    for transform in slide.findall('.//a:xfrm', NS):
        for item in transform:
            for key, value in list(item.attrib.items()):
                item.set(key, str(int(value)*200))
    schema.assertValid(slide); schema.assertValid(pres)
    parts['ppt/slides/slide1.xml']=X.tostring(slide);parts['ppt/presentation.xml']=X.tostring(pres);parts['ppt/media/image.png']=png
    path=CASES/f'{i:03d}.pptx'
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for name,data in parts.items():
            info=zipfile.ZipInfo(name,(2026,9,25,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,data)
    source=entry(path)
    q=json.loads(Path(base['request']['path']).read_text());q['fill']['expectedSourceSha256']=source['sha256']
    if c['backgroundOnly']:q['fill']['targets']=[dict(kind='background')]
    request=CASES/f'{i:03d}.json';request.write_text(json.dumps(q))
    run=subprocess.run(['target/debug/examples/source_image_layout',str(request),str(path),'--paint'],capture_output=True,check=True)
    assert not run.stderr,run.stderr
    output=CASES/f'{i:03d}.result.json';output.write_bytes(run.stdout)
    r=json.loads(run.stdout);assert r['result']['status']=='laidOut'
    success=c['rotate'] or c['backgroundOnly']
    assert r['paint']['status']==('compiled' if success else 'error'),r
    assert len(r['decoded'])==1 and r['decoded'][0]['pixelsSha256']==entry(ROOT/'pixels.rgba')['sha256']
    if not success:assert 'paints' not in r['paint'] and 'stationary image orientation policy required' in r['paint']['message']
    assert entry(path)==source
    records.append(dict(name=f'{i:03d}',config=c,success=success,source=source,request=entry(request),output=entry(output)))
result=dict(format='musteroffice.image-world-fixtures/1',base=base['source'],pixels=entry(ROOT/'pixels.rgba'),pageSize=dict(width='1280000',height='960000'),emuPerPixel=20000,nativeRequests=len(records),successfulRequests=sum(c['success'] for c in records),officialXsdParts=len(records)*2,example=entry('target/debug/examples/source_image_layout'),cases=records)
(ROOT/'fixtures.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k in ['nativeRequests','successfulRequests','officialXsdParts']}))
