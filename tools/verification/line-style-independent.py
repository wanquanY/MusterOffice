"""Check actual ZIP inheritance, physical provenance, edit preservation and XSD."""
import hashlib,json
from pathlib import Path
from lxml import etree as E
from line_style_reference import Package,NS
from mce_reference import project
ROOT=Path('.codex-work/line-style');XSD=Path('.codex-work/ecma376/xsd')
def sha(raw):return hashlib.sha256(raw).hexdigest()
def read(path):return json.loads(Path(path).read_text())
def checked(case,key):
 raw=Path(case[key+'Path']).read_bytes();assert sha(raw)==case[key+'Sha256'],(case['name'],key);return raw
records=[];objects=edits=xsd_parts=0
for case in read(ROOT/'parity.json')['cases']:
 response=json.loads(checked(case,'response'));source=checked(case,'source')
 if response['status']!='evaluated':continue
 request=json.loads(checked(case,'request'));package=Package(case['sourcePath'])
 expected=[{'nativeId':key,'outcome':package.resolve(request['surface'],key)} for key in request['objects']]
 assert response['styles']=={**{k:request[k] for k in ['surface','profile']},'sourceSha256':sha(source),'objects':expected},(case['name'],response['styles']['objects'],expected)
 if 'editRequestPath' in case:
  edit=json.loads(checked(case,'editRequest'));checked(case,'originalSource')
  before=Package(case['originalSourcePath']).raw;after=package.raw;assert before.keys()==after.keys()
  target=edit['edits'][0];part=target['target']['part']
  assert all(before[p]==after[p] for p in before if p!=part),case['name']
  a=E.fromstring(before[part]);b=E.fromstring(after[part]);path='p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r/a:t'
  assert b.find(path,NS).text==target['replacement'];b.find(path,NS).text=a.find(path,NS).text
  assert E.tostring(a)==E.tostring(b),case['name'];edits+=1
 objects+=len(expected);records.append({'name':case['name'],'sourceSha256':sha(source),'responseSha256':case['responseSha256'],'objects':len(expected)})
schemas={k:E.XMLSchema(E.parse(str(XSD/f),E.XMLParser(resolve_entities=False,no_network=True))) for k,f in [('surface','pml.xsd'),('theme','dml-main.xsd')]}
xsd_records=[]
for case in read(ROOT/'manifest.json')['cases']:
 assert sha(Path(case['path']).read_bytes())==case['sha256'];package=Package(case['path']);count=0;excluded=[]
 for part,raw in package.raw.items():
  if not part.endswith('.xml'):continue
  kind='theme' if part.startswith('/ppt/theme/') else 'surface' if part.startswith(('/ppt/slides/','/ppt/slideLayouts/','/ppt/slideMasters/')) else None
  if kind is None:continue
  tree=project(raw)[0]
  if case['name']=='retained-attribute' and part=='/ppt/slides/slide1.xml':
   assert not schemas[kind].validate(tree);excluded.append(part);continue
  schemas[kind].assertValid(tree);count+=1
 xsd_parts+=count;xsd_records.append({'name':case['name'],'sourceSha256':case['sha256'],'validParts':count,'intentionalNonconformingParts':excluded})
# Hand-written semantic checks in addition to independent DOM/layer selection.
def style(name):return read(ROOT/('style-'+name+'.response.json'))['styles']['objects'][0]['outcome']['line']
assert style('defaults')['width']['value']=='9525' and style('defaults')['fill']['kind']=='none'
assert style('partial')['width']['value']=='0'
assert style('partial')['compound']['declaredBy']['object']['part'].startswith('/ppt/slideLayouts/')
assert style('partial')['alignment']['declaredBy']['object']['part'].startswith('/ppt/slideMasters/')
assert style('local-style-before-parent')['width']['value']=='50800'
assert style('zero-reference-parent')['width']['value']=='114300'
assert style('empty-dash-block')['dash']['stops']==[]
assert style('miter-same-kind-parent')['join']['limit']['value']=='500000'
assert style('miter-exact')['join']['limit']['value']=='800.123456789%'
assert style('miter-zero')['join']['limit']['value']=='0'
assert style('theme-override-inherited-reference')['width']['value']=='127000'
assert style('initial-object-default-not-cascade')['width']['value']=='9525'
assert style('color-order')['fill']['color']['color']['transforms']==[{'kind':'tint','value':'12.34%'},{'kind':'alphaMod','value':'12.3456789%'},{'kind':'alpha','value':'+0050000'}]
result={'format':'musteroffice.line-style-independent/1','cases':records,'objects':objects,'editedPackagesPreserved':edits,'validXsdParts':xsd_parts,'xsdCases':xsd_records,'xsdInputs':[{'path':str(p),'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],'scope':'Owned unique-placeholder corpus; independent ZIP/DOM relationship, theme and property selection. No numeric color evaluation, render or application fidelity acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'batches':len(records),'objects':objects,'edits':edits,'validXsdParts':xsd_parts}))
