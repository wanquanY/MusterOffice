"""Owned animation corpus. Expected frames are checked separately with Fraction."""
import copy,json,random
from pathlib import Path
root=Path('.codex-work/timeline');root.mkdir(parents=True,exist_ok=True)
base=json.loads(Path('fixtures/presentations/native-export/request.json').read_text())
slide=base['document']['slideOrder'][0];targets=base['document']['slides'][slide]['objects']
def time(n,d=1):return dict(ticks=str(n),timescale=d)
rng=random.Random(20260926);cases=[]
for case in range(32):
 nodes=[]
 for i in range(12):
  start=dict(kind='at',offset=time(rng.randrange(0,6),4))
  choice=rng.randrange(3)
  if i and choice==1:start=dict(kind='after',node=nodes[rng.randrange(i)]['id'],event=rng.choice(['begin','end']),delay=time(rng.randrange(0,3),10))
  if choice==2:start=dict(kind='click',target=rng.choice([None,*targets[:3]]),delay=time(rng.randrange(0,3),8))
  nodes.append(dict(id=f'node:{i}',start=start,duration=time(rng.randrange(1,100),rng.choice([3,7,30,1000,30000])),repeatMilli=rng.choice([1,500,999,1000,2000,2500,3001]),fill=rng.choice(['remove','freeze']),effect=dict(kind='rotation',target=rng.choice(targets[:3]),**{'from':rng.choice([-2147483648,-43200000,0,1,2147483647]),'to':rng.choice([-2147483648,-21600000,0,64800000,2147483647])})))
 rng.shuffle(nodes)
 document=copy.deepcopy(base['document']);document['timelines']={slide:dict(format='musteroffice.timeline/0.1-draft',nodes=nodes)}
 events=[dict(generation='7',sequence=i+1,at=time(i,2),event=dict(kind='click',target=rng.choice([None,*targets[:3]]))) for i in range(12)]
 # Exact repeated transport delivery of the current last event.
 events.insert(4,copy.deepcopy(events[3]))
 cases.append(dict(name=f'graph-{case}',document=document,slide=slide,events=events,samples=[time(0),time(1001,30000),time(1,2),time(3),time(8),time(80)]))
# Exportable graph with both native dependency events, clicks, fill modes and fractional repeat.
export=copy.deepcopy(base);nodes=[]
starts=[dict(kind='at',offset=time(0)),dict(kind='after',node='export:0',event='end',delay=time(125,1000)),dict(kind='after',node='export:1',event='begin',delay=time(1,2)),dict(kind='click',target=targets[0],delay=time(0)),dict(kind='click',target=None,delay=time(1,4))]
for i,start in enumerate(starts):nodes.append(dict(id=f'export:{i}',start=start,duration=time(2),repeatMilli=2500 if i==1 else 1000,fill='remove' if i==2 else 'freeze',effect=dict(kind='rotation',target=targets[i%3],**{'from':-43200000,'to':64800000})))
export['document']['timelines']={slide:dict(format='musteroffice.timeline/0.1-draft',nodes=nodes)}
(root/'export-request.json').write_text(json.dumps(export,indent=2)+'\n')
(root/'corpus.json').write_text(json.dumps(dict(cases=cases),indent=2)+'\n')
print(json.dumps(dict(graphs=len(cases),frames=sum(len(c['samples'])for c in cases))))
