"""Bind actual Native page fixtures, and construct owned negative PPTX probes."""
import copy, hashlib, json, zipfile
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/resource-page'); out=root/'sources';out.mkdir(parents=True,exist_ok=True)
P='http://schemas.openxmlformats.org/presentationml/2006/main';A='http://schemas.openxmlformats.org/drawingml/2006/main';R='http://schemas.openxmlformats.org/officeDocument/2006/relationships';ns=dict(p=P,a=A)
def entry(path):
 b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(name):
 with zipfile.ZipFile(root/'native'/f'{name}.pptx') as z:return {n:z.read(n) for n in z.namelist()}
def mutate(name,base,kind):
 parts=load(base);s=X.fromstring(parts['ppt/slides/slide1.xml']);pic=s.find('.//p:pic',ns)
 if kind=='stationary':pic.find('p:blipFill',ns).set('rotWithShape','0')
 elif kind=='missing':pic.find('p:blipFill/a:blip',ns).set('{'+R+'}embed','missing-owned')
 elif kind=='external':pic.find('p:blipFill/a:blip',ns).set('{'+R+'}link','owned-external')
 elif kind=='hidden':pic.find('p:nvPicPr/p:cNvPr',ns).set('hidden','1')
 elif kind=='effect':X.SubElement(pic.find('p:blipFill/a:blip',ns),'{'+A+'}grayscl')
 elif kind=='background-replay':
  shape=s.find('.//p:sp',ns);shape.set('useBgFill','1');shape.remove(shape.find('p:txBody',ns))
  bg=s.find('p:cSld/p:bg/p:bgPr',ns)
  for c in list(bg):bg.remove(c)
  fill=copy.deepcopy(pic.find('p:blipFill',ns));fill.tag='{'+A+'}blipFill';bg.append(fill)
 elif kind=='background-alpha':
  shape=s.find('.//p:sp',ns);shape.set('useBgFill','1');shape.remove(shape.find('p:txBody',ns))
  color=s.find('p:cSld/p:bg/p:bgPr/a:solidFill/a:srgbClr',ns);X.SubElement(color,'{'+A+'}alpha',val='50000')
 elif kind=='group-fill':
  group=s.find('.//p:grpSp/p:grpSpPr',ns);fill=copy.deepcopy(pic.find('p:blipFill',ns));fill.tag='{'+A+'}blipFill';group.append(fill)
  fill=pic.find('p:spPr/a:solidFill',ns);fill.getparent().replace(fill,X.Element('{'+A+'}grpFill'))
 elif kind=='bad-png':parts['ppt/media/owned-image.png']=b'\x89PNG\r\n\x1a\nowned invalid encoded pixels'
 else:raise AssertionError(kind)
 parts['ppt/slides/slide1.xml']=X.tostring(s)
 path=out/(name+'.pptx')
 with zipfile.ZipFile(path,'w') as z:
  for n,b in parts.items():
   info=zipfile.ZipInfo(n,(2026,9,25,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
 return entry(path)
cases=[]
for path in sorted((root/'native').glob('*.pptx')):
 name=path.stem
 cases.append(dict(name=name,source=entry(path),status='rendered',priorPlan=entry(root/'native'/(name+'.plan.json')),priorPixels=entry(root/'native'/(name+'.rgba'))))
assert len(cases)==15,len(cases)
for kind,base,code,issue in [
 ('stationary','dual-fill','MAPPING_NOT_IMPLEMENTED','stationaryOrientation'),
 ('missing','dual-fill','RESOURCE_REQUIRED','resource'),
 ('external','dual-fill','RESOURCE_REQUIRED','resource'),
 ('effect','dual-fill','MAPPING_NOT_IMPLEMENTED',None),
 ('background-replay','image-text','MAPPING_NOT_IMPLEMENTED',None),
 ('background-alpha','image-text','MAPPING_NOT_IMPLEMENTED',None),
 ('group-fill','group-direct-image','MAPPING_NOT_IMPLEMENTED',None),
 ('bad-png','dual-fill','INPUT_INVALID',None),
 ('hidden','dual-fill',None,None),
]:
 cases.append(dict(name=kind,source=mutate(kind,base,kind),status='error' if code else 'rendered',code=code,imageIssue=issue,
                  imageSource='linkedSource' if kind=='external' else 'embeddedSnapshot',decodeCalls=1 if kind=='bad-png' else 0))
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'));checked=0
for c in cases:
 with zipfile.ZipFile(c['source']['path']) as z:
  for n in z.namelist():
   if not n.endswith('.xml'):continue
   node=X.fromstring(z.read(n));q=X.QName(node)
   if q.namespace==P and q.localname in ['sld','sldLayout','sldMaster','presentation']:
    schema.assertValid(node);checked+=1
report=dict(format='musteroffice.resource-page-fixtures/1',cases=cases,officialXsdParts=checked)
(root/'fixtures.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(sourceCases=len(cases),officialXsdParts=checked)))
