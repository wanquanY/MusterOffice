"""Explicit end-condition corpus, independent scalar interval formulas and static controls."""
import copy,hashlib,json,random,zipfile
from fractions import Fraction as F
from pathlib import Path
from lxml import etree as X
root=Path('.codex-work/container-lifecycle');out=root/'fixtures';out.mkdir(parents=True,exist_ok=True)
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
base=json.loads(Path('fixtures/presentations/playback/page.json').read_text());base['page']['document']['objects']['group:1']['appearance']['stroke']={'kind':'inherit'};slide=base['page']['slide'];author=[];rng=random.Random(2026092606)
events=[dict(sequence=i+1,at=time(v),event=dict(kind='click',target=None))for i,v in enumerate([0,2,2,4,8])]
for i in range(25):
 parallel=i%2==0;gate=F(1,2);kind=i%4
 a=leaf('a','shape:1',F(rng.randrange(2,9),4),2000,transform(125000,bool(i%3),25000,25000),F(1,4),'hold',-21600000,21600000)
 b=leaf('b','shape:2',F(3,2),2500,transform(-200000,True,25000,0),F(1,4),'remove'if i%3==0 else'hold',2699999,8100001)
 # This reference also exercises transitive atomic deletion; it cannot be the
 # winning bound because the finite repeat duration is shorter.
 b['endConditions']=[after('a','begin',100)]
 end=[at(2)]if kind==0 else[after('a','begin',F(7,4))]if kind==1 else[after('a','end',F(3,4))]if kind==2 else[click(F(1,4)),at(8)]
 inner=container('inner','parallel'if parallel else'sequence',['a','b'],dict(kind='automatic'))
 outer=container('seq','parallel',['inner'],dict(kind='automatic'if i%3==0 else'indefinite'),start=at(gate));outer['endConditions']=end
 q=copy.deepcopy(base);q['page']['document']['timelines']={slide:dict(format='musteroffice.timeline/0.2-draft',nodes=[a,b],tree=dict(roots=['seq'],containers=[outer,inner]))};samples=[]
 start_a=gate+F(1,4);natural_a=start_a+duration(a);start_b=gate+F(1,4)if parallel else natural_a+F(1,4);natural_b=start_b+duration(b);auto_end=max(natural_a,natural_b)
 for now in map(F,[0,F(1,3),F(1,2),F(3,4),1,F(5,4),F(3,2),2,F(9,4),F(5,2),3,4,8,20,F(1,3)]):
  stop=F(2) if kind==0 else start_a+F(7,4)if kind==1 else natural_a+F(3,4)if kind==2 else F(9,4)if now>=2 else F(8)
  if i%3==0:stop=min(stop,auto_end)
  values={};states={}
  for n,start,natural in [(a,start_a,natural_a),(b,start_b,natural_b)]:
   enabled=start<stop;end=min(natural,stop);states[n['id']]=dict(start=exact(start)if enabled else None,end=exact(end)if enabled else None)
   if not enabled or now<start:continue
   # A held ancestor retains active descendants clipped at its own endpoint.
   if now>=end and natural<=stop and n['fill']=='remove':continue
   iteration,p=progress(n,min(now,end)-start,now>=end,natural-start);states[n['id']].update(iteration=iteration,progress=exact(p));e=n['effect'];values[e['target']]=exact(e['from']+(e['to']-e['from'])*p)
  samples.append(dict(at=time(now),rotations=values,expectedNodes=states))
 author.append(dict(name=f'container-{i}'if i<24 else'nested-cutoff',page=q,samples=samples,events=events,through=time(30)))
put(root/'author-fixtures.json',dict(cases=author))
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
def holder(name,kind,ends,ids,automatic=False):
 h=X.fromstring(f'<p:{kind} xmlns:p="{P}"><p:cTn id="{name}" dur="indefinite" restart="never" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn></p:{kind}>');common=h.find('p:cTn',ns)
 if ends:
  el=X.SubElement(common,f'{{{P}}}endCondLst')
  for end in ends:native_condition(el,end,ids)
 if automatic:
  sync=X.SubElement(common,f'{{{P}}}endSync',evt='end',delay='0');X.SubElement(sync,f'{{{P}}}rtn',val='all')
 X.SubElement(common,f'{{{P}}}childTnLst');return h
