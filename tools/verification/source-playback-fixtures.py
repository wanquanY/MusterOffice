"""Add original native timing to frozen owned resource-page fixtures; independent XML controls."""
import copy,hashlib,io,json,zipfile
from pathlib import Path
from fractions import Fraction
from lxml import etree as X
root=Path('.codex-work/source-playback');out=root/'fixtures';out.mkdir(parents=True,exist_ok=True)
P='http://schemas.openxmlformats.org/presentationml/2006/main';A='http://schemas.openxmlformats.org/drawingml/2006/main';ns=dict(p=P,a=A)
prior_path=Path('.codex-work/playback-session/replay.json');prior=json.loads(prior_path.read_text())
def entry(p):
 p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(r):assert entry(r['path'])==r;return Path(r['path']).read_bytes()
def pack(parts,path):
 with zipfile.ZipFile(path,'w')as z:
  for n,b in parts.items():
   info=zipfile.ZipInfo(n,(2026,9,26,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
 return entry(path)
def time(n,d=1):return dict(ticks=str(n),timescale=d)
def sub(parent,name,**a):return X.SubElement(parent,'{'+P+'}'+name,**{k:str(v)for k,v in a.items()})
def timing(root,nodes):
 for old in root.findall('p:timing',ns):root.remove(old)
 t=sub(root,'timing');li=sub(t,'tnLst');par=sub(li,'par');ct=sub(par,'cTn',id=1,dur='indefinite',restart='never',nodeType='tmRoot');children=sub(ct,'childTnLst')
 for i,node in enumerate(nodes):
  anim=sub(children,'animRot',**{'from':node['from'],'to':node['to']});b=sub(anim,'cBhvr',additive='repl',accumulate='none',xfrmType='pt');ct=sub(b,'cTn',id=i+2,dur=node['duration'],repeatCount=1000,restart='never',fill=node['fill']);l=sub(ct,'stCondLst');cond=sub(l,'cond',delay=0)
  if node.get('click'):cond.set('evt','onClick');sub(sub(cond,'tgtEl'),'sldTgt')
  sub(sub(b,'tgtEl'),'spTgt',spid=node['id']);sub(sub(b,'attrNameLst'),'attrName').text='r'
 return t
records=[];checked=0;schema=X.XMLSchema(X.parse('.codex-work/ecma376/xsd/pml.xsd'))
selections=[('image-text','previous/image-text'),('group-image','previous/group-direct-image'),('masters','previous/master-layout-slide-images'),('circle','new/equal-width-offset'),('text','previous/foreground-text')]
for name,suffix in selections:
 case=next(c for c in prior['cases']if c['name'].endswith(suffix));base=load(case['source']);q=json.loads(load(case['request']));fonts=case['fonts'];load(fonts)
 with zipfile.ZipFile(io.BytesIO(base))as z:parts={n:z.read(n)for n in z.namelist()}
 slide=q['page']['slide'].lstrip('/');tree=X.fromstring(parts[slide]);objects=tree.findall('.//p:sp',ns)+tree.findall('.//p:pic',ns)+tree.findall('.//p:grpSp',ns)
 def native(o):return int(o.find('./*/p:cNvPr',ns).get('id'))
 ids=[native(o)for o in objects];assert ids
 nodes=[dict(id=n,**{'from':0,'to':21600000 if i%2==0 else -21600000},duration=2000,fill='remove'if i%2 else 'freeze')for i,n in enumerate(ids)]
 if name=='circle':nodes[0]['to']=2700001;nodes[0]['from']=2699999
 if name=='group-image':
  groups={native(o)for o in tree.findall('.//p:grpSp',ns)};assert groups
  for node in nodes:
   if node['id']in groups:node['to']+=1
 timing(tree,nodes);parts[slide]=X.tostring(tree);source=pack(parts,out/(name+'.pptx'));q['page']['expectedSourceSha256']=source['sha256'];binding=dict(session=name,revision=source['sha256'],generation='11')
 for i,(n,d)in enumerate([(0,1),(1,3),(1,2),(1,1),(3,2),(2,1),(1,3)]):
  at=Fraction(n,d);request=dict(page=q,sample=dict(binding=binding,at=time(n,d),history=None));p=out/f'{name}-{i}.json';p.write_text(json.dumps(request));expected={};
  for node in nodes:
   duration=Fraction(node['duration'],1000)
   if at>=duration:
    if node['fill']=='remove':continue
    value=Fraction(node['to'])
   else:value=Fraction(node['from'])+Fraction(node['to']-node['from'])*at/duration
   expected['sp.'+str(node['id'])]=dict(numerator=str(value.numerator),denominator=str(value.denominator))
  record=dict(name=f'{name}-{i}',request=entry(p),source=source,fonts=fonts,expectedRotations=expected,expectedBindings={f'sp.{v}':dict(part=q['page']['slide'],nativeId=v)for v in ids},baseSource=case['source'])
  # An independently rewritten native transform is an exact pixel control only
  # when every sampled angle is integral. Remove timing; leave other parts intact.
  if all(v['denominator']=='1'for v in expected.values()):
   control=copy.deepcopy(tree)
   control.remove(control.find('p:timing',ns))
   for o in control.findall('.//p:sp',ns)+control.findall('.//p:pic',ns)+control.findall('.//p:grpSp',ns):
    key='sp.'+str(native(o))
    if key in expected:
     xf=o.find('p:grpSpPr/a:xfrm',ns)if X.QName(o).localname=='grpSp'else o.find('p:spPr/a:xfrm',ns)
     assert xf is not None;xf.set('rot',expected[key]['numerator'])
   cp=dict(parts);cp[slide]=X.tostring(control);cs=pack(cp,out/f'{name}-{i}.static.pptx');cq=copy.deepcopy(q);cq['page']['expectedSourceSha256']=cs['sha256'];path=out/f'{name}-{i}.static.json';path.write_text(json.dumps(cq));record['staticControl']=dict(source=cs,request=entry(path))
  records.append(record)
 # Native parser failures must occur before fonts/shaping/image/raster calls.
 if name=='image-text':
  for kind in ['unknown','click','missing-target']:
   alt=copy.deepcopy(tree);an=alt.find('.//p:animRot',ns)
   if kind=='unknown':an.set('by','60000')
   elif kind=='click':
    cond=an.find('.//p:cond',ns);cond.set('evt','onClick');sub(sub(cond,'tgtEl'),'sldTgt')
   else:an.find('.//p:cBhvr/p:tgtEl/p:spTgt',ns).set('spid','999999')
   cp=dict(parts);cp[slide]=X.tostring(alt);s=pack(cp,out/f'{kind}.pptx');rq=copy.deepcopy(q);rq['page']['expectedSourceSha256']=s['sha256'];b=dict(binding,revision=s['sha256']);query=dict(page=rq,sample=dict(binding=b,at=time(1,3),history=None));p=out/(kind+'.json');p.write_text(json.dumps(query));records.append(dict(name=kind,source=s,request=entry(p),fonts=fonts,error={'unknown':'MAPPING_NOT_IMPLEMENTED','click':'EVENT_HISTORY_REQUIRED','missing-target':'INPUT_INVALID'}[kind],noComponentCalls=True))
   if kind=='click':
    for suffix,at in [('after',Fraction(1,3)),('before',Fraction(0))]:
     query=copy.deepcopy(query);query['sample']['at']=time(at.numerator,at.denominator);query['sample']['history']=dict(binding=b,through=time(10),events=[dict(generation=b['generation'],sequence=1,at=time(1,4),event=dict(kind='click',target=None))])
     expected={}
     for j,node in enumerate(nodes):
      start=Fraction(1,4)if j==0 else Fraction(0)
      if at<start:continue
      v=Fraction(node['from'])+Fraction(node['to']-node['from'])*(at-start)/Fraction(node['duration'],1000)
      expected['sp.'+str(node['id'])]=dict(numerator=str(v.numerator),denominator=str(v.denominator))
     path=out/('click-'+suffix+'.json');path.write_text(json.dumps(query));records.append(dict(name='click-'+suffix,source=s,request=entry(path),fonts=fonts,expectedRotations=expected,expectedBindings={f'sp.{v}':dict(part=q['page']['slide'],nativeId=v)for v in ids},baseSource=case['source']))
# Source mismatch and revision mismatch independently fenced; precision refuses pixels.
positive=next(r for r in records if r['name']=='image-text-1')
for name,code in [('source-conflict','SOURCE_CONFLICT'),('revision-conflict','REVISION_CONFLICT'),('viewport-invalid','INPUT_INVALID'),('precision','PRECISION_EXCEEDED')]:
 r=copy.deepcopy(positive);q=json.loads(load(positive['request']));
 if name=='source-conflict':q['page']['page']['expectedSourceSha256']='0'*64
 elif name=='revision-conflict':q['sample']['binding']['revision']='0'*64
 else:q['page']['page']['viewport']['coordinateTolerance']='256'if name=='precision'else '1'
 p=out/(name+'.json');p.write_text(json.dumps(q));r.update(name=name,request=entry(p),error=code,noComponentCalls=name!='precision');r.pop('expectedRotations',None);r.pop('expectedBindings',None);r.pop('staticControl',None);records.append(r)
# Validate every referenced native source/control, including click and rejected
# semantic timing declarations. Structural XSD and runtime meaning are separate.
checked=0;sources={c['source']['path']for c in records}|{c['staticControl']['source']['path']for c in records if 'staticControl'in c}
for path in sorted(sources):
 with zipfile.ZipFile(path)as z:
  for n in z.namelist():
   if not n.endswith('.xml'):continue
   node=X.fromstring(z.read(n));tag=X.QName(node)
   if tag.namespace==P and tag.localname in ['sld','sldLayout','sldMaster','presentation']:schema.assertValid(node);checked+=1
(root/'fixtures.json').write_text(json.dumps(dict(format='musteroffice.source-playback-fixtures/1',previous=entry(prior_path),cases=records,positiveCases=sum('error'not in c for c in records),officialXsdParts=checked),indent=2)+'\n')
print(json.dumps(dict(cases=len(records),officialXsdParts=checked)))
