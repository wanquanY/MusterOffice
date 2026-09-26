"""Owned native PPTX -> real decoder -> layout, checked with exact Fractions.

This covers native source identity and local paint geometry, not page rendering.
"""
from fractions import Fraction as F
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

ROOT = Path('.codex-work/image-layout')
CASES = ROOT/'cases'
CASES.mkdir(parents=True, exist_ok=True)
P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
PR = 'http://schemas.openxmlformats.org/package/2006/relationships'
CT = 'http://schemas.openxmlformats.org/package/2006/content-types'
NS = {'p': P, 'a': A, 'r': R}
Q = 1 << 32
schema = X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
sha = lambda b: hashlib.sha256(b).hexdigest()


def entry(p):
    b = p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=sha(b))


def child(parent, tag, **attrs):
    prefix, name = tag.split(':')
    return X.SubElement(parent, '{'+NS[prefix]+'}'+name, **{k: str(v) for k, v in attrs.items()})


def image(density):
    def chunk(name, data):
        return struct.pack('>I', len(data))+name+data+struct.pack('>I', zlib.crc32(name+data))
    pixels = bytes([80, 30, 100, 128])*20
    png = b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB', 5, 4, 8, 6, 0, 0, 0))
    if density:
        png += chunk(b'pHYs', struct.pack('>IIB', *density, 1))
    return png+chunk(b'IDAT', zlib.compress(b''.join(b'\0'+pixels[y*20:(y+1)*20] for y in range(4))))+chunk(b'IEND', b'')


def fill(parent, c, picture=False):
    f = child(parent, 'p:blipFill' if picture else 'a:blipFill', dpi=c['dpi'], rotWithShape=int(c['rotate']))
    b = child(f, 'a:blip'); b.set('{'+R+'}embed', 'rIdImage')
    child(f, 'a:srcRect', **dict(zip('ltrb', c['crop'])))
    if c['mode'] == 'stretch':
        child(child(f, 'a:stretch'), 'a:fillRect', **dict(zip('ltrb', c['rect'])))
    else:
        child(f, 'a:tile', algn=c['align'], flip=c['flip'], sx=c['sx'], sy=c['sy'], tx=c['tx'], ty=c['ty'])


