"""Verify real line color composition, dependencies, XML preservation and XSD."""
import hashlib,json,math,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import project
from line_style_reference import Package,NS
from line_color_reference import evaluate,maths
ROOT=Path('.codex-work/line-colors');XSD=Path('.codex-work/ecma376/xsd')
def sha(raw):return hashlib.sha256(raw).hexdigest()
def read(p):return json.loads(Path(p).read_text())
def checked(c,k):
 b=Path(c[k+'Path']).read_bytes();assert sha(b)==c[k+'Sha256'],(c['name'],k);return b
presets=read('.codex-work/ecma376/preset-values.json');coverage={'models':set(),'transforms':set(),'presets':set()};records=[];objects=edits=resolved=unresolved=0;max16=0;maxfloat=0.
for c in read(ROOT/'parity.json')['cases']:
 response=json.loads(checked(c,'response'));checked(c,'source')
 if response['status']!='evaluated':continue
 req=json.loads(checked(c,'request'));package=Package(c['sourcePath'])
 with zipfile.ZipFile(c['sourcePath']) as z:env=maths.environment(z,req['surface'])
 actual=response['colors'];assert actual['colorMapping']==env[1] and actual['colorScheme']==env[3]
 assert actual['sourceSha256']==req['expectedSourceSha256']==c['sourceSha256']
 assert all(actual[k]==req[k] for k in ['surface','lineProfile','colorProfile'])
 assert len(actual['objects'])==len(req['objects'])
 for identity,record in zip(req['objects'],actual['objects'],strict=True):
  expected=package.resolve(req['surface'],identity);assert record['nativeId']==identity and record['style']==expected,(c['name'],identity)
  if expected['status']=='unresolved':paint={'kind':'unresolvedStyle'}
  elif expected['line']['fill']['kind']=='none':paint={'kind':'none'}
  else:
   expr=expected['line']['fill']['color'];paint=evaluate(expr['color'],expr['placeholder'],env,req['context'],presets,coverage)
  if paint['kind']=='solid' and paint['outcome']['status']=='resolved':
   a=record['paint']['outcome'];e=paint['outcome'];assert a['status']=='resolved',(c['name'],a,e)
   assert a['rgba8']==e['rgba8'] and a['clippedForSrgb']==e['clippedForSrgb'],(c['name'],a,e)
   delta=max(abs(x-y) for x,y in zip(a['rgba16'],e['rgba16']));assert delta<=1,(c['name'],delta);max16=max(max16,delta)
   for k in ['srgb','linear']:
    for x,y in zip(a[k],e[k]):
     assert math.isclose(x,y,rel_tol=2e-12,abs_tol=2e-12),(c['name'],k,x,y)
     maxfloat=max(maxfloat,abs(x-y))
   assert record['paint']['dependencies']==paint['dependencies'] and record['paint']['notices']==paint['notices'],(c['name'],record['paint'],paint)
   resolved+=1
  else:
   assert record['paint']==paint,(c['name'],record['paint'],paint)
   if paint['kind']=='solid':unresolved+=1
  objects+=1
 if 'editRequestPath' in c:
  edit=json.loads(checked(c,'editRequest'));checked(c,'originalSource');before=Package(c['originalSourcePath']).raw;after=package.raw;assert before.keys()==after.keys()
  part=edit['edits'][0]['target']['part'];assert all(before[p]==after[p] for p in before if p!=part)
  a=E.fromstring(before[part]);b=E.fromstring(after[part]);path='p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r/a:t'
  assert b.find(path,NS).text==edit['edits'][0]['replacement'];b.find(path,NS).text=a.find(path,NS).text;assert E.tostring(a)==E.tostring(b);edits+=1
 records.append({'name':c['name'],'sourceSha256':c['sourceSha256'],'responseSha256':c['responseSha256'],'objects':len(actual['objects'])})
assert [len(coverage[k]) for k in ['models','transforms','presets']]==[6,28,190]
schemas={k:E.XMLSchema(E.parse(str(XSD/f),E.XMLParser(resolve_entities=False,no_network=True))) for k,f in [('surface','pml.xsd'),('theme','dml-main.xsd')]};xsd=[];parts=0
for c in read(ROOT/'manifest.json')['cases']:
 p=Package(c['path']);assert sha(Path(c['path']).read_bytes())==c['sha256'];count=0;excluded=[]
 for part,raw in p.raw.items():
  if not part.endswith('.xml'):continue
  kind='theme' if part.startswith('/ppt/theme/') else 'surface' if part.startswith(('/ppt/slides/','/ppt/slideLayouts/','/ppt/slideMasters/')) else None
  if kind:
   tree=project(raw)[0]
   if c['name'] in ['color-missing-map','color-direct-slot-no-map'] and part=='/ppt/slideMasters/slideMaster2.xml':
    assert not schemas[kind].validate(tree);assert tree.find('p:clrMap',NS) is None;excluded.append(part);continue
   schemas[kind].assertValid(tree);count+=1
 parts+=count;xsd.append({'name':c['name'],'sourceSha256':c['sha256'],'parts':count,'intentionalNonconformingParts':excluded})
result={'format':'musteroffice.line-color-independent/1','cases':records,'objects':objects,'resolvedColors':resolved,'unresolvedColors':unresolved,'editedPackagesPreserved':edits,'coverage':{k:sorted(v) for k,v in coverage.items()},'maxRgba16Difference':max16,'maxWorkingChannelAbsoluteDifference':maxfloat,'workingChannelTolerance':{'absolute':2e-12,'relative':2e-12},'rgba8Tolerance':0,'rgba16Tolerance':1,'validXsdParts':parts,'xsdCases':xsd,'xsdInputs':[{'path':str(p),'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],'presetFactsSha256':sha(Path('.codex-work/ecma376/preset-values.json').read_bytes()),'scope':'Independent ZIP/XML provenance and Decimal draft computation, not Office/WPS color or render fidelity.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['cases','coverage','xsdCases','xsdInputs','scope']}))
