"""Generate owned, editable, asymmetric group probes; no fonts or external media.

Each page has sixteen separately identifiable geometric cases and four global
registration marks. All source objects remain native paths/groups in the PPTX.
"""
import json,copy,pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1] if len(sys.argv)>1 else '.codex-work/group-compat'); root.mkdir(parents=True,exist_ok=True); E=9525
base={'document':json.load(open('fixtures/presentations/basic-shape.json')),'slide':'slide:1'}
base['document']['pageSize']={'width':'7620000','height':'5715000'}
base['document']['slides']['slide:1']['background']={'kind':'value','value':{'kind':'solid','color':{'kind':'srgb','rgba':{'red':255,'green':255,'blue':255,'alpha':255}}}}
shape=base['document']['objects']['shape:1'];shape['content']['text']=None
shape['appearance']={'fill':{'kind':'value','value':{'kind':'solid','color':{'kind':'srgb','rgba':{'red':180,'green':40,'blue':70,'alpha':255}}}},'stroke':{'kind':'value','value':{'kind':'none'}}}
defaults=json.load(open('fixtures/presentations/native-export/request.json'))['defaults']
def xy(x,y):return {'x':str(round(x*E)),'y':str(round(y*E))}
def size(w,h):return {'width':str(round(w*E)),'height':str(round(h*E))}
def obj(id,parent,x,y,w,h,angle=0,flip=0):
 o=copy.deepcopy(shape);o['id']=id;o['parent']=parent;o['transform']={'origin':xy(x,y),'size':size(w,h),'rotation':angle,'flipHorizontal':bool(flip&1),'flipVertical':bool(flip&2)};return o
angles=[2699999,2700000,2700001,8099999,8100000,8100001,13499999,13500000,13500001,18899999,18900000,18900001,-2700000,-8100000,24300000,-18900000]
specs={
'angles':[{'angle':a,'parentAngle':0,'parentFlip':0,'flip':0} for a in angles],
'flips':[{'angle':a,'parentAngle':1800000,'parentFlip':f,'flip':f//2} for a in [1800000,2700000,6000000,-3600000] for f in range(4)],
'nested':[{'angle':a,'parentAngle':1800000,'parentFlip':f,'flip':1,'nestedAngle':b} for a,b in [(1800000,1800000),(2700000,2700000),(-3600000,6000000),(6000000,-3600000)] for f in range(4)]}
(root/'empty.bin').write_bytes(b'')
manifest=[]
for name,cases in specs.items():
 q=copy.deepcopy(base);d=q['document'];d['title']='Owned group compatibility '+name;d['objects']={};roots=[]
 for i,c in enumerate(cases):
  col,row=i%4,i//4; cx,cy=100+col*200,75+row*150
  gid='group:'+str(i);parent={'kind':'slide','id':'slide:1'}
  g=obj(gid,parent,cx-70,cy-42,140,84,c['parentAngle'],c['parentFlip']);g['content']={'kind':'group','children':[],'viewport':size(70,70)};g['appearance']={};d['objects'][gid]=g;roots.append(gid)
  pid=gid
  if 'nestedAngle' in c:
   nid='nested:'+str(i);n=obj(nid,{'kind':'group','id':gid},12,14,48,36,c['nestedAngle'],(i+1)%4);n['content']={'kind':'group','children':[],'viewport':size(64,48)};n['appearance']={};d['objects'][nid]=n;g['content']['children']=[nid];pid=nid
  sid='shape:'+str(i);o=obj(sid,{'kind':'group','id':pid},8,17,54,30,c['angle'],c['flip'])
  o['content']['geometry']={'kind':'path','viewport':{'width':'100','height':'100'},'commands':[{'kind':'move','to':{'x':'0','y':'0'}},{'kind':'line','to':{'x':'100','y':'0'}},{'kind':'line','to':{'x':'70','y':'100'}},{'kind':'line','to':{'x':'0','y':'65'}},{'kind':'close'}]};d['objects'][sid]=o;d['objects'][pid]['content']['children']=[sid]
 for j,(x,y) in enumerate([(8,8),(788,8),(8,588),(788,588)]):
  id='marker:'+str(j);o=obj(id,{'kind':'slide','id':'slide:1'},x,y,4,4);o['content']['geometry']={'kind':'rectangle'};o['appearance']['fill']['value']['color']['rgba']={'red':15,'green':180,'blue':200,'alpha':255};d['objects'][id]=o;roots.append(id)
 d['slides']['slide:1']['objects']=roots
 stem=root/name;stem.with_suffix('.request.json').write_text(json.dumps(q));stem.with_suffix('.export.json').write_text(json.dumps({'document':d,'defaults':defaults,'resourceBindings':[]}));out=root/((sys.argv[2] if len(sys.argv)>2 else '')+name+'.pptx');out.unlink(missing_ok=True)
 subprocess.run(['target/release/mo-cli','pptx-export',str(stem.with_suffix('.export.json')),str(root/'empty.bin'),str(out)],check=True,capture_output=True,timeout=30)
 manifest.append({'name':name,'cases':cases,'request':str(stem.with_suffix('.request.json')),'pptx':str(out)})
(root/'probes.json').write_text(json.dumps(manifest,indent=2)+'\n')
print('Generated 3 editable decks, 48 asymmetric probes with 4 registration markers per slide')
