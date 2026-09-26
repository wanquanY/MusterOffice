"""Owned native PPTX transform fixtures; expected frames are author input, not output."""
import copy, hashlib, json, zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A, P
ROOT=Path('.codex-work/source-placement');ROOT.mkdir(exist_ok=True)
NS={'a':A,'p':P};SLIDE='ppt/slides/slide1.xml';LAYOUT='ppt/slideLayouts/slideLayout2.xml';MASTER='ppt/slideMasters/slideMaster2.xml'
sha=lambda b:hashlib.sha256(b).hexdigest()
base=next(c for c in json.loads(Path('.codex-work/preset-expansion/manifest.json').read_text())['cases'] if c['name']=='rect-square')
assert sha(Path(base['path']).read_bytes())==base['sha256']
with zipfile.ZipFile(base['path']) as z:BASE={n:z.read(n) for n in z.namelist()}
def parse(s):return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}">{s}</root>')
def xf(f,group=False,extra=''):
    if f is None:return ''
    attrs=' '.join(f'{k}="{f[k]}"' for k in ['rot','flipH','flipV'] if k in f)
    s=f'<a:xfrm {attrs}>'
    for name,keys in [('off',['x','y']),('ext',['cx','cy']),('chOff',['x','y']),('chExt',['cx','cy'])]:
        if name in f:s+=f'<a:{name} '+ ' '.join(f'{k}="{v}"' for k,v in zip(keys,f[name],strict=True))+'/>'
    return s+extra+'</a:xfrm>'
def shape(f,ph=False,transform=None,id=99):
    t=xf(f) if transform is None else transform
    return parse(f'<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Owned source placement probe"/><p:cNvSpPr/><p:nvPr>'+('<p:ph type="body" idx="1"/>' if ph else '')+f'</p:nvPr></p:nvSpPr><p:spPr>{t}<a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="DB234B"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>')[0]
def tree(parts,file,objects,root=''):
    d=E.fromstring(parts[file]);t=d.find('p:cSld/p:spTree',NS)
    for c in list(t):
        if E.QName(c).localname not in ['nvGrpSpPr','grpSpPr']:t.remove(c)
    prop=t.find('p:grpSpPr',NS)
    for c in list(prop):prop.remove(c)
    for c in parse(root):prop.append(c)
    for c in objects:t.append(c)
    parts[file]=E.tostring(d,encoding='UTF-8',xml_declaration=True)
