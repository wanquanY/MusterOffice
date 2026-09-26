"""Owned actual PPTX line colors, including native reference composition."""
import copy,hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P
ROOT=Path('.codex-work/line-colors');ROOT.mkdir(exist_ok=True,parents=True)
NS={'a':A,'p':P};S='ppt/slides/slide1.xml';T='ppt/theme/theme2.xml';cases=[]
def sha(b):return hashlib.sha256(b).hexdigest()
def read(path):return json.loads(Path(path).read_text())
def xml(text):return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}">{text}</root>')
def make(name,base,colors,context=None,reference=None,theme_color=None,expected='evaluated'):
 b=Path(base['path']).read_bytes();assert sha(b)==base['sha256']
 with zipfile.ZipFile(base['path']) as z:parts={n:z.read(n) for n in z.namelist()}
 root=E.fromstring(parts[S]);tree=root.find('p:cSld/p:spTree',NS);template=tree.find('p:sp',NS);assert template is not None
 for node in list(tree):
  if E.QName(node).localname not in ['nvGrpSpPr','grpSpPr']:tree.remove(node)
 for i,color in enumerate(colors):
  node=copy.deepcopy(template);identity=node.find('p:nvSpPr/p:cNvPr',NS);identity.set('id',str(100+i));identity.set('name',f'Owned line color {i}')
  for ph in node.findall('p:nvSpPr/p:nvPr/p:ph',NS):ph.getparent().remove(ph)
  props=node.find('p:spPr',NS);props.clear();props.extend(list(xml('<a:xfrm><a:off x="914400" y="914400"/><a:ext cx="2743200" cy="1371600"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:solidFill>'+color+'</a:solidFill></a:ln>')))
  for old in node.findall('p:style',NS):node.remove(old)
  if reference is not None:
   style=E.Element(f'{{{P}}}style');style.extend(list(xml('<a:lnRef idx="1">'+reference+'</a:lnRef><a:fillRef idx="0"/><a:effectRef idx="0"/><a:fontRef idx="minor"/>')));node.insert(list(node).index(props)+1,style)
  tree.append(node)
 parts[S]=E.tostring(root,xml_declaration=True,encoding='UTF-8')
 if theme_color:
  root=E.fromstring(parts[T]);accent=root.find('a:themeElements/a:clrScheme/a:accent1',NS);accent.clear();accent.extend(list(xml(theme_color)));parts[T]=E.tostring(root,xml_declaration=True,encoding='UTF-8')
 path=ROOT/(name+'.pptx')
 with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
  for part,raw in parts.items():
   info=zipfile.ZipInfo(part,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,raw)
 cases.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'objects':list(range(100,100+len(colors))),'context':context or {'systemColors':{},'placeholder':None},'expected':expected,'base':{'path':base['path'],'sha256':base['sha256']}})
for c in read('.codex-work/pptx-colors-fixtures/manifest.json')['cases']:
 make(c['name'],c,[f'<a:schemeClr val="{slot}"/>' for slot in c['request']['colors']],c['request']['context'],expected='LIMIT_EXCEEDED' if c.get('expectedFirst')=='LIMIT_EXCEEDED' else 'evaluated')
base=read('.codex-work/line-style/manifest.json')['cases'][0]
probes=[
 ('subbyte-reference','<a:schemeClr val="phClr"><a:redMod val="50000"/></a:schemeClr>','<a:srgbClr val="010000"><a:redMod val="50000"/></a:srgbClr>'),
 ('latent-hue','<a:schemeClr val="phClr"><a:sat val="100000"/></a:schemeClr>','<a:hslClr hue="14400000" sat="0" lum="50000"/>'),
 ('latent-black','<a:schemeClr val="phClr"><a:lum val="50000"/></a:schemeClr>','<a:hslClr hue="14400000" sat="100000" lum="0"/>'),
 ('reference-alpha-order','<a:schemeClr val="phClr"><a:alphaOff val="25000"/><a:alphaMod val="50000"/></a:schemeClr>','<a:srgbClr val="1248CD"><a:alpha val="20000"/><a:alphaMod val="123.456789%"/></a:srgbClr>'),
 ('nested-missing','<a:schemeClr val="phClr"/>','<a:schemeClr val="phClr"/>'),
 ('nested-host','<a:schemeClr val="phClr"><a:alphaMod val="50000"/></a:schemeClr>','<a:schemeClr val="phClr"><a:alphaMod val="50000"/></a:schemeClr>'),
 ('unused-missing-system','<a:srgbClr val="FF2200"/>','<a:sysClr val="windowText"/>'),
 ('used-missing-system','<a:schemeClr val="phClr"/>','<a:sysClr val="windowText"/>'),
 ('system-file','<a:schemeClr val="phClr"/>','<a:sysClr val="windowText" lastClr="112233"/>'),
 ('system-host','<a:schemeClr val="phClr"/>','<a:sysClr val="windowText" lastClr="112233"/>'),
 ('extended-reference','<a:schemeClr val="phClr"/>','<a:scrgbClr r="200000" g="-50000" b="50000"/>'),
 ('reference-transform-order','<a:schemeClr val="phClr"><a:greenMod val="50000"/><a:tint val="75000"/></a:schemeClr>','<a:srgbClr val="1248CD"><a:tint val="33333"/><a:greenMod val="20000"/></a:srgbClr>'),
 ('theme-ph-context','<a:schemeClr val="accent1"><a:redMod val="50000"/></a:schemeClr>','<a:srgbClr val="010000"><a:redMod val="50000"/></a:srgbClr>'),
 ('theme-reference-cycle','<a:schemeClr val="accent1"/>','<a:schemeClr val="accent1"/>'),
 ('unused-reference-cycle','<a:srgbClr val="AABBCC"/>','<a:schemeClr val="accent1"/>'),
 ('host-fallback-no-reference','<a:schemeClr val="phClr"><a:alphaMod val="50000"/></a:schemeClr>',None),
]
for name,color,reference in probes:
 context={'systemColors':{'windowText':[44,55,66]} if name=='system-host' else {},'placeholder':[255,0,0,128] if name in ['nested-host','host-fallback-no-reference'] else None}
 theme='<a:schemeClr val="phClr"/>' if name in ['theme-ph-context','theme-reference-cycle'] else '<a:schemeClr val="accent1"/>' if name=='unused-reference-cycle' else None
 make(name,base,[color],context,reference,theme)
(ROOT/'manifest.json').write_text(json.dumps({'format':'musteroffice.line-color-fixtures/1','cases':cases},indent=2)+'\n');print(json.dumps({'files':len(cases)}))
