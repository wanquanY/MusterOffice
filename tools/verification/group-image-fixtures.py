"""Bind actual Native paired sources and validate original failure mutations."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/group-image');out=root/'sources';out.mkdir(parents=True,exist_ok=True)
A='http://schemas.openxmlformats.org/drawingml/2006/main';P='http://schemas.openxmlformats.org/presentationml/2006/main';R='http://schemas.openxmlformats.org/officeDocument/2006/relationships';ns=dict(a=A,p=P)
def entry(p):
 b=Path(p).read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
cases=[]
for p in sorted((root/'native').glob('*.pptx')):
 cases.append(dict(name=p.stem,source=entry(p),pixels=entry(p.with_suffix('.rgba')),info=entry(p.with_suffix('.info.json')),status='rendered'))
assert len(cases)==20,len(cases)
for name in ['stationary','missing','external']:
 with zipfile.ZipFile(root/'native/siblings.pptx') as z:parts={n:z.read(n) for n in z.namelist()}
 s=X.fromstring(parts['ppt/slides/slide1.xml']);fill=s.find('.//p:grpSp/p:grpSpPr/a:blipFill',ns)
 if name=='stationary':fill.set('rotWithShape','0')
 elif name=='missing':fill.find('a:blip',ns).set('{'+R+'}embed','missing-owned')
 else:
  b=fill.find('a:blip',ns);b.attrib.pop('{'+R+'}embed');b.set('{'+R+'}link','owned-external')
 parts['ppt/slides/slide1.xml']=X.tostring(s);path=out/(name+'.pptx')
 with zipfile.ZipFile(path,'w') as z:
  for n,b in parts.items():
   i=zipfile.ZipInfo(n,(2026,9,25,0,0,0));i.compress_type=zipfile.ZIP_DEFLATED;z.writestr(i,b)
 cases.append(dict(name='inherited-'+name,source=entry(path),status='error',code='MAPPING_NOT_IMPLEMENTED' if name=='stationary' else 'RESOURCE_REQUIRED',imageIssue='stationaryOrientation' if name=='stationary' else 'resource',imageSource='linkedSource' if name=='external' else 'embeddedSnapshot'))
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'));parts_checked=0
for c in cases:
 with zipfile.ZipFile(c['source']['path']) as z:
  for n in z.namelist():
   if not n.endswith('.xml'):continue
   node=X.fromstring(z.read(n));q=X.QName(node)
   if q.namespace==P and q.localname in ['sld','sldLayout','sldMaster','presentation']:
    schema.assertValid(node);parts_checked+=1
report=dict(format='musteroffice.group-image-fixtures/1',cases=cases,officialXsdParts=parts_checked)
(root/'fixtures.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(cases=len(cases),officialXsdParts=parts_checked)))
