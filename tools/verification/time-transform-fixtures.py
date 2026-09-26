"""Exact behavior-clock corpus; easing reference integrates velocity trapezoids."""
import copy,hashlib,json,random,zipfile
from fractions import Fraction as F
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/time-transform');out=root/'fixtures';out.mkdir(parents=True,exist_ok=True)
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
 tf=n['timeTransform'];return fraction(n['duration'])*(2 if tf['autoReverse']else 1)*F(n['repeatMilli'],1000)/F(abs(tf['speedMilliPercent']),100000)
def progress(n,elapsed,ended):
 tf=n['timeTransform'];cycle=fraction(n['duration'])*(2 if tf['autoReverse']else 1);length=cycle*F(n['repeatMilli'],1000);local=elapsed*F(abs(tf['speedMilliPercent']),100000)
 if tf['speedMilliPercent']<0:local=length-local
 position=local/cycle;iteration=position.numerator//position.denominator;p=position-iteration
 if p==0 and position>0 and ((tf['speedMilliPercent']>0 and ended)or(tf['speedMilliPercent']<0 and elapsed==0)):iteration-=1;p=F(1)
 if tf['autoReverse']:p=1-abs(2*p-1)
 return str(iteration),integrate(p,tf)
base=json.loads(Path('fixtures/presentations/playback/page.json').read_text());base['page']['document']['objects']['group:1']['appearance']['stroke']={'kind':'inherit'};slide=base['page']['slide'];rng=random.Random(2026092603);author=[]
for i in range(32):
 tf=[transform(125000,False,25000,50000),transform(-125000,False,50000,25000),transform(200000,True,25000,25000)][i%3] if i<3 else transform(rng.choice([-200000,-50000,-33333,1,25000,100000,200000]),bool(i%2),*rng.choice([(0,0),(100000,0),(0,100000),(50000,50000),(1,99999),(33333,33333)]))
 nodes=[leaf('a','shape:1',F(rng.randrange(1,9),4),rng.choice([1,500,1000,1500,2000,2500,3001]),tf,0,'hold',-21600000,21600000),leaf('b','shape:2',F(3,2),1500,transform(-200000,True,25000,0),F(1,4),'hold',2699999,8100001)]
 is_tree=i%3!=0 or i<3;parallel=i%2==0;parent_start=F(1,2)if is_tree else F(0);cap=parent_start+F(5,2)if is_tree and i%4==0 else None
 if not is_tree:nodes[1]['start']=at(F(1,2))
 begin={'a':parent_start,'b':parent_start+fraction(nodes[1]['start']['offset'])+(F(0)if parallel or not is_tree else duration(nodes[0]))};natural={n['id']:begin[n['id']]+duration(n)for n in nodes}
 timing=dict(format='musteroffice.timeline/0.2-draft'if is_tree else'musteroffice.timeline/0.1-draft',nodes=list(reversed(nodes))if i%2 else nodes)
 if is_tree:timing['tree']=dict(roots=['seq'],containers=[container('seq','parallel'if parallel else'sequence',['a','b'],dict(kind='fixed',duration=time(cap-parent_start))if cap is not None else dict(kind='automatic'),start=at(parent_start))])
 q=copy.deepcopy(base);q['page']['document']['timelines']={slide:timing};samples=[];points={F(0),F(1,3),F(1,2),F(1),F(2),F(4),F(10)}
 for n in nodes:
  start=begin[n['id']];end=natural[n['id']];points|={start,(start+end)/2,end,end+F(1,3)}
 if cap is not None:points|={cap,cap+1}
 for now in sorted(points)+[F(1,3)]:
  values={};states={};clock=min(now,cap)if cap is not None else now
  for n in nodes:
   start=begin[n['id']];end=natural[n['id']];active_start=start if cap is None or start<cap else None;active_end=min(end,cap)if active_start is not None and cap is not None else end if active_start is not None else None
   states[n['id']]=dict(start=exact(active_start)if active_start is not None else None,end=exact(active_end)if active_end is not None else None)
   if active_start is None or clock<start:continue
   iteration,p=progress(n,min(clock,active_end)-start,clock>=active_end);states[n['id']].update(iteration=iteration,progress=exact(p));effect=n['effect'];values[effect['target']]=exact(effect['from']+(effect['to']-effect['from'])*p)
  samples.append(dict(at=time(now),rotations=values,expectedNodes=states))
 author.append(dict(name=f'clock-{i}',page=q,samples=samples))
# Mixed group/child frame with ancestor cutoff on reversed, eased animation.
q=copy.deepcopy(base);nodes=[leaf('a','group:1',4,2500,transform(-125000,True,25000,50000),fill='remove',lo=-10800000,hi=16200000),leaf('b','shape:1',2,1500,transform(200000,True,50000,50000),fill='hold',lo=2699999,hi=8100001)];q['page']['document']['timelines']={slide:dict(format='musteroffice.timeline/0.2-draft',nodes=nodes,tree=dict(roots=['outer'],containers=[container('outer','parallel',['inner'],dict(kind='fixed',duration=time(3))),container('inner','parallel',['a','b'],dict(kind='automatic'),'remove')]))}
samples=[]
for now in [F(0),F(1,3),F(1),F(3,2),F(2),F(3),F(5),F(1,3)]:
 values={}
 for n in nodes:
  elapsed=min(now,F(3),duration(n));_,p=progress(n,elapsed,now>=min(F(3),duration(n)));values[n['effect']['target']]=exact(n['effect']['from']+(n['effect']['to']-n['effect']['from'])*p)
 samples.append(dict(at=time(now),rotations=values))
