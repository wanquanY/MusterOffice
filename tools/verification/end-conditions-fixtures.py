"""Explicit end-condition corpus, independent scalar interval formulas and static controls."""
import copy,hashlib,json,random,zipfile
from fractions import Fraction as F
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/end-conditions');out=root/'fixtures';out.mkdir(parents=True,exist_ok=True)
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def put(p,v):p.write_text(json.dumps(v,indent=2)+'\n');return entry(p)
def time(n,d=1):
 n=F(n,d);assert n.denominator<=4294967295;return dict(ticks=str(n.numerator),timescale=n.denominator)
def exact(n):n=F(n);return dict(numerator=str(n.numerator),denominator=str(n.denominator))
def fraction(t):return F(int(t['ticks']),t['timescale'])
def transform(speed=100000,reverse=False,acc=0,dec=0):return dict(speedMilliPercent=speed,autoReverse=reverse,accelerationMilliPercent=acc,decelerationMilliPercent=dec)
def at(n):return dict(kind='at',offset=time(n))
def container(id,kind,children,duration,fill='hold',start=None):return dict(id=id,kind=kind,children=children,start=start or at(0),duration=duration,fill=fill)
def leaf(id,target,duration,repeat,tf,start=0,fill='hold',lo=0,hi=16200000):return dict(id=id,start=at(start),duration=time(duration),repeatMilli=repeat,fill=fill,timeTransform=tf,effect=dict(kind='rotation',target=target,**{'from':lo,'to':hi}))
def integrate(progress,tf):
 # Integrate a piecewise linear velocity ramp by trapezoid areas, then normalize
 # by its total area. This differs from production's three closed-form branches.
 a=F(tf['accelerationMilliPercent'],100000);b=F(tf['decelerationMilliPercent'],100000);area=F(0);total=F(0)
 for left,right,vl,vr in [(F(0),a,F(0),F(1)),(a,1-b,F(1),F(1)),(1-b,F(1),F(1),F(0))]:
  if left==right:continue
  total+=(right-left)*(vl+vr)/2
  t=min(right,max(left,progress));v=vl+(vr-vl)*(t-left)/(right-left);area+=(t-left)*(vl+v)/2
 return area/total
def duration(n):
 tf=n['timeTransform'];cycle=fraction(n['duration'])*(2 if tf['autoReverse']else 1)
 bounds=[]
 if n['repeatMilli']!='indefinite':bounds.append(cycle*F(n['repeatMilli'],1000))
 d=n.get('repeatDuration')
 if isinstance(d,dict):bounds.append(fraction(d))
 return min(bounds)/F(abs(tf['speedMilliPercent']),100000)if bounds else None
def progress(n,elapsed,ended,endpoint=None):
 tf=n['timeTransform'];cycle=fraction(n['duration'])*(2 if tf['autoReverse']else 1);local=elapsed*F(abs(tf['speedMilliPercent']),100000)
 if tf['speedMilliPercent']<0:
  end=endpoint if endpoint is not None else duration(n)
  assert end is not None
  local=(end-elapsed)*F(abs(tf['speedMilliPercent']),100000)
 position=local/cycle;iteration=position.numerator//position.denominator;p=position-iteration
 if p==0 and position>0 and ((tf['speedMilliPercent']>0 and ended)or(tf['speedMilliPercent']<0 and elapsed==0)):iteration-=1;p=F(1)
 if tf['autoReverse']:p=1-abs(2*p-1)
 return str(iteration),integrate(p,tf)
def after(name,event,delay):return dict(kind='after',node=name,event=event,delay=time(delay))
def click(delay=0):return dict(kind='click',target=None,delay=time(delay))
def end_bound(node,start,explicit):
 natural=duration(node);return min(explicit,start+natural)if natural is not None else explicit
