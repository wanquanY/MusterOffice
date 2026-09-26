"""Independent Fraction/DOM/XSD oracle for the actual public runtime outputs."""
import hashlib,json,zipfile
from fractions import Fraction as F
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/timeline');report=json.loads((root/'product.json').read_text())
def entry(path):
 b=Path(path).read_bytes();return dict(path=str(path),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(record):
 assert entry(record['path'])==record;return Path(record['path']).read_bytes()
def time(v):return F(int(v['ticks']),v['timescale'])
def ratio(v):return None if v is None else F(int(v['numerator']),int(v['denominator']))
frames=properties=nodes_checked=0
for case in report['cases']:
 if case['operation']!='evaluate':continue
 q=json.loads(load(case['request']));answer=json.loads(load(case['response']))
 if answer['status']!='evaluated':continue
 nodes=q['snapshot']['document']['timelines'][q['slide']]['nodes'];at=time(q['at']);byid={n['id']:n for n in nodes}
 # Recursive event dependency solution, deliberately different from the kernel's queue.
 starts={};ends={}
 def activate(key):
  if key in starts:return starts[key]
  n=byid[key];s=n['start'];kind=s['kind']
  if kind=='at':v=time(s['offset'])
  elif kind=='after':
   parent=activate(s['node']);v=None if parent is None else (parent if s['event']=='begin' else ends[s['node']])+time(s['delay'])
  else:
   eligible=[time(e['at']) for e in q['history']['events'] if time(e['at'])<=at and e['event']['target']==s['target']]
   v=min(eligible)+time(s['delay']) if eligible else None
  starts[key]=v;ends[key]=None if v is None else v+time(n['duration'])*F(n['repeatMilli'],1000);return v
 contributions={};actual=answer['frame']['state'];assert time(actual['time'])==at
 cursor=max([e['sequence']for e in q['history']['events']if time(e['at'])<=at]or[0]);assert actual['eventCursor']==cursor
 for i,(n,f) in enumerate(zip(nodes,actual['nodes'],strict=True)):
  key=n['id'];start=activate(key);end=ends[key];nodes_checked+=1
  assert f['node']==key and ratio(f['start'])==start and ratio(f['end'])==end
  if start is None:phase='waiting'
  elif at<start:phase='scheduled'
  elif at>=end:phase='finished' if n['fill']=='remove' else 'frozen'
  else:phase='active'
  assert f['phase']==phase,(case['name'],n,f,phase)
  if phase not in ('active','frozen'):assert f['iteration'] is None and f['progress'] is None;continue
  elapsed=min(at-start,end-start);cycles=elapsed/time(n['duration']);iteration=cycles.numerator//cycles.denominator;progress=cycles-iteration
  if phase=='frozen' and progress==0:iteration-=1;progress=F(1)
  assert int(f['iteration'])==iteration and ratio(f['progress'])==progress
  effect=n['effect'];value=effect['from']+(effect['to']-effect['from'])*progress;target=effect['target'];priority=(start,i)
  if target not in contributions or priority>contributions[target][0]:contributions[target]=(priority,value)
 expected={k:v for k,(_,v)in contributions.items()};assert {k:ratio(v)for k,v in actual['rotations'].items()}==expected
 properties+=len(expected);frames+=1
 # Reduced, positive-denominator output is part of the cross-language contract.
 for v in [*actual['rotations'].values(),*[f[k]for f in actual['nodes']for k in ['start','end','progress']if f[k] is not None]]:
  r=ratio(v);assert str(r.numerator)==v['numerator'] and str(r.denominator)==v['denominator']
ns={'p':'http://schemas.openxmlformats.org/presentationml/2006/main'}
parser=X.XMLParser(resolve_entities=False,no_network=True);schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd',parser));parts=behaviors=0
for exported in report['exports']:
 q=json.loads(load(exported['request']));load(exported['output']);doc=q['document'];slide=doc['slideOrder'][0]
 with zipfile.ZipFile(exported['output']['path'])as z:
  for name in z.namelist():
   if name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/')) and name.endswith('.xml'):
    schema.assertValid(X.fromstring(z.read(name),parser));parts+=1
  tree=X.fromstring(z.read('ppt/slides/slide1.xml'),parser);names={e.get('name'):e.get('id')for e in tree.findall('.//p:cNvPr',ns)}
  actual=tree.findall('p:timing/p:tnLst/p:par/p:cTn/p:childTnLst/p:animRot',ns);expected=doc['timelines'][slide]['nodes'];assert len(actual)==len(expected)
  ids={n['id']:str(i+2)for i,n in enumerate(expected)}
  for a,n in zip(actual,expected):
   behaviors+=1;assert int(a.get('from'))==n['effect']['from'] and int(a.get('to'))==n['effect']['to']
   b=a.find('p:cBhvr',ns);c=b.find('p:cTn',ns);assert c.get('id')==ids[n['id']];assert int(c.get('dur'))==time(n['duration'])*1000;assert int(c.get('repeatCount'))==n['repeatMilli'];assert c.get('fill')==n['fill'] and c.get('restart')=='never'
   assert b.find('p:tgtEl/p:spTgt',ns).get('spid')==names[n['effect']['target']]
   cond=c.find('p:stCondLst/p:cond',ns);s=n['start'];assert int(cond.get('delay'))==time(s.get('offset',s.get('delay')))*1000
   if s['kind']=='after':assert cond.get('evt')=={'begin':'onBegin','end':'onEnd'}[s['event']] and cond.find('p:tn',ns).get('val')==ids[s['node']]
   elif s['kind']=='click':
    assert cond.get('evt')=='onClick';target=cond.find('p:tgtEl',ns)
    if s['target'] is None:assert target.find('p:sldTgt',ns)is not None
    else:assert target.find('p:spTgt',ns).get('spid')==names[s['target']]
   else:assert cond.get('evt')is None and len(cond)==0
out=dict(format='musteroffice.timeline-reference/1',product=entry(root/'product.json'),frames=frames,nodes=nodes_checked,properties=properties,xsdParts=parts,nativeBehaviors=behaviors)
(root/'reference.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out))
