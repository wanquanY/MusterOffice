"""Owned structurally valid native gradients that need diagnostics, not fallback."""
import hashlib,io,json,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/gradient-field');out=root/'invalid-source';out.mkdir(exist_ok=True)
source=root/'native/right.pptx';q=json.loads((root/'native/right.request.json').read_text())
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
with zipfile.ZipFile(source) as z:parts={n:z.read(n) for n in z.namelist()}
xml=parts['ppt/slides/slide1.xml'].decode()
lin='<a:lin ang="0" scaled="0"/>'
records=[]
for name,old,new in [
    ('stationary','<a:gradFill >','<a:gradFill rotWithShape="0">'),
    ('path',lin,'<a:path path="circle"/>'),
    ('nonpositive-tile','</a:gradFill>','<a:tileRect l="60000" r="60000"/></a:gradFill>'),
    ('unordered-stops','<a:gs pos="0">','<a:gs pos="100000">'),
]:
    assert old in xml
    changed=xml.replace(old,new,1)
    if name=='unordered-stops':
        changed=changed.replace('<a:gs pos="100000"><a:srgbClr val="0000FF"/>','<a:gs pos="0"><a:srgbClr val="0000FF"/>')
    schema.assertValid(X.fromstring(changed.encode()))
    b=io.BytesIO()
    with zipfile.ZipFile(b,'w',compression=zipfile.ZIP_DEFLATED) as z:
        for n,value in parts.items():
            info=zipfile.ZipInfo(n,date_time=(2026,9,25,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
            z.writestr(info,changed.encode() if n=='ppt/slides/slide1.xml' else value)
    data=b.getvalue();page=dict(q,expectedSourceSha256=hashlib.sha256(data).hexdigest())
    path=out/(name+'.pptx');path.write_bytes(data)
    (out/(name+'.json')).write_text(json.dumps(dict(profile='drawingml-resource-page-q32-v1-draft',page=page,imageSource='embeddedSnapshot',sampling='nearest',fonts=None)))
    records.append(name)
(root/'invalid-source.json').write_text(json.dumps(dict(cases=records,officialSlideParts=4))+'\n')
print(json.dumps(dict(cases=records,officialSlideParts=4)))
