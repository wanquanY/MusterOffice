"""Independent exact-time + high-precision transform + cardinal pixel reference."""
import hashlib,json
from pathlib import Path
from fractions import Fraction as F
import mpmath as M
M.mp.dps=100
root=Path('.codex-work/playback-render');report=json.loads((root/'product.json').read_text())
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):assert entry(r['path'])==r;return Path(r['path']).read_bytes()
def time(t):return F(int(t['ticks']),t['timescale'])
def mp(v):v=F(v);return M.mpf(v.numerator)/v.denominator
def exact(v):return F(int(v['numerator']),int(v['denominator']))
def properties(q):
 p=q['playback'];nodes=p['snapshot']['document']['timelines'][p['slide']]['nodes'];now=time(p['at']);byid={n['id']:n for n in nodes};starts={};ends={};values={}
 def start(key):
  if key in starts:return starts[key]
  n=byid[key];s=n['start']
  if s['kind']=='at':a=time(s['offset'])
  elif s['kind']=='after':
   parent=start(s['node']);a=None if parent is None else (parent if s['event']=='begin' else ends[s['node']])+time(s['delay'])
  else:
   events=[time(e['at'])for e in p['history']['events']if time(e['at'])<=now and e['event']['target']==s['target']];a=min(events)+time(s['delay'])if events else None
  starts[key]=a;ends[key]=None if a is None else a+time(n['duration'])*F(n['repeatMilli'],1000);return a
 for i,n in enumerate(nodes):
  a=start(n['id']);end=ends[n['id']]
  if a is None or now<a or(now>=end and n['fill']=='remove'):continue
  count=min(now-a,end-a)/time(n['duration']);progress=count%1
  if now>=end and progress==0:progress=F(1)
  e=n['effect'];v=e['from']+(e['to']-e['from'])*progress;priority=(a,i)
  if e['target']not in values or priority>values[e['target']][0]:values[e['target']]=(priority,v)
 return {key:v for key,(_,v)in values.items()}
scalars=objects=frames=0;max_ratio=M.mpf(0)
for case in report['cases']:
 if case['calls']!=1:continue
 q=json.loads(load(case['request']));compiled=json.loads(load(case['compiled']))['frame'];doc=q['playback']['snapshot']['document'];rotations=properties(q);assert {k:exact(v)for k,v in compiled['frame']['state']['rotations'].items()}==rotations
 found={o['object']:o for s in compiled['placements']['surfaces']for o in s['objects']};reference={};turn=21600000
 def walk(key,parent):
  o=doc['objects'][key];t=o['transform'];group=o['content']['kind']=='group';w,h=[int(t['size'][a])for a in ['width','height']];sw,sh=([int(o['content']['viewport'][a])for a in ['width','height']]if group else[w,h]);x,y=[int(t['origin'][a])for a in ['x','y']]
  angle=rotations.get(key,F(t['rotation']))%turn;swap=2700000<=angle<8100000 or 13500000<=angle<18900000
  scales=[parent['scale'][1-i if swap else i]*(F(n,d)if d else 1)for i,(n,d)in enumerate([(w,sw),(h,sh)])]
  flips=[parent['flips'][0]^t['flipHorizontal'],parent['flips'][1]^t['flipVertical']];total=(parent['angle']+(-angle if parent['flips'][0]^parent['flips'][1]else angle))%turn
  theta=mp(total)*2*M.pi/turn;c,s=M.cos(theta),M.sin(theta);sx,sy=[mp(v)*(-1 if flips[i]else 1)for i,v in enumerate(scales)]
  linear=M.matrix([[c*sx,-s*sy],[s*sx,c*sy]]);center=parent['matrix']*M.matrix([mp(F(x)+F(w,2)),mp(F(y)+F(h,2))])+parent['offset'];anchor=M.matrix([mp(F(sw,2)),mp(F(sh,2))]);reference[key]=(linear,center,anchor)
  if group:
   state=dict(scale=scales,flips=flips,angle=total,matrix=linear,offset=center-linear*anchor)
   for child in o['content']['children']:walk(child,state)
 state=dict(scale=[F(1),F(1)],flips=[False,False],angle=F(0),matrix=M.eye(2),offset=M.matrix([0,0]))
 for key in doc['slides'][q['playback']['slide']]['objects']:walk(key,state)
 assert set(reference)==set(found)
 for key,(linear,center,anchor)in reference.items():
  objects+=1;o=found[key]
  # Affine wire order is row-major; Q32 values and separate outward bounds.
  for actual,bound,expected in zip(o['affine']['linear'],o['uncertainty']['linear'],[linear[0,0],linear[0,1],linear[1,0],linear[1,1]],strict=True):
   error=abs(M.mpf(actual)/2**32-expected);budget=M.mpf(bound)/2**32;assert error<=budget+M.mpf('1e-75'),(case['name'],key,actual,bound,str(error));scalars+=1
   if budget:max_ratio=max(max_ratio,error/budget)
  for i,axis in enumerate(['x','y']):
   actual=M.mpf(o['affine']['translation'][axis])/2**32;budget=M.mpf(o['uncertainty']['translation'][axis])/2**32;assert abs(actual-center[i])<=budget+M.mpf('1e-75');assert M.mpf(o['anchor'][axis])/2**32==anchor[i];scalars+=3
  assert compiled['page']['info']['documentSha256']==q['playback']['snapshot']['semanticDigest']
 frames+=1
rigid={c['name']:c for c in report['cases']if c['name'].startswith('rigid-')};base=load(rigid['rigid-0']['pixels']);w,h=320,240;pixel_checks=0
assert len(set(base[i:i+4]for i in range(0,len(base),4)))>1
for quarter in range(1,5):
 actual=load(rigid[f'rigid-{quarter}']['pixels']);expected=bytearray()
 for y in range(h):
  for x in range(w):
   if quarter==1:ox,oy=y+40,279-x
   elif quarter==2:ox,oy=319-x,239-y
   elif quarter==3:ox,oy=279-y,x-40
   else:ox,oy=x,y
   expected.extend(base[(oy*w+ox)*4:(oy*w+ox)*4+4]if 0<=ox<w and 0<=oy<h else b'\xff\xff\xff\xff');pixel_checks+=1
 assert bytes(expected)==actual,rigid[f'rigid-{quarter}']['name']
# Backward seeking and recovery have identical geometry/pixels for identical state.
for a,b in [('grouped-1','grouped-9'),('grouped-1','recovery'),('interactive-1','interactive-5')]:
 aa=next(c for c in report['cases']if c['name']==a);bb=next(c for c in report['cases']if c['name']==b);assert load(aa['pixels'])==load(bb['pixels']);assert load(aa['compiled'])==load(bb['compiled'])
out=dict(format='musteroffice.playback-render-reference/1',product=entry(root/'product.json'),frames=frames,objects=objects,scalarChecks=scalars,precisionDigits=M.mp.dps,maximumBoundFraction=str(max_ratio),rigidPixels=pixel_checks,rigidCases=4,seekAndRecoveryEqual=True)
(root/'reference.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out))
