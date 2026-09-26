"""Structurally valid owned inputs for public radial geometry failures."""
import hashlib,json,re,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/radial-layout');out=root/'invalid';out.mkdir(exist_ok=True)
base=root/'native/center-point.pptx'
mutations={
 'inverted-focus':lambda s:re.sub(r'<a:fillToRect[^>]*/>', '<a:fillToRect l="80%" t="50%" r="80%" b="50%"/>',s),
 'collapsed-tile':lambda s:s.replace('</a:gradFill>','<a:tileRect l="75%" r="25%"/></a:gradFill>'),
 'no-locus':lambda s:s.replace('<a:prstGeom prst="rect"/>','<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l="0" t="0" r="w" b="h"/><a:pathLst><a:path><a:moveTo><a:pt x="0" y="0"/></a:moveTo></a:path></a:pathLst></a:custGeom>'),
 'wrong-shade':lambda s:s.replace('path="circle"','path="shape"'),
}
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
records=[];parts=0
for name,mutate in mutations.items():
 path=out/(name+'.pptx')
 with zipfile.ZipFile(base) as src,zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as dst:
  for record in src.infolist():
   data=src.read(record.filename)
   if record.filename=='ppt/slides/slide1.xml':
    changed=mutate(data.decode()).encode();assert changed!=data;data=changed
   dst.writestr(record,data)
 with zipfile.ZipFile(path) as src:
  for n in src.namelist():
   if not n.endswith('.xml'):continue
   node=X.fromstring(src.read(n));q=X.QName(node)
   if q.namespace=='http://schemas.openxmlformats.org/presentationml/2006/main' and q.localname in ['presentation','sld','sldLayout','sldMaster']:schema.assertValid(node);parts+=1
 b=path.read_bytes();records.append(dict(name=name,source=dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())))
(root/'invalid.json').write_text(json.dumps(dict(cases=records,officialXsdParts=parts),indent=2)+'\n');print(json.dumps(dict(cases=len(records),parts=parts)))
