"""Owned nested clocks with closed-form Fraction references and native XML controls."""
import copy,hashlib,json,random,zipfile
from fractions import Fraction as F
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/timing-tree');out=root/'fixtures';out.mkdir(parents=True,exist_ok=True)
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def put(p,v):p.write_text(json.dumps(v,indent=2)+'\n');return entry(p)
def time(n,d=1):
 n=F(n,d);return dict(ticks=str(n.numerator),timescale=n.denominator)
def exact(n):n=F(n);return dict(numerator=str(n.numerator),denominator=str(n.denominator))
def at(n):return dict(kind='at',offset=time(n))
def container(id,kind,children,duration,fill='hold',start=None):return dict(id=id,kind=kind,children=children,start=start or at(0),duration=duration,fill=fill)
def leaf(id,target,start,duration,fill='hold',lo=0,hi=21600000):return dict(id=id,start=at(start),duration=time(duration),repeatMilli=1000,fill=fill,effect=dict(kind='rotation',target=target,**{'from':lo,'to':hi}))
base=json.loads(Path('fixtures/presentations/playback/page.json').read_text());base['page']['document']['objects']['group:1']['appearance']['stroke']={'kind':'inherit'};slide=base['page']['slide'];rng=random.Random(2026092602);author=[]
# Two behaviors in a fixed sequential scope. The oracle uses the explicit interval
# formula sA=P+A; sB=sA+dA+B, not the production event-DAG implementation.
for i in range(24):
 P=F(rng.randrange(0,7),4);A=F(rng.randrange(0,7),8);B=F(rng.randrange(0,7),8)
 da=F(rng.randrange(1,10),4);db=F(rng.randrange(1,10),4);cap=P+F(rng.randrange(1,31),4)
 sa=P+A;ea=sa+da;sb=ea+B;eb=sb+db
 fills=[rng.choice(['remove','freeze','hold'])for _ in range(3)]
 nodes=[leaf('a','shape:1',A,da,fills[0],-21600000,21600000),leaf('b','shape:2',B,db,fills[1],2699999,8100001)]
 timing=dict(format='musteroffice.timeline/0.2-draft',nodes=list(reversed(nodes)) if i%2 else nodes,tree=dict(roots=['seq'],containers=[container('seq','sequence',['a','b'],dict(kind='fixed',duration=time(cap-P)),fills[2],at(P))]))
 q=copy.deepcopy(base);q['page']['document']['timelines']={slide:timing}
 samples=[]
 for now in sorted(set([F(0),F(1,3),sa,(sa+ea)/2,ea,sb,(sb+eb)/2,eb,cap,cap+1,F(80)])):
  values={};clock=min(now,cap);parent_visible=now>=P and (now<cap or fills[2]!='remove')
  for name,target,s,e,fill,lo,hi,nextstart in [('a','shape:1',sa,ea,fills[0],-21600000,21600000,sb),('b','shape:2',sb,eb,fills[1],2699999,8100001,None)]:
   if not parent_visible or s>=cap or clock<s:continue
   if clock<min(e,cap):value=F(lo)+(hi-lo)*(clock-s)/(e-s)
   elif e>cap:value=F(lo)+(hi-lo)*(cap-s)/(e-s)
   elif fill=='remove' or (fill=='freeze' and nextstart is not None and nextstart<cap and clock>=nextstart):continue
   else:value=F(hi)
   values[target]=exact(value)
  samples.append(dict(at=time(now),rotations=values))
 author.append(dict(name=f'sequence-{i}',page=q,samples=samples,intervals={name:dict(start=exact(s)if s<cap else None,end=exact(min(e,cap))if s<cap else None)for name,s,e in [('a',sa,ea),('b',sb,eb)]}))
# One exportable author tree also exercises nested automatic duration and a
# parallel branch. Its static transforms are checked through public rendering.
q=copy.deepcopy(base);t=q['page']['document']['timelines'][slide];t['format']='musteroffice.timeline/0.2-draft'
t['nodes'][0]['fill']='remove';t['nodes'][0]['duration']=time(10)
t['nodes'][1]['fill']='hold';t['nodes'][1]['duration']=time(2)
t['tree']=dict(roots=['outer'],containers=[container('outer','parallel',['inner'],dict(kind='fixed',duration=time(3))),container('inner','parallel',[n['id']for n in t['nodes']],dict(kind='automatic'),'remove')])
samples=[]
for n in [F(0),F(1,3),F(1),F(2),F(3),F(4),F(1,3)]:
 values={'group:1':exact(-21600000+43200000*min(n,F(3))/10),'shape:1':exact(2699999+2*min(n,F(2))/2)}
 samples.append(dict(at=time(n),rotations=values))