base=json.loads(Path('fixtures/presentations/playback/page.json').read_text());base['page']['document']['objects']['group:1']['appearance']['stroke']={'kind':'inherit'};slide=base['page']['slide'];rng=random.Random(2026092605);author=[]
events=[dict(sequence=i+1,at=time(v),event=dict(kind='click',target=None))for i,v in enumerate([0,1,2,2,4,8])]
for i in range(24):
 is_tree=i%3!=0 or i<3;parallel=i%2==0;gate=F(1,2)if is_tree else F(0);cap=gate+F(7,4)if is_tree and i%4==0 else None
 tf=transform(rng.choice([-200000,-125000,50000,125000]),bool(i%2),*rng.choice([(0,0),(25000,50000),(50000,50000)]))
 a=leaf('a','shape:1',F(rng.randrange(1,9),4),'indefinite'if i%4==0 else 2500,tf,F(1,4),'hold',-21600000,21600000)
 a['endConditions']=[at(2),after('a','begin',F(3,2)),click(F(1,4))]
 b=leaf('b','shape:2',F(3,2),1500,transform(-200000,True,25000,0),F(1,4),'hold',2699999,8100001)
 b['endConditions']=[after('a','begin'if parallel or not is_tree else'end',2 if parallel or not is_tree else 1)]
 nodes=[a,b];timing=dict(format='musteroffice.timeline/0.2-draft'if is_tree else'musteroffice.timeline/0.1-draft',nodes=list(reversed(nodes))if i%2 else nodes)
 if is_tree:timing['tree']=dict(roots=['seq'],containers=[container('seq','parallel'if parallel else'sequence',['a','b'],dict(kind='fixed',duration=time(cap-gate))if cap is not None else dict(kind='automatic'),start=at(gate))])
 q=copy.deepcopy(base);q['page']['document']['timelines']={slide:timing};samples=[]
 for now in [F(0),F(1,3),F(1,2),F(3,4),F(1),F(5,4),F(3,2),F(7,4),F(2),F(9,4),F(5,2),F(3),F(5),F(10),F(1,3)]:
  begin={'a':gate+F(1,4)};stops=[gate+2,begin['a']+F(3,2)]
  if now>=1:stops.append(F(5,4))
  natural={'a':end_bound(a,begin['a'],min(stops))};ends={'a':min(natural['a'],cap)if cap is not None else natural['a']}
  begin['b']=gate+F(1,4)if parallel or not is_tree else ends['a']+F(1,4)
  natural['b']=end_bound(b,begin['b'],begin['a']+2 if parallel or not is_tree else ends['a']+1);ends['b']=min(natural['b'],cap)if cap is not None else natural['b']
  values={};states={};clock=min(now,cap)if cap is not None else now
  for n in nodes:
   name=n['id'];start=begin[name];end=ends[name];enabled=cap is None or start<cap
   states[name]=dict(start=exact(start)if enabled else None,end=exact(end)if enabled else None)
   if not enabled or clock<start:continue
   iteration,p=progress(n,min(clock,end)-start,clock>=end,natural[name]-start);states[name].update(iteration=iteration,progress=exact(p));e=n['effect'];values[e['target']]=exact(e['from']+(e['to']-e['from'])*p)
  samples.append(dict(at=time(now),rotations=values,expectedNodes=states))
 author.append(dict(name=f'end-{i}',page=q,samples=samples,events=events,through=time(20)))
