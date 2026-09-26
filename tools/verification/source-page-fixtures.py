"""Owned native page fixtures. Expected ink colors/layers come from input XML."""
import copy, hashlib, json, zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A, P
ROOT=Path('.codex-work/source-page'); ROOT.mkdir(exist_ok=True)
NS={'a':A,'p':P}; SLIDE='ppt/slides/slide1.xml'; LAYOUT='ppt/slideLayouts/slideLayout2.xml'; MASTER='ppt/slideMasters/slideMaster2.xml'
base=next(c for c in json.loads(Path('.codex-work/source-placement/manifest.json').read_text())['cases'] if c['name']=='root-absent')
sha=lambda b:hashlib.sha256(b).hexdigest()
assert sha(Path(base['path']).read_bytes())==base['sha256']
with zipfile.ZipFile(base['path']) as z: BASE={n:z.read(n) for n in z.namelist()}
def parse(s): return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}">{s}</root>')
def shape(id,color,x=1000000,y=1000000,w=2000000,h=1000000,preset='rect',extra='',attrs='',nv='',ph='',text=''):
    return parse(f'<p:sp {attrs}><p:nvSpPr><p:cNvPr id="{id}" name="Owned source page probe" {nv}/><p:cNvSpPr/><p:nvPr>{ph}</p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="{preset}"/><a:solidFill>{color}</a:solidFill><a:ln><a:noFill/></a:ln>{extra}</p:spPr>{text}</p:sp>')[0]
rgb=lambda c:f'<a:srgbClr val="{c}"/>'
def emit(name,objects,flags=(None,None),mutate=None,issue=None,oracle=None):
    parts=dict(BASE)
    for i,part in enumerate([MASTER,LAYOUT,SLIDE]):
        root=E.fromstring(parts[part]);tree=root.find('p:cSld/p:spTree',NS)
        for c in list(tree):
            if E.QName(c).localname not in ['nvGrpSpPr','grpSpPr']:tree.remove(c)
        props=tree.find('p:grpSpPr',NS)
        for c in list(props):props.remove(c)
        for c in objects[i]:tree.append(copy.deepcopy(c))
        if i>0 and flags[2-i] is not None:root.set('showMasterSp',str(int(flags[2-i])))
        if mutate:mutate(root,part)
        parts[part]=E.tostring(root,encoding='UTF-8',xml_declaration=True)
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w') as z:
        for n,b in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    CASES.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'issue':issue,'oracle':oracle})
CASES=[]
layers=[[shape(22,rgb('0000FF'),x=1000000,y=1000000,w=5000000,h=3000000)],[shape(22,rgb('00FF00'),x=2000000,y=2000000,w=5000000,h=3000000)],[shape(22,rgb('FF0000'),x=3000000,y=3000000,w=5000000,h=3000000)]]
for s,l in [(None,None),(False,None),(None,False),(False,False),(True,True)]:
    emit(f'layers-{s}-{l}',layers,(s,l),oracle={'rectangles':([{'color':[0,0,255],'box':[1000000,1000000,6000000,4000000]}] if s is not False and l is not False else [])+([{'color':[0,255,0],'box':[2000000,2000000,7000000,5000000]}] if s is not False else [])+[{'color':[255,0,0],'box':[3000000,3000000,8000000,6000000]}]})
for preset in ['ellipse','roundRect','hexagon','triangle','diamond','heart','star5','arc','wedgeRoundRectCallout']:
    emit('preset-'+preset,[[],[],[shape(22,rgb('DB234B'),preset=preset)]])
ph='<p:ph type="body" idx="1"/>'
emit('template-placeholders',[[shape(22,rgb('0000FF'),ph=ph)],[shape(23,rgb('00FF00'),ph=ph)],[]],oracle={'rectangles':[]})
def map_override(root,part):
    if part==SLIDE:
        c=root.find('p:clrMapOvr',NS);c.clear();c.append(parse('<a:overrideClrMapping bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent2" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>')[0])
emit('master-color-context',[[shape(22,'<a:schemeClr val="accent1"/>')],[],[]],mutate=map_override,oracle={'rectangles':[{'color':[27,114,232],'box':[1000000,1000000,3000000,2000000]}]})
def slide_background(root,part):
    if part==SLIDE:root.find('p:cSld',NS).insert(0,parse('<p:bg><p:bgPr><a:solidFill><a:srgbClr val="123456"/></a:solidFill></p:bgPr></p:bg>')[0])
emit('master-background-context',[[shape(22,rgb('FF0000'),attrs='useBgFill="1"')],[],[]],mutate=slide_background,oracle={'background':[18,52,86],'rectangles':[{'color':[18,52,86],'box':[1000000,1000000,3000000,2000000]}]})
bad=shape(22,rgb('FF0000'),extra='<a:scene3d/>')
g=parse('<p:grpSp><p:nvGrpSpPr><p:cNvPr id="2" name="Hidden group" hidden="1"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:grpSp>')[0];g.append(bad)
emit('hidden-group',[[],[],[g,shape(23,rgb('00FF00'))]],oracle={'rectangles':[{'color':[0,255,0],'box':[1000000,1000000,3000000,2000000]}]})
emit('visible-unknown',[[],[],[bad]],issue='visual')
emit('visible-text',[[],[],[shape(22,rgb('FF0000'),text='<p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Visible text</a:t></a:r></a:p></p:txBody>')]],issue='text')
emit('visible-effects',[[],[],[shape(22,rgb('FF0000'),extra='<a:effectLst><a:glow rad="50000"><a:srgbClr val="000000"/></a:glow></a:effectLst>')]],issue='effects')
emit('empty-effects',[[],[],[shape(22,rgb('FF0000'),extra='<a:effectLst/>')]],oracle={'rectangles':[{'color':[255,0,0],'box':[1000000,1000000,3000000,2000000]}]})
for kind,content in [('pic','<p:nvPicPr><p:cNvPr id="22" name="Picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr/>'),('graphicFrame','<p:nvGraphicFramePr><p:cNvPr id="22" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="1" cy="1"/></p:xfrm><a:graphic><a:graphicData uri="urn:owned:unknown"/></a:graphic>')]:
    emit('visible-'+kind,[[],[],[parse(f'<p:{kind}>{content}</p:{kind}>')[0]]],issue='object')
stroke=shape(22,rgb('FF0000'));ln=stroke.find('p:spPr/a:ln',NS);ln.clear();ln.set('w','100000');ln.append(parse('<a:solidFill><a:srgbClr val="000000"/></a:solidFill>')[0]);ln.append(parse('<a:round/>')[0]);emit('native-round-stroke',[[],[],[stroke]])
for name,child,issue in [('dashed-line','<a:prstDash val="dash"/>','line'),('miter-line','<a:miter lim="400000"/>','line')]:
    s=copy.deepcopy(stroke);ln=s.find('p:spPr/a:ln',NS)
    if name=='miter-line':ln.remove(ln.find('a:round',NS))
    ln.insert(1,parse(child)[0]);emit(name,[[],[],[s]],issue=issue)
group=parse('<p:grpSp><p:nvGrpSpPr><p:cNvPr id="2" name="Group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm rot="2700001"><a:off x="2000000" y="1000000"/><a:ext cx="5000000" cy="2000000"/><a:chOff x="1000000" y="1000000"/><a:chExt cx="2000000" cy="1000000"/></a:xfrm></p:grpSpPr></p:grpSp>')[0];group.append(shape(22,rgb('DB234B'),preset='ellipse'));emit('group-ellipse',[[],[],[group]])
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':CASES},indent=2)+'\n');print(json.dumps({'cases':len(CASES)}))