for group in ['fixed-deadline','child-begin','child-end','click-stop','nested-stop','seq-stop']:
 tree=copy.deepcopy(original);lst=tree.find('p:timing/p:tnLst/p:par/p:cTn/p:childTnLst',ns);animations=list(lst);ids={str(i):a.find('p:cBhvr/p:cTn',ns).get('id')for i,a in enumerate(animations)};ids['inner']='21';lst.clear()
 ends=[at(3)]if group=='fixed-deadline'else[after('0','begin',3)]if group=='child-begin'else[after('0','end',F(1,2))]if group=='child-end'else[click(F(1,2)),at(8)]if group=='click-stop'else[after('inner','end',1)]if group=='nested-stop'else[after('0','end',1)]
 outer=holder('20','seq'if group=='seq-stop'else'par',ends,ids,group=='seq-stop');lst.append(outer);parent=outer
 if group=='nested-stop':
  parent=holder('21','par',[at(4)],ids,True);outer.find('p:cTn/p:childTnLst',ns).append(parent)
 nodes=[]
 for i,a in enumerate(animations):
  tf=transform(100000 if i==0 else 50000,False,25000,50000);n=leaf(str(i),f'sp.{900 if i==0 else 43}',2 if i==0 else 3,3000 if i==0 else 2000,tf,F(1,2));nodes.append(n)
  a.set('from','0');a.set('to','16200000');common=a.find('p:cBhvr/p:cTn',ns);common.set('dur',str(2000 if i==0 else 3000));common.set('repeatCount',str(n['repeatMilli']));common.set('fill','remove');common.set('spd',str(tf['speedMilliPercent']));common.set('autoRev','false');common.set('accel','25000');common.set('decel','50000');start=common.find('p:stCondLst',ns);start.clear();native_condition(start,n['start'],ids);parent.find('p:cTn/p:childTnLst',ns).append(a)
 p=dict(parts);p[part]=X.tostring(tree);source=pack(p,out/(group+'.pptx'));sources.append(source)
 for i,now in enumerate(map(F,[0,F(1,3),F(1,2),1,2,F(5,2),3,F(7,2),4,7,F(15,2),8,20,F(1,3)])):
  v=copy.deepcopy(q);v['page']['page']['expectedSourceSha256']=source['sha256'];binding=dict(session=group,revision=source['sha256'],generation='11');h=dict(binding=binding,through=time(30),events=[dict(generation='11',sequence=j+1,at=time(t),event=dict(kind='click',target=None))for j,t in enumerate([2,2,4,8])]);v['sample']=dict(binding=binding,at=time(now),history=h);values={}
  stop=F(3)if group=='fixed-deadline'else F(7,2)if group=='child-begin'else F(7)if group=='child-end'else F(5,2)if group=='click-stop'and now>=2 else F(8)if group=='click-stop'else F(4)if group=='nested-stop'else F(15,2)
  for index,n in enumerate(nodes):
   start=F(7)if group=='seq-stop'and index==1 else F(1,2);natural=start+duration(n);end=min(natural,stop)
   if now<start or start>=stop:continue
   if now>=end and natural<=stop:continue # natural remove differs from held ancestor clipping
   _,value=progress(n,min(now,end)-start,now>=end,natural-start);values[n['effect']['target']]=exact(16200000*value)
  name=f'{group}-{i}';case=dict(name=name,group=group,source=source,fonts=c['fonts'],request=put(out/(name+'.json'),v),expectedRotations=values)
  # Static control replaces only visible rotating objects. A naturally removed
  # behavior uses its base value, exactly as the author and source page compiler.
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