def pptx(c):
    s = X.Element('{'+P+'}sld', nsmap=NS)
    common = child(s, 'p:cSld')
    fill(child(child(common, 'p:bg'), 'p:bgPr'), c)
    tree = child(common, 'p:spTree')
    nv = child(tree, 'p:nvGrpSpPr'); child(nv, 'p:cNvPr', id=1, name='Root'); child(nv, 'p:cNvGrpSpPr'); child(nv, 'p:nvPr')
    child(tree, 'p:grpSpPr')
    for picture, native_id, w, h in [(True, 9, 1000, 800), (False, 14, 2000, 1600)]:
        shape = child(tree, 'p:pic' if picture else 'p:sp')
        nv = child(shape, 'p:nvPicPr' if picture else 'p:nvSpPr')
        child(nv, 'p:cNvPr', id=native_id, name='Picture' if picture else 'Image fill')
        child(nv, 'p:cNvPicPr' if picture else 'p:cNvSpPr'); child(nv, 'p:nvPr')
        if picture: fill(shape, c, True)
        prop = child(shape, 'p:spPr')
        transform = child(prop, 'a:xfrm', rot=5400000, flipH=1)
        child(transform, 'a:off', x=100, y=200); child(transform, 'a:ext', cx=w, cy=h)
        child(child(prop, 'a:prstGeom', prst='rect'), 'a:avLst')
        if picture: child(prop, 'a:noFill')
        else: fill(prop, c)
        child(child(prop, 'a:ln'), 'a:noFill')
    pres = X.Element('{'+P+'}presentation', nsmap=NS)
    rel = child(child(pres, 'p:sldIdLst'), 'p:sldId', id=256); rel.set('{'+R+'}id', 'rId1')
    child(pres, 'p:sldSz', cx=9144000, cy=7315200); child(pres, 'p:notesSz', cx=9144000, cy=7315200)
    schema.assertValid(s); schema.assertValid(pres)
    def rels(kind, target, rid):
        root = X.Element('{'+PR+'}Relationships', nsmap={None: PR})
        X.SubElement(root, '{'+PR+'}Relationship', Id=rid, Type=R+'/'+kind, Target=target)
        return X.tostring(root)
    types = X.Element('{'+CT+'}Types', nsmap={None: CT})
    X.SubElement(types, '{'+CT+'}Default', Extension='rels', ContentType='application/vnd.openxmlformats-package.relationships+xml')
    for part, content in [('ppt/presentation.xml', 'application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml'),
                          ('ppt/slides/slide1.xml', 'application/vnd.openxmlformats-officedocument.presentationml.slide+xml'),
                          ('ppt/media/image.png', 'image/png')]:
        X.SubElement(types, '{'+CT+'}Override', PartName='/'+part, ContentType=content)
    parts = {'[Content_Types].xml': X.tostring(types), '_rels/.rels': rels('officeDocument', 'ppt/presentation.xml', 'rId1'),
             'ppt/presentation.xml': X.tostring(pres), 'ppt/_rels/presentation.xml.rels': rels('slide', 'slides/slide1.xml', 'rId1'),
             'ppt/slides/slide1.xml': X.tostring(s), 'ppt/slides/_rels/slide1.xml.rels': rels('image', '../media/image.png', 'rIdImage'),
             'ppt/media/image.png': image(c['density'])}
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, data in parts.items():
            info = zipfile.ZipInfo(name, (2026, 9, 25, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)
    return buf.getvalue()


def percentage(s): return F(s[:-1])/100 if s.endswith('%') else F(s)/100000
def coordinate(s):
    for unit, value in [('mm', 36000), ('cm', 360000), ('in', 914400), ('pt', 12700), ('pc', 152400), ('pi', 152400)]:
        if s.endswith(unit): return F(s[:-2])*value
    return F(s)
def rectangle(edges, w, h):
    l, t, r, b = map(percentage, edges)
    return [l*w, t*h, (1-r)*w, (1-b)*h]
def reference(c, w, h):
    source = rectangle(c['crop'], 5, 4); sw, sh = source[2]-source[0], source[3]-source[1]
    if c['mode'] == 'stretch':
        target = rectangle(c['rect'], w, h)
        step = [(target[2]-target[0])/sw, (target[3]-target[1])/sh]
    else:
        pixel = [F(914400, c['dpi'])]*2 if c['dpi'] else [F(36000000, n) for n in c['density']]
        step = [pixel[0]*percentage(c['sx']), pixel[1]*percentage(c['sy'])]
        tw, th = sw*step[0], sh*step[1]
        align = {'tl': (0,0), 't': (1,0), 'tr': (2,0), 'l': (0,1), 'ctr': (1,1), 'r': (2,1), 'bl': (0,2), 'b': (1,2), 'br': (2,2)}[c['align']]
        l, t = (w-tw)*F(align[0],2)+coordinate(c['tx']), (h-th)*F(align[1],2)+coordinate(c['ty'])
        target = [l, t, l+tw, t+th]
    origin = [target[0]-source[0]*step[0], target[1]-source[1]*step[1]]
    return source, target, step, origin


base = dict(mode='stretch', crop=['0']*4, rect=['0']*4, dpi=0, density=None, rotate=True,
            align='tl', flip='none', sx='100000', sy='100%', tx='0', ty='0')
configs = []
for crop, rect in itertools.product([['0']*4, ['25%','12500','25%','12500'], ['-25000','0','-25000','0'], ['1.23456789123%','2.5%','0','0']],
                                     [['0']*4, ['10000','25000','30000','0'], ['-10%','0','0','-10%']]):
    for rotate in [True, False]: configs.append(({**base, 'crop':crop, 'rect':rect, 'rotate':rotate}, True))
for align, flip, setting in itertools.product(['tl','t','tr','l','ctr','r','bl','b','br'], ['none','x','y','xy'], [0,1]):
    configs.append(({**base, 'mode':'tile', 'align':align, 'flip':flip,
                     'crop':['25%','0','0','25%'], 'sx':'50%', 'sy':'125000', 'tx':'-0.0001in', 'ty':'0.002cm',
                     'dpi':96 if setting else 0, 'density':(7200,3600)}, True))
for mutation in [dict(crop=['100000','0','0','0']), dict(rect=['0','0','100000','0']),
                 dict(mode='tile'), dict(mode='tile', density=(0,3600)),
                 dict(mode='tile', dpi=96, sx='0'), dict(mode='tile', dpi=96, sx='-1')]:
    configs.append(({**base, **mutation}, False))
records = []; values = 0
for n, (config, success) in enumerate(configs):
    prefix = CASES/f'{n:03d}'
    source = prefix.with_suffix('.pptx'); source.write_bytes(pptx(config))
    request = prefix.with_suffix('.json')
    source_entry = entry(source)
    request.write_text(json.dumps(dict(fill=dict(expectedSourceSha256=source_entry['sha256'], surface='/ppt/slides/slide1.xml',
        targets=[dict(kind='picture',nativeId=9),dict(kind='object',nativeId=14),dict(kind='background')],
        profile='ms-oi29500-fills-2024-draft-v1'),selection='embeddedSnapshot')))
    run = subprocess.run(['target/debug/examples/source_image_layout', str(request), str(source)]+(['--audit'] if n == 0 else []), capture_output=True, check=True)
    assert not run.stderr, run.stderr
    output = prefix.with_suffix('.result.json'); output.write_bytes(run.stdout)
    result = json.loads(run.stdout)
    assert entry(source) == source_entry
    assert result['catalog']['sourceSha256'] == source_entry['sha256']
    assert result['result']['status'] == ('laidOut' if success else 'error'), (n, result)
    assert len(result['catalog']['resources']) == len(result['decoded']) == 1
    decoded = result['decoded'][0]
    assert (decoded['width'], decoded['height']) == (5,4)
    assert decoded['sourceSha256'] == result['catalog']['resources'][0]['sha256'] == sha(image(config['density']))
    if success:
        plans = result['result']['plans']; assert len(plans) == 3
        assert [p['target'] for p in plans] == json.loads(request.read_text())['fill']['targets']
        for plan, dimensions in zip(plans, [(1000,800),(2000,1600),(9144000,7315200)]):
            assert plan['resource'] == 0
            source_rect, target, step, origin = reference(config, *dimensions)
            layout = plan['layout']; errors = layout['uncertainty']
            for key, expected in [('sourceRectangle',source_rect),('fillRectangle',target),('pixelStep',step),('origin',origin)]:
                keys = 'ltrb' if len(expected)==4 else 'xy'
                names = dict(l='left',t='top',r='right',b='bottom',x='x',y='y')
                bounds = errors[key]
                for i, k in enumerate(keys):
                    actual = F(int(layout[key][names[k]]), Q)
                    bound = F(int(bounds[i] if isinstance(bounds,list) else bounds[names[k]]), Q)
                    assert abs(actual-expected[i]) <= bound, (n,key,k,actual,expected[i],bound)
                    # At ordinary scales, no inflated catch-all bound is accepted.
                    assert bound <= F(1, 1_000_000), (n,key,bound)
                    values += 1
            assert layout['rotateWithShape'] == config['rotate']
            assert layout['clipToFillRectangle'] == (config['mode']=='stretch')
            for axis in 'xy':
                expected = 'clamp' if config['mode']=='stretch' else 'mirror' if axis in config['flip'] else 'repeat'
                assert layout['tile'+axis.upper()] == expected
            if plan['placement']:
                assert plan['placement']['transform']['rotation']['value'] == 5400000
                assert plan['placement']['transform']['flipHorizontal']['value'] is True
    records.append(dict(name=f'{n:03d}',config=config,success=success,source=entry(source),request=entry(request),output=entry(output)))
report = dict(format='musteroffice.image-layout-reference/1', nativeRequests=len(records),
              successfulRequests=sum(c['success'] for c in records), sourceTargetLayouts=sum(c['success'] for c in records)*3,
              independentBoundedValues=values, officialXsdParts=len(records)*2, resourceReuseReferences=len(records)*3,
              sourceFileHashUnchanged=True, example=entry(Path('target/debug/examples/source_image_layout')), cases=records)
report['bindingChecks'] = json.loads(Path(records[0]['output']['path']).read_text())['checks']
assert len(report['bindingChecks']['rejected']) == 9
assert report['bindingChecks']['cancelledCheckpoints'] > 0
# The source operation is read-only; verify every package again after execution.
for c in records: assert entry(Path(c['source']['path'])) == c['source']
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k not in ['format','example','cases']}))
