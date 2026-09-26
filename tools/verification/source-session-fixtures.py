"""Owned mixed text/image group with independent fractional rotations and XML controls."""
import copy,hashlib,json,zipfile
from pathlib import Path
from fractions import Fraction
from lxml import etree as X
root=Path('.codex-work/source-session');out=root/'fixtures';out.mkdir(parents=True,exist_ok=True)
P='http://schemas.openxmlformats.org/presentationml/2006/main';A='http://schemas.openxmlformats.org/drawingml/2006/main';ns=dict(p=P,a=A)
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):assert entry(r['path'])==r;return Path(r['path']).read_bytes()
def pack(parts,path):
 with zipfile.ZipFile(path,'w')as z:
  for n,b in parts.items():
   i=zipfile.ZipInfo(n,(2026,9,26,0,0,0));i.compress_type=zipfile.ZIP_DEFLATED;z.writestr(i,b)
 return entry(path)
prior=json.loads(Path('.codex-work/source-playback/product.json').read_text());base=next(c for c in prior['cases']if c['name']=='image-text-0');load(base['source']);q=json.loads(load(base['request']));fonts=base['fonts'];load(fonts)
with zipfile.ZipFile(base['source']['path'])as z:parts={n:z.read(n)for n in z.namelist()}
slide=q['page']['page']['slide'].lstrip('/');tree=X.fromstring(parts[slide]);sp=tree.find('p:cSld/p:spTree',ns)
group=X.fromstring(f'<p:grpSp xmlns:p="{P}" xmlns:a="{A}"><p:nvGrpSpPr><p:cNvPr id="900" name="Owned mixed group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm flipH="1"><a:off x="50000" y="100000"/><a:ext cx="1400000" cy="900000"/><a:chOff x="0" y="0"/><a:chExt cx="2000000" cy="1000000"/></a:xfrm></p:grpSpPr></p:grpSp>')
for o in list(sp):
 if X.QName(o).localname in ['sp','pic']:sp.remove(o);group.append(o)
sp.append(group);animations=tree.findall('.//p:animRot',ns)
animations[0].set('from','2699999');animations[0].set('to','8100001');animations[0].find('.//p:spTgt',ns).set('spid','900')
animations[1].set('from','0');animations[1].set('to','21600000');animations[1].find('.//p:spTgt',ns).set('spid','43');animations[1].find('.//p:cTn',ns).set('fill','freeze')
parts[slide]=X.tostring(tree);source=pack(parts,out/'mixed.pptx');q['page']['page']['expectedSourceSha256']=source['sha256'];q['sample']['binding']=dict(session='mixed',revision=source['sha256'],generation='11')
cases=[]
for i,(n,d)in enumerate([(0,1),(1,3),(1,2),(1,1),(3,2),(2,1),(1,3)]):
 at=Fraction(n,d);request=copy.deepcopy(q);request['sample']['at']=dict(ticks=str(n),timescale=d)
 rotations={f'sp.{id}':dict(numerator=str(v.numerator),denominator=str(v.denominator))for id,v in [(900,Fraction(2699999)+5400002*at/2),(43,21600000*at/2)]}
 p=out/f'mixed-{i}.json';p.write_text(json.dumps(request));c=dict(name=f'mixed-{i}',request=entry(p),source=source,fonts=fonts,expectedRotations=rotations)
 if all(v['denominator']=='1'for v in rotations.values()):
  control=copy.deepcopy(tree);control.remove(control.find('p:timing',ns))
  for o in control.findall('.//p:grpSp',ns)+control.findall('.//p:sp',ns):
   id=o.find('./*/p:cNvPr',ns).get('id');v=rotations.get('sp.'+id)
   if v:o.find(('p:grpSpPr'if X.QName(o).localname=='grpSp'else'p:spPr')+'/a:xfrm',ns).set('rot',v['numerator'])
  cp=dict(parts);cp[slide]=X.tostring(control);cs=pack(cp,out/f'mixed-{i}.static.pptx');cq=copy.deepcopy(request['page']);cq['page']['expectedSourceSha256']=cs['sha256'];p=out/f'mixed-{i}.static.json';p.write_text(json.dumps(cq));c['staticControl']=dict(source=cs,request=entry(p))
 cases.append(c)
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'));checked=0
for s in [source]+[c['staticControl']['source']for c in cases if 'staticControl'in c]:
 with zipfile.ZipFile(s['path'])as z:
  for name in z.namelist():
   if name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/'))and name.endswith('.xml')and '/_rels/'not in name:schema.assertValid(X.fromstring(z.read(name)));checked+=1
report=dict(format='musteroffice.source-session-fixtures/1',baseSource=base['source'],cases=cases,officialXsdParts=checked)
(root/'fixtures.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(cases=len(cases),officialXsdParts=checked)))