author.append(dict(name='nested-cutoff',page=q,samples=samples))
put(root/'author-fixtures.json',dict(cases=author))
# Reuse a sealed, owned image + glyph + anisotropic group resource fixture.
prior=json.loads(Path('.codex-work/source-session/fixtures.json').read_text());c=prior['cases'][0]
assert entry(c['source']['path'])==c['source'];assert entry(c['fonts']['path'])==c['fonts']
q=json.loads(Path(c['request']['path']).read_text())
with zipfile.ZipFile(c['source']['path'])as z:parts={n:z.read(n)for n in z.namelist()}
P='http://schemas.openxmlformats.org/presentationml/2006/main';A='http://schemas.openxmlformats.org/drawingml/2006/main';ns=dict(p=P,a=A);slide=q['page']['page']['slide'].lstrip('/')
original=X.fromstring(parts[slide]);list_path='p:timing/p:tnLst/p:par/p:cTn/p:childTnLst'
def pack(parts,path):
 with zipfile.ZipFile(path,'w')as z:
  for name,b in parts.items():
   info=zipfile.ZipInfo(name,(2026,9,26,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
 return entry(path)
def par(kind,id,dur,fill,start,children):
 node=X.fromstring(f'<p:{kind} xmlns:p="{P}"><p:cTn id="{id}" dur="{dur}" restart="never" fill="{fill}"><p:stCondLst>{start}</p:stCondLst><p:childTnLst/></p:cTn></p:{kind}>')
 for child in children:node.find('p:cTn/p:childTnLst',ns).append(child)
 return node
cases=[];sources=[]
for variant in ['sequence-freeze','sequence-hold','nested-cutoff','click-sequence']:
 tree=copy.deepcopy(original);lst=tree.find(list_path,ns);children=list(lst);lst.clear()
 for child in children:
  common=child.find('p:cBhvr/p:cTn',ns);common.set('dur','2000');common.set('fill','hold');common.find('p:stCondLst/p:cond',ns).set('delay','0');child.set('from','0');child.set('to','16200000')
 if variant.startswith('sequence'):
  children[0].find('p:cBhvr/p:cTn',ns).set('fill',variant.split('-')[1]);lst.append(par('seq',20,6000,'hold','<p:cond delay="500"/>',children))
 elif variant=='nested-cutoff':
  children[0].find('p:cBhvr/p:cTn',ns).set('dur','10000');children[0].find('p:cBhvr/p:cTn',ns).set('fill','remove')
  inner=par('par',21,'indefinite','remove','<p:cond delay="0"/>',children);inner.find('p:cTn',ns).insert(1,X.fromstring(f'<p:endSync xmlns:p="{P}" evt="end" delay="0"><p:rtn val="all"/></p:endSync>'))
  lst.append(par('par',20,3000,'hold','<p:cond delay="0"/>',[inner]))
 else:
  for child in children:
   cond=child.find('p:cBhvr/p:cTn/p:stCondLst/p:cond',ns);cond.set('evt','onClick');cond.append(X.fromstring(f'<p:tgtEl xmlns:p="{P}"><p:sldTgt/></p:tgtEl>'))
  lst.append(par('seq',20,'indefinite','hold','<p:cond evt="onClick" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>',children))
 p=dict(parts);p[slide]=X.tostring(tree);source=pack(p,out/(variant+'.pptx'));sources.append(source)
 times=[F(0),F(1,3),F(1,2),F(1),F(3,2),F(2),F(5,2),F(3),F(4),F(9,2),F(7),F(1,3)]
 for i,now in enumerate(times):
  v=copy.deepcopy(q);v['page']['page']['expectedSourceSha256']=source['sha256'];binding=dict(session=variant,revision=source['sha256'],generation='11');v['sample']=dict(binding=binding,at=time(now),history=None);values={}
  if variant.startswith('sequence'):
   for target,start in [(900,F(1,2)),(43,F(5,2))]:
    if now>=start and not(target==900 and variant=='sequence-freeze'and now>=F(5,2)):values[f'sp.{target}']=exact(16200000*min(now-start,F(2))/2)
  elif variant=='nested-cutoff':values={'sp.900':exact(16200000*min(now,F(3))/10),'sp.43':exact(16200000*min(now,F(2))/2)}
  else:
   events=[dict(generation='11',sequence=j+1,at=time(t),event=dict(kind='click',target=None))for j,t in enumerate([F(1),F(1),F(3),F(4)])]
   v['sample']['history']=dict(binding=binding,through=time(20),events=events)
   for target,start in [(900,F(1)),(43,F(3))]:
    if now>=start:values[f'sp.{target}']=exact(16200000*min(now-start,F(2))/2)
  name=f'{variant}-{i}';case=dict(name=name,group=variant,source=source,fonts=c['fonts'],request=put(out/(name+'.json'),v),expectedRotations=values)
  if i in [3,5,7,9,10]and all(value['denominator']=='1'for value in values.values()):
   control=copy.deepcopy(tree);control.remove(control.find('p:timing',ns))
   for object in control.findall('.//p:grpSp',ns)+control.findall('.//p:sp',ns):
    id=object.find('./*/p:cNvPr',ns).get('id');value=values.get('sp.'+id)
    if value:object.find(('p:grpSpPr'if X.QName(object).localname=='grpSp'else'p:spPr')+'/a:xfrm',ns).set('rot',value['numerator'])
   cp=dict(p);cp[slide]=X.tostring(control);cs=pack(cp,out/(name+'.static.pptx'));sources.append(cs);cq=copy.deepcopy(v['page']);cq['page']['expectedSourceSha256']=cs['sha256'];case['staticControl']=dict(source=cs,request=put(out/(name+'.static.json'),cq))
  cases.append(case)
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'));checked=0
for source in sources:
 with zipfile.ZipFile(source['path'])as z:
  for name in z.namelist():
   if name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/'))and name.endswith('.xml')and '/_rels/'not in name:schema.assertValid(X.fromstring(z.read(name)));checked+=1
put(root/'source-fixtures.json',dict(cases=cases,officialXsdParts=checked))
print(json.dumps(dict(authorGraphs=len(author),authorSamples=sum(len(c['samples'])for c in author),sourceSamples=len(cases),officialXsdParts=checked)))
