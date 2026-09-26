"""Independent physical XML color terms, lazy contexts and Decimal working colors."""
import hashlib,json,math,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import project,A,P
from drawingml_reference import color as declaration
from fill_reference import declarations
from line_color_reference import evaluate,maths,Unresolved
ROOT=Path('.codex-work/fill-colors');XSD=Path('.codex-work/ecma376/xsd');NS={'a':A,'p':P}
read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
def checked(c,k):
 b=Path(c[k+'Path']).read_bytes();assert sha(b)==c[k+'Sha256'],(c['name'],k);return b
presets=read('.codex-work/ecma376/preset-values.json');coverage={'models':set(),'transforms':set(),'presets':set()};checks=[]
slots=resolved=unresolved=edits=contexts=targets=0;max16=0;maxfloat=0.
for c in read(ROOT/'parity.json')['cases']:
 response=json.loads(checked(c,'response'));checked(c,'source')
 if response['status']!='evaluated':continue
 req=json.loads(checked(c,'request'))
 with zipfile.ZipFile(c['sourcePath']) as z:
  raw={n:z.read(n) for n in z.namelist()};env=maths.environment(z,req['surface'])
 parsed={}
 def part(path):
  if path not in parsed:
   dom,_,ordinals=project(raw[path.lstrip('/')],with_ordinals=True);parsed[path]=(dom,ordinals,{i:n for n,i in ordinals.items()})
  return parsed[path]
 def color_term(expression,fill_kind,slot):
  term=expression['color'];origin=term['declaredBy']
  if origin['kind']=='profileDefault':
   assert expression['contextOwner'] is None and not term['transforms']
   expected={'kind':'scheme','slot':'bg1'} if fill_kind=='solid' else {'kind':'srgb','rgb':[0,0,0] if slot in [0,'foreground'] else [255,255,255]}
   assert term['value']==expected,(c['name'],term,expected)
   return {'value':expected,'transforms':[]}
  owner=origin.get('owner',origin.get('via'));assert expression['contextOwner']==owner
  source_part=origin.get('part',owner['part']);_,ords,physical=part(source_part);node=physical[origin['sourceOrdinal']]
  expected=declaration(node,ords);assert term['value']==expected['value'] and term['transforms']==expected['transforms'],c['name']
  return expected
 def placeholder_lookup(expression,holder):
  owner=expression['contextOwner']
  if owner is None:return None
  dom,ords,_=part(owner['part']);kind=owner['target']['kind'];ref=None
  if kind in ['object','line']:
   identity=next(n for n in dom.findall('.//p:cNvPr',NS) if int(n.get('id'))==owner['target']['nativeId'])
   obj=identity.getparent().getparent();ref=obj.find('p:style/a:'+('lnRef' if kind=='line' else 'fillRef'),NS)
  elif kind=='background':ref=dom.find('p:cSld/p:bg/p:bgRef',NS)
  if ref is None:return None
  r=declarations(ref,ords)
  holder['value']={'owner':owner,'referenceOrdinal':ords[ref],'colorOrdinal':r['color']['sourceOrdinal'] if r['color'] else None}
  if r['retainedOrdinals']:raise Unresolved('retainedPlaceholderContext',part=owner['part'],sourceOrdinal=r['retainedOrdinals'][0])
  return r['color']
 def verify_slot(actual,expression,kind,slot):
  global slots,resolved,unresolved,contexts,max16,maxfloat
  holder={'value':None};term=color_term(expression,kind,slot)
  expected=evaluate(term,lambda:placeholder_lookup(expression,holder),env,req['context'],presets,coverage)
  assert actual['placeholder']==holder['value'],(c['name'],actual['placeholder'],holder)
  assert actual['dependencies']==expected['dependencies'] and actual['notices']==expected['notices'],c['name']
  a=actual['outcome'];e=expected['outcome'];assert a['status']==e['status'],(c['name'],a,e)
  if e['status']=='resolved':
   assert a['rgba8']==e['rgba8'] and a['clippedForSrgb']==e['clippedForSrgb'],(c['name'],a,e)
   delta=max(abs(x-y) for x,y in zip(a['rgba16'],e['rgba16']));assert delta<=1,(c['name'],delta);max16=max(max16,delta)
   for key in ['srgb','linear']:
    for x,y in zip(a[key],e[key]):
     assert math.isclose(x,y,rel_tol=2e-12,abs_tol=2e-12),(c['name'],key,x,y);maxfloat=max(maxfloat,abs(x-y))
   resolved+=1
  else:assert a==e,(c['name'],a,e);unresolved+=1
  slots+=1;contexts+=holder['value'] is not None
 actual=response['colors'];assert actual['colorMapping']==env[1] and actual['colorScheme']==env[3]
 assert actual['sourceSha256']==req['expectedSourceSha256']==c['sourceSha256']
 assert all(actual[k]==req[k] for k in ['surface','fillProfile','colorProfile'])
 for target,record in zip(req['targets'],actual['targets'],strict=True):
  assert record['target']==target;style=record['style'];colors=record['colors'];targets+=1
  if style['status']=='unresolved':assert colors=={'kind':'unresolvedStyle'};continue
  fill=style['fill'];kind=fill['kind']
  if kind in ['none','image']:assert colors=={'kind':'none' if kind=='none' else 'imageResourcesRequired'};continue
  assert colors['kind']==kind
  if kind=='solid':verify_slot(colors['color'],fill['color'],kind,0)
  elif kind=='pattern':
   for k in ['foreground','background']:verify_slot(colors[k],fill['pattern'][k],kind,k)
  else:
   for n,(a,e) in enumerate(zip(colors['stops'],fill['gradient']['stops']['value'],strict=True)):verify_slot(a,e['color'],kind,n)
 if 'stylesPath' in c:
  baseline=json.loads(checked(c,'styles'));assert [{'target':r['target'],'outcome':r['style']} for r in actual['targets']]==baseline['styles']['targets']
 if 'editRequestPath' in c:
  edit=json.loads(checked(c,'editRequest'));checked(c,'originalSource')
  with zipfile.ZipFile(c['originalSourcePath']) as z:before={n:z.read(n) for n in z.namelist()}
  assert before.keys()==raw.keys();path=edit['edits'][0]['target']['part'].lstrip('/')
  assert all(before[p]==raw[p] for p in before if p!=path)
  a=E.fromstring(before[path]);b=E.fromstring(raw[path]);target=edit['edits'][0]['target']
  def text(dom):
   identity=next(n for n in dom.findall('.//p:cNvPr',NS) if int(n.get('id'))==target['objectId'] and not any(E.QName(parent).localname=='extLst' for parent in n.iterancestors()))
   shape=identity.getparent().getparent()
   return shape.findall('p:txBody/a:p',NS)[target['paragraph']].findall('a:r/a:t',NS)[target['run']]
  at,bt=text(a),text(b);assert at.text==edit['edits'][0]['expectedText'];assert bt.text==edit['edits'][0]['replacement'];bt.text=at.text
  assert E.tostring(a)==E.tostring(b),c['name'];edits+=1
 checks.append({'name':c['name'],'sourceSha256':c['sourceSha256'],'responseSha256':c['responseSha256'],'targets':len(actual['targets'])})