author.append(dict(name='nested-cutoff',page=q,samples=samples));put(root/'author-fixtures.json',dict(cases=author))
# Sealed source image/text group; native attributes use both integer and decimal
# percent spellings and explicit autoRev values.
prior=json.loads(Path('.codex-work/source-session/fixtures.json').read_text());c=prior['cases'][0];assert entry(c['source']['path'])==c['source'];assert entry(c['fonts']['path'])==c['fonts'];q=json.loads(Path(c['request']['path']).read_text())
with zipfile.ZipFile(c['source']['path'])as z:parts={n:z.read(n)for n in z.namelist()}
P='http://schemas.openxmlformats.org/presentationml/2006/main';A='http://schemas.openxmlformats.org/drawingml/2006/main';ns=dict(p=P,a=A);part=q['page']['page']['slide'].lstrip('/');original=X.fromstring(parts[part]);cases=[];sources=[]
def pack(parts,path):
 with zipfile.ZipFile(path,'w')as z:
  for name,b in parts.items():
   info=zipfile.ZipInfo(name,(2026,9,26,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
 return entry(path)
for group,tf,repeat,cap,click in [('forward-ease',transform(125000,False,25000,50000),2500,None,False),('reverse-repeat',transform(-125000,False,50000,25000),2500,None,False),('auto-reverse',transform(200000,True,25000,25000),1500,None,False),('clipped-reverse',transform(-125000,True,25000,50000),2500,F(3),False),('click-clock',transform(200000,True,25000,25000),1500,None,True)]:
 tree=copy.deepcopy(original);lst=tree.find('p:timing/p:tnLst/p:par/p:cTn/p:childTnLst',ns);animations=list(lst);lst.clear();kind='seq'if click else'par';holder=X.fromstring(f'<p:{kind} xmlns:p="{P}"><p:cTn id="20" dur="{int(cap*1000)if cap is not None else "indefinite"}" restart="never" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst/></p:cTn></p:{kind}>');lst.append(holder);nodes=[]
 for i,a in enumerate(animations):
  a.set('from','0');a.set('to','16200000');common=a.find('p:cBhvr/p:cTn',ns);common.set('dur','2000');common.set('repeatCount',str(repeat));common.set('fill','remove'if cap is not None else'hold');common.set('spd',f'{tf["speedMilliPercent"]/1000:.3f}%'if i==0 else str(tf['speedMilliPercent']));common.set('autoRev','true'if tf['autoReverse']else'false');common.set('accel',str(tf['accelerationMilliPercent']));common.set('decel',str(tf['decelerationMilliPercent']));cond=common.find('p:stCondLst/p:cond',ns);cond.set('delay','0')
  if click:cond.set('evt','onClick');cond.append(X.fromstring(f'<p:tgtEl xmlns:p="{P}"><p:sldTgt/></p:tgtEl>'))
  holder.find('p:cTn/p:childTnLst',ns).append(a);nodes.append(leaf(str(i),f'sp.{900 if i==0 else 43}',2,repeat,tf))
 p=dict(parts);p[part]=X.tostring(tree);source=pack(p,out/(group+'.pptx'));sources.append(source)
 times=[F(0),F(1,3),F(1,2),F(1),F(3,2),F(2),F(5,2),F(3),F(4),F(5),F(8),F(1,3)]
 for i,now in enumerate(times):
  v=copy.deepcopy(q);v['page']['page']['expectedSourceSha256']=source['sha256'];binding=dict(session=group,revision=source['sha256'],generation='11');v['sample']=dict(binding=binding,at=time(now),history=None);values={}
  if click:v['sample']['history']=dict(binding=binding,through=time(20),events=[dict(generation='11',sequence=j+1,at=time(t),event=dict(kind='click',target=None))for j,t in enumerate([F(1),F(2),F(4),F(8)])])
  for index,n in enumerate(nodes):
   start=(F(1)if index==0 else F(4))if click else F(0)
   if now<start:continue
   elapsed=min(now-start,duration(n),cap if cap is not None else duration(n));_,progress_value=progress(n,elapsed,elapsed==duration(n)or(cap is not None and now>=cap));values[n['effect']['target']]=exact(16200000*progress_value)
  name=f'{group}-{i}';case=dict(name=name,group=group,source=source,fonts=c['fonts'],request=put(out/(name+'.json'),v),expectedRotations=values)
  if all(value['denominator']=='1'for value in values.values()):
   control=copy.deepcopy(tree);control.remove(control.find('p:timing',ns))
   for object in control.findall('.//p:grpSp',ns)+control.findall('.//p:sp',ns):
    id=object.find('./*/p:cNvPr',ns).get('id');value=values.get('sp.'+id)
    if value:object.find(('p:grpSpPr'if X.QName(object).localname=='grpSp'else'p:spPr')+'/a:xfrm',ns).set('rot',value['numerator'])
   cp=dict(p);cp[part]=X.tostring(control);cs=pack(cp,out/(name+'.static.pptx'));sources.append(cs);cq=copy.deepcopy(v['page']);cq['page']['expectedSourceSha256']=cs['sha256'];case['staticControl']=dict(source=cs,request=put(out/(name+'.static.json'),cq))
  cases.append(case)
schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'));checked=0
for source in sources:
 with zipfile.ZipFile(source['path'])as z:
  for name in z.namelist():
   if name.startswith(('ppt/slides/','ppt/slideMasters/','ppt/slideLayouts/'))and name.endswith('.xml')and '/_rels/'not in name:schema.assertValid(X.fromstring(z.read(name)));checked+=1
put(root/'source-fixtures.json',dict(cases=cases,officialXsdParts=checked));print(json.dumps(dict(authorGraphs=len(author),authorSamples=sum(len(c['samples'])for c in author),sourceSamples=len(cases),officialXsdParts=checked)))
