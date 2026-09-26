"""Inspect exported native XML directly against author declarations and fixed XSD."""
import hashlib,json,zipfile
from pathlib import Path
from fractions import Fraction as F
from lxml import etree as X
root=Path('.codex-work/container-lifecycle');product=json.loads((root/'product.json').read_text());schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
P='http://schemas.openxmlformats.org/presentationml/2006/main';ns={'p':P};records=[];parts=0;entries=0
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def milliseconds(t):
 n=F(int(t['ticks']),t['timescale'])*1000;assert n.denominator==1;return str(n.numerator)
for export in product['exports']:
 assert entry(export['output']['path'])==export['output'];q=json.loads(Path(export['request']['path']).read_text());doc=q['document'];timeline=doc['timelines'][doc['slideOrder'][0]];tree=timeline['tree'];nodes=timeline['nodes']+tree['containers'];ids={n['id']:str(i+2)for i,n in enumerate(nodes)}
 with zipfile.ZipFile(export['output']['path'])as z:
  slide=X.fromstring(z.read('ppt/slides/slide1.xml'));root_list=slide.find('p:timing/p:tnLst/p:par/p:cTn/p:childTnLst',ns)
  native={};parents={}
  def walk(lst,parent):
   if lst is None:return []
   children=[]
   for node in lst:
    kind=X.QName(node).localname;common=node.find('p:cBhvr/p:cTn'if kind=='animRot'else'p:cTn',ns);id=common.get('id');assert id not in native
    native[id]=(kind,node,common);parents[id]=parent;children.append(id);walk(common.find('p:childTnLst',ns),id)
   return children
  roots=walk(root_list,None);assert roots==[ids[id]for id in tree['roots']];assert len(native)==len(nodes)
  for node in nodes:
   id=ids[node['id']];kind,native_node,common=native[id];assert common.get('restart')=='never';assert common.get('fill')==node['fill'];start=node['start'];condition=common.find('p:stCondLst/p:cond',ns)
   assert start['kind']=='at';assert condition.attrib=={'delay':milliseconds(start['offset'])}
   if 'effect'in node:
    assert kind=='animRot';assert common.get('dur')==milliseconds(node['duration']);assert common.get('repeatCount')==str(node['repeatMilli']);assert common.get('repeatDur')==(None if 'repeatDuration'not in node else 'indefinite'if node['repeatDuration']=='indefinite'else milliseconds(node['repeatDuration']));assert native_node.get('from')==str(node['effect']['from']);assert native_node.get('to')==str(node['effect']['to'])
    tf=node['timeTransform'];assert common.get('spd')==str(tf['speedMilliPercent']);assert common.get('autoRev')==('1'if tf['autoReverse']else'0');assert common.get('accel')==str(tf['accelerationMilliPercent']);assert common.get('decel')==str(tf['decelerationMilliPercent'])
   else:
    assert kind=={'parallel':'par','sequence':'seq'}[node['kind']];children=common.find('p:childTnLst',ns)
    actual=[]if children is None else [v.find('p:cBhvr/p:cTn'if X.QName(v).localname=='animRot'else'p:cTn',ns).get('id')for v in children]
    assert actual==[ids[id]for id in node['children']]
    d=node['duration'];assert common.get('dur')==('indefinite'if d['kind']!='fixed'else milliseconds(d['duration']))
    if d['kind']=='automatic':
     end=common.find('p:endSync',ns);assert end.attrib=={'evt':'end','delay':'0'};assert end.find('p:rtn',ns).attrib=={'val':'all'}
    else:assert common.find('p:endSync',ns)is None
   expected_ends=node.get('endConditions',[]);end_list=common.find('p:endCondLst',ns)
   assert (end_list is not None)==bool(expected_ends)
   actual_ends=[]if end_list is None else list(end_list);assert len(actual_ends)==len(expected_ends)
   for expected,actual in zip(expected_ends,actual_ends):
    assert X.QName(actual).localname=='cond'
    if expected['kind']=='at':assert actual.attrib=={'delay':milliseconds(expected['offset'])}and len(actual)==0
    elif expected['kind']=='after':
     assert actual.attrib=={'evt':{'begin':'onBegin','end':'onEnd'}[expected['event']],'delay':milliseconds(expected['delay'])}
     assert len(actual)==1 and actual.find('p:tn',ns).attrib=={'val':ids[expected['node']]}
    else:
     assert expected['kind']=='click'and expected['target']is None
     assert actual.attrib=={'evt':'onClick','delay':milliseconds(expected['delay'])}
     assert len(actual)==1 and actual.find('p:tgtEl/p:sldTgt',ns)is not None
   entries+=1
  for name in z.namelist():
   if name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/'))and name.endswith('.xml')and '/_rels/'not in name:schema.assertValid(X.fromstring(z.read(name)));parts+=1
 records.append(dict(name=export['name'],output=export['output'],nativeEntries=len(native)))
report=dict(format='musteroffice.container-lifecycle-native-reference/1',exports=records,officialXsdParts=parts,checkedNativeEntries=entries)
(root/'native-reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