assert [len(coverage[k]) for k in ['models','transforms','presets']]==[6,28,190]
schemas={k:E.XMLSchema(E.parse(str(XSD/f),E.XMLParser(resolve_entities=False,no_network=True))) for k,f in [('surface','pml.xsd'),('theme','dml-main.xsd')]};xsd=[];valid=invalid=0
for c in read(ROOT/'manifest.json')['cases']:
 assert sha(Path(c['path']).read_bytes())==c['sha256'];count=bad=0
 with zipfile.ZipFile(c['path']) as z:
  for path in z.namelist():
   if not path.endswith('.xml'):continue
   kind='theme' if path.startswith('ppt/theme/') else 'surface' if path.startswith(('ppt/slides/','ppt/slideLayouts/','ppt/slideMasters/')) else None
   if kind:
    tree=project(z.read(path))[0]
    nonconforming=(c.get('intentionalMissingMap') and path=='ppt/slideMasters/slideMaster2.xml') or (c.get('retainedContextProbe') and path=='ppt/slides/slide1.xml')
    if nonconforming:assert not schemas[kind].validate(tree);bad+=1
    else:schemas[kind].assertValid(tree);count+=1
 valid+=count;invalid+=bad;xsd.append({'name':c['name'],'sourceSha256':c['sha256'],'validParts':count,'intentionalNonconformingParts':bad})
result={'format':'musteroffice.fill-color-independent/1','cases':checks,'targets':targets,'colorSlots':slots,'resolvedColors':resolved,'unresolvedColors':unresolved,'nativeContexts':contexts,'editedPackagesPreserved':edits,'coverage':{k:sorted(v) for k,v in coverage.items()},'maxRgba16Difference':max16,'maxWorkingChannelAbsoluteDifference':maxfloat,'workingChannelTolerance':{'absolute':2e-12,'relative':2e-12},'rgba8Tolerance':0,'rgba16Tolerance':1,'validXsdParts':valid,'intentionalNonconformingParts':invalid,'xsdCases':xsd,'xsdInputs':[{'path':str(p),'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],'presetFactsSha256':sha(Path('.codex-work/ecma376/preset-values.json').read_bytes()),'scope':'Independent XML color terms, native contexts and Decimal draft computation. Fill selection is cross-checked against the separate fill query; not a second full inheritance algorithm or Office/WPS visual acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['cases','coverage','xsdCases','xsdInputs','scope']}))