# Nested parent clipping leaves the declared explicit reverse endpoint intact.
q=copy.deepcopy(base);nodes=[leaf('a','group:1',4,'indefinite',transform(-125000,True,25000,50000),fill='remove',lo=-10800000,hi=16200000),leaf('b','shape:1',2,1500,transform(200000,True,50000,50000),fill='hold',lo=2699999,hi=8100001)]
for n in nodes:n['endConditions']=[after(n['id'],'begin',F(5))]
q['page']['document']['timelines']={slide:dict(format='musteroffice.timeline/0.2-draft',nodes=nodes,tree=dict(roots=['outer'],containers=[container('outer','parallel',['inner'],dict(kind='fixed',duration=time(3))),container('inner','parallel',['a','b'],dict(kind='automatic'),'remove')]))}
samples=[]
for now in [F(0),F(1,3),F(1),F(3,2),F(2),F(3),F(5),F(1,3)]:
 values={}
 for n in nodes:
  natural=end_bound(n,F(0),F(5));end=min(F(3),natural);_,p=progress(n,min(now,end),now>=end,natural);e=n['effect'];values[e['target']]=exact(e['from']+(e['to']-e['from'])*p)
 samples.append(dict(at=time(now),rotations=values))
author.append(dict(name='nested-cutoff',page=q,samples=samples));put(root/'author-fixtures.json',dict(cases=author))
prior=json.loads(Path('.codex-work/source-session/fixtures.json').read_text());c=prior['cases'][0];assert entry(c['source']['path'])==c['source'];assert entry(c['fonts']['path'])==c['fonts'];q=json.loads(Path(c['request']['path']).read_text())
with zipfile.ZipFile(c['source']['path'])as z:parts={n:z.read(n)for n in z.namelist()}
P='http://schemas.openxmlformats.org/presentationml/2006/main';A='http://schemas.openxmlformats.org/drawingml/2006/main';ns=dict(p=P,a=A);part=q['page']['page']['slide'].lstrip('/');original=X.fromstring(parts[part]);cases=[];sources=[]
def pack(parts,path):
 with zipfile.ZipFile(path,'w')as z:
  for name,b in parts.items():
   info=zipfile.ZipInfo(name,(2026,9,26,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
 return entry(path)
def native_condition(parent,condition,ids):
 cond=X.SubElement(parent,f'{{{P}}}cond',delay=str(int(fraction(condition.get('offset',condition.get('delay')))*1000)))
 if condition['kind']=='click':
  cond.set('evt','onClick');target=X.SubElement(cond,f'{{{P}}}tgtEl');X.SubElement(target,f'{{{P}}}sldTgt')
 elif condition['kind']=='after':
  cond.set('evt','onBegin'if condition['event']=='begin'else'onEnd');X.SubElement(cond,f'{{{P}}}tn',val=ids[condition['node']])
for group in ['click-stop','delayed-stop','dependent-stop','sequence-stop','earliest-stop','reverse-stop','clipped-stop']:
 reverse=group in ['reverse-stop','clipped-stop'];tf=transform(-125000 if reverse else 200000,False,25000,50000);count=2500 if reverse else'indefinite';cap=F(3,2)if group=='clipped-stop'else None;sequential=group=='sequence-stop'
 tree=copy.deepcopy(original);lst=tree.find('p:timing/p:tnLst/p:par/p:cTn/p:childTnLst',ns);animations=list(lst);ids={str(i):a.find('p:cBhvr/p:cTn',ns).get('id')for i,a in enumerate(animations)};lst.clear();kind='seq'if sequential else'par';holder=X.fromstring(f'<p:{kind} xmlns:p="{P}"><p:cTn id="20" dur="{int(cap*1000)if cap is not None else "indefinite"}" restart="never" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst/></p:cTn></p:{kind}>');lst.append(holder);nodes=[]
 for i,a in enumerate(animations):
  n=leaf(str(i),f'sp.{900 if i==0 else 43}',2,count,tf);ends=[click(F(1,2)if group in ['delayed-stop','reverse-stop']else F(1,4)if group=='earliest-stop'else F(0)),at(3 if reverse else 6)]
  if group=='dependent-stop'and i==1:ends=[after('0','end',F(1,2))]
  if group=='clipped-stop':ends=[at(3)]
  if sequential:ends=[click()];n['start']=click()
  n['endConditions']=ends;nodes.append(n);a.set('from','0');a.set('to','16200000');common=a.find('p:cBhvr/p:cTn',ns);common.set('dur','2000');common.set('repeatCount',str(count));common.set('fill','remove'if cap is not None else'hold');common.set('spd',str(tf['speedMilliPercent']));common.set('autoRev','false');common.set('accel',str(tf['accelerationMilliPercent']));common.set('decel',str(tf['decelerationMilliPercent']));start=common.find('p:stCondLst',ns);start.clear();native_condition(start,n['start'],ids);end=X.SubElement(common,f'{{{P}}}endCondLst')
  for condition in ends:native_condition(end,condition,ids)
  holder.find('p:cTn/p:childTnLst',ns).append(a)
 p=dict(parts);p[part]=X.tostring(tree);source=pack(p,out/(group+'.pptx'));sources.append(source)
 for i,now in enumerate([F(0),F(1,3),F(1,2),F(1),F(5,4),F(3,2),F(2),F(5,2),F(3),F(4),F(8),F(1,3)]):
  v=copy.deepcopy(q);v['page']['page']['expectedSourceSha256']=source['sha256'];binding=dict(session=group,revision=source['sha256'],generation='11');h=dict(binding=binding,through=time(20),events=[dict(generation='11',sequence=j+1,at=time(t),event=dict(kind='click',target=None))for j,t in enumerate([1,1,2,4,8])]);v['sample']=dict(binding=binding,at=time(now),history=h);values={}
  for index,n in enumerate(nodes):
   start=(F(1)if index==0 else F(2))if sequential else F(0)
   if now<start:continue
   if sequential:explicit=F(1)if index==0 else F(4)if now>=4 else None
   elif group=='clipped-stop':explicit=F(3)
   elif group=='dependent-stop'and index==1:explicit=F(3,2)if now>=1 else F(13,2)
   else:explicit=(F(3,2)if group in ['delayed-stop','reverse-stop']else F(5,4)if group=='earliest-stop'else F(1))if now>=1 else F(3)if reverse else F(6)
   natural=end_bound(n,start,explicit)if explicit is not None else None;end=min(natural,cap)if natural is not None and cap is not None else cap if natural is None else natural;elapsed=min(now-start,end-start)if end is not None else now-start;_,value=progress(n,elapsed,end is not None and now>=end,None if natural is None else natural-start);values[n['effect']['target']]=exact(16200000*value)
  name=f'{group}-{i}';case=dict(name=name,group=group,source=source,fonts=c['fonts'],request=put(out/(name+'.json'),v),expectedRotations=values)
  if all(value['denominator']=='1'for value in values.values()):
   control=copy.deepcopy(tree);control.remove(control.find('p:timing',ns))
   for obj in control.findall('.//p:grpSp',ns)+control.findall('.//p:sp',ns):
    oid=obj.find('./*/p:cNvPr',ns).get('id');value=values.get('sp.'+oid)
    if value:obj.find(('p:grpSpPr'if X.QName(obj).localname=='grpSp'else'p:spPr')+'/a:xfrm',ns).set('rot',value['numerator'])
   cp=dict(p);cp[part]=X.tostring(control);cs=pack(cp,out/(name+'.static.pptx'));sources.append(cs);cq=copy.deepcopy(v['page']);cq['page']['expectedSourceSha256']=cs['sha256'];case['staticControl']=dict(source=cs,request=put(out/(name+'.static.json'),cq))
  cases.append(case)
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'));checked=0
for source in sources:
 with zipfile.ZipFile(source['path'])as z:
  for name in z.namelist():
   if name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/'))and name.endswith('.xml')and '/_rels/'not in name:schema.assertValid(X.fromstring(z.read(name)));checked+=1
put(root/'source-fixtures.json',dict(cases=cases,officialXsdParts=checked));print(json.dumps(dict(authorGraphs=len(author),authorSamples=sum(len(c['samples'])for c in author),sourceSamples=len(cases),officialXsdParts=checked)))