CASES=[]
LEAF={'off':[1000001,1000003],'ext':[1000001,500003]}
GROUP={'off':[2000001,1000003],'ext':[4000003,2000001],'chOff':[1000001,500003],'chExt':[2000001,1000003]}
def emit(name,frames,root=None,extra=None,error=None,unresolved=None,ph=None,raw=None,render=False):
    parts=dict(BASE);leaf=copy.deepcopy(frames[-1]);node=shape(leaf,ph is not None,raw);ids=[99];expected=[{'id':99,'frames':frames}]
    for i,f in reversed(list(enumerate(frames[:-1]))):
        id=10+i;g=parse(f'<p:grpSp><p:nvGrpSpPr><p:cNvPr id="{id}" name="Source group {i}"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{xf(f,True,extra if i==0 and extra else "")}</p:grpSpPr></p:grpSp>')[0];g.append(node);node=g;ids.append(id);expected.append({'id':id,'frames':frames[:i+1],'group':True})
    tree(parts,SLIDE,[node],xf(root,True))
    if ph is not None:
        tree(parts,MASTER,[shape(ph['master'],True,id=98)]);tree(parts,LAYOUT,[shape(ph['layout'],True,id=97)])
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    record={'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'ids':ids,'expected':expected,'error':error,'unresolved':unresolved,'render':render,'root':root,'placeholder':ph}
    CASES.append(record);return record
emit('root-absent',[LEAF],render=True)
emit('root-transform',[LEAF],root={**GROUP,'rot':1234567,'flipH':1},render=True)
for ax in [(0,0),(0,1000003),(2000001,0),(2000001,1000003)]:
    for rot in [0,5400000,-5400000,2700001]:emit(f'group-axis-{ax[0]}-{ax[1]}-{rot}',[{**GROUP,'chExt':ax,'rot':rot},LEAF],render=rot==0)
for angle in [-2147483648,-21600001,-1,0,1,2699999,2700000,2700001,5399999,5400000,8099999,8100000,8100001,10800000,13499999,13500000,13500001,16199999,16200000,18899999,18900000,18900001,21599999,21600000,21600001,2147483647]:
    for fh,fv in [(0,0),(1,0),(0,1),(1,1)]:
        emit(f'nested-{angle}-{fh}-{fv}',[{**GROUP,'rot':1876543,'flipH':fh,'flipV':fv},{**GROUP,'off':[1000003,-300001],'chOff':[-700003,1000001],'rot':angle},{**LEAF,'rot':-765432,'flipH':1}],render=angle==2700001)
for label,f in [('empty',{}),('no-child-extent',{k:v for k,v in GROUP.items() if k!='chExt'}),('no-child-origin',{k:v for k,v in GROUP.items() if k!='chOff'}),('zero-target',{**GROUP,'ext':[0,0]}),('one-target-zero',{**GROUP,'ext':[0,2000001]})]:emit('group-'+label,[f,LEAF])
for label,local in [('absent',None),('empty',{}),('rotation',{'rot':0}),('size',{'ext':LEAF['ext']}),('origin',{'off':LEAF['off']}),('full',LEAF)]:
    master={**LEAF,'rot':5400000,'flipH':1};ph={'master':master,'layout':None}
    r=emit('placeholder-'+label,[local],ph=ph,render=True)
    resolved={**LEAF,**({} if local is None else local)}
    if local is None:resolved.update(rot=5400000,flipH=1)
    r['expected'][0]['frames']=[resolved]
for label,opaque in [('foreign','<x:transform xmlns:x="urn:future"><a:off x="123" y="456"/></x:transform>'),('future','<a:future/>')]:emit('opaque-group-'+label,[GROUP,LEAF],extra=opaque,unresolved='retainedTransform')
for label,raw,kind in [('attr','<a:xfrm future="1">'+xf(LEAF).split('>',1)[1],'retainedTransform'),('missing-origin','<a:xfrm><a:ext cx="10" cy="20"/></a:xfrm>','missingOrigin'),('missing-size','<a:xfrm><a:off x="10" y="20"/></a:xfrm>','missingSize'),('range','<a:xfrm><a:off x="30000000000000" y="0"/><a:ext cx="10" cy="20"/></a:xfrm>','invalidCoordinate')]:emit('unresolved-'+label,[LEAF],raw=raw,unresolved=kind)
emit('large-coordinates',[{'off':[3000000000,0],'ext':[10,20]}])
large={**GROUP,'ext':[20000000000000,20000000000000],'chExt':[1,1]}
r=emit('numeric-depth',[large,large,LEAF],unresolved='numericRange');r['unresolvedIds']=[99]
for label,raw in [('duplicate','<a:xfrm/><a:xfrm/>'),('reverse','<a:xfrm><a:ext cx="1" cy="2"/><a:off x="0" y="0"/></a:xfrm>'),('negative','<a:xfrm><a:ext cx="-1" cy="2"/></a:xfrm>'),('child-in-leaf','<a:xfrm><a:chExt cx="1" cy="2"/></a:xfrm>'),('text','<a:xfrm>unexpected</a:xfrm>')]:emit('reject-'+label,[LEAF],raw=raw,error='INPUT_INVALID')
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':CASES},indent=2)+'\n');print(json.dumps({'cases':len(CASES),'renderCandidates':sum(c['render'] for c in CASES)}))
