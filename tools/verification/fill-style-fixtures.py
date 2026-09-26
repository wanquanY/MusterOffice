"""Owned multi-level OOXML inputs and explicit fill profile expectations.

The expectations are authored from the native declarations; no kernel output
is used to produce them. Color expressions are intentionally not sampled.
"""
import copy
import hashlib
import json
import zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A, P
from fill_reference import R

ROOT=Path('.codex-work/fill-resolution'); ROOT.mkdir(exist_ok=True,parents=True)
SLIDE='ppt/slides/slide1.xml'; LAYOUT='ppt/slideLayouts/slideLayout2.xml'; MASTER='ppt/slideMasters/slideMaster2.xml'; THEME='ppt/theme/theme2.xml'
NS={'a':A,'p':P}; cases=[]
base=next(c for c in json.loads(Path('.codex-work/pptx-color-map-fixtures/manifest.json').read_text())['cases'] if c['name']=='authored')
assert hashlib.sha256(Path(base['path']).read_bytes()).hexdigest()==base['sha256']
with zipfile.ZipFile(base['path']) as z: parts={n:z.read(n) for n in z.namelist()}
def fragment(xml): return E.fromstring(f'<r xmlns:a="{A}" xmlns:p="{P}" xmlns:r="{R}">{xml}</r>')
def solid(hex): return f'<a:solidFill><a:srgbClr val="{hex}"/></a:solidFill>'
def emit(name,layers=('', '', ''),theme='<a:noFill/>',reference=None,expected=None,hierarchy=False,mutate=None,conforming=True):
    owned=dict(parts); trees={n:E.fromstring(b) for n,b in parts.items() if n in [SLIDE,LAYOUT,MASTER,THEME]}
    for part,fill in zip([SLIDE,LAYOUT,MASTER],layers):
        root=trees[part]; shape=root.find('p:cSld/p:spTree/p:sp',NS); assert shape is not None
        props=shape.find('p:spPr',NS)
        for n in list(props): props.remove(n)
        for n in fragment(fill): props.append(n)
        if hierarchy:
            nv=shape.find('p:nvSpPr/p:nvPr',NS)
            for n in list(nv): nv.remove(n)
            nv.append(fragment('<p:ph type="body" idx="7"/>')[0])
        style=shape.find('p:style',NS)
        if style is not None: shape.remove(style)
        if part==SLIDE and reference is not None:
            shape.insert(list(shape).index(props)+1,fragment(f'<p:style><a:lnRef idx="0"/><a:fillRef idx="{reference}"><a:srgbClr val="AABBCC"/></a:fillRef><a:effectRef idx="0"/><a:fontRef idx="minor"/></p:style>')[0])
    fill_list=trees[THEME].find('a:themeElements/a:fmtScheme/a:fillStyleLst',NS); fill_list.replace(fill_list[0],fragment(theme)[0])
    bg_list=trees[THEME].find('a:themeElements/a:fmtScheme/a:bgFillStyleLst',NS); bg_list.replace(bg_list[0],fragment(solid('FEDCBA'))[0])
    target_id=int(trees[SLIDE].find('p:cSld/p:spTree/p:sp/p:nvSpPr/p:cNvPr',NS).get('id'))
    if mutate: mutate(trees)
    for part,tree in trees.items(): owned[part]=E.tostring(tree,xml_declaration=True,encoding='UTF-8')
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for n,b in owned.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b)
    cases.append({'name':name,'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'nativeId':target_id,'expectations':expected or {},'xsdConforming':conforming})

def expect(**pairs): return {'/styles/targets/0/outcome/fill/'+k.replace('__','/'):v for k,v in pairs.items()}
emit('defaults',expected=expect(kind='none'))
emit('empty-solid',layers=('<a:solidFill/>','',''),expected=expect(kind='solid',color__color__value__slot='bg1'))
emit('empty-gradient',layers=('<a:gradFill/>','',''),expected=expect(kind='gradient',gradient__shade__angle__value=0,gradient__shade__scaled__value=False,gradient__rotateWithShape__value=True,gradient__stops__value__1__position__value='100000'))
emit('empty-pattern',layers=('<a:pattFill/>','',''),expected=expect(pattern__preset__value='pct5',pattern__foreground__color__value__rgb=[0,0,0]))
for i in [0,1000]: emit('reference-'+str(i),reference=i,layers=('',solid('AABBCC'),''),hierarchy=True,expected=expect(kind='none'))
emit('background-style-index',reference=1001,expected=expect(color__color__value__rgb=[254,220,186],color__color__declaredBy__part='/'+THEME))
emit('out-of-range-reference',reference=999,expected={'/styles/targets/0/outcome/reason/kind':'styleIndexOutOfRange'})
emit('unused-reference',layers=(solid('123456'),'',''),reference=4294967295,expected=expect(kind='solid',color__color__value__rgb=[18,52,86]))
emit('local-type-blocks-theme',layers=('<a:gradFill/>','',''),theme=solid('123456'),reference=1,expected=expect(kind='gradient',gradient__stops__value__0__color__color__value__rgb=[0,0,0]))
stops='<a:gsLst><a:gs pos="50.00%"><a:schemeClr val="phClr"><a:alphaMod val="12.123456789%"/></a:schemeClr></a:gs><a:gs pos="+0050000"><a:srgbClr val="ABCDEF"/></a:gs></a:gsLst>'
emit('four-layer-gradient', layers=('<a:gradFill rotWithShape="0"><a:lin/></a:gradFill>','<a:gradFill flip="x"><a:lin scaled="1"/></a:gradFill>','<a:gradFill><a:tileRect l="-12.123456789%"/></a:gradFill>'),theme=f'<a:gradFill>{stops}<a:lin ang="60000"/></a:gradFill>',reference=1,hierarchy=True,expected=expect(gradient__flip__declaredBy__owner__part='/'+LAYOUT,gradient__shade__angle__value=60000,gradient__shade__scaled__value=True,gradient__tileRect__left__value='-12.123456789%',gradient__tileRect__top__value='0',gradient__stops__value__0__color__contextOwner__part='/'+SLIDE))
emit('path-rect-inherits-edges',layers=('<a:gradFill><a:path><a:fillToRect l="0"/></a:path></a:gradFill>','<a:gradFill><a:path path="circle"><a:fillToRect t="25%"/></a:path></a:gradFill>',''),theme='<a:gradFill><a:lin ang="60000"/></a:gradFill>',reference=1,hierarchy=True,expected=expect(gradient__shade__kind='path',gradient__shade__path__value='circle',gradient__shade__fillToRect__top__value='25%',gradient__shade__fillToRect__right__value='50000'))
emit('empty-path-rect',layers=('<a:gradFill><a:path><a:fillToRect/></a:path></a:gradFill>','',''),expected=expect(gradient__shade__fillToRect__top__value='50000'))
emit('schema-tile-rect',layers=('<a:gradFill><a:tileRect/></a:gradFill>','<a:gradFill><a:tileRect t="25%"/></a:gradFill>',''),hierarchy=True,expected=expect(gradient__tileRect__top__value='0',gradient__tileRect__top__declaredBy__kind='schemaDefault'))
emit('pattern-inheritance',layers=('<a:pattFill><a:fgClr><a:srgbClr val="010203"/></a:fgClr></a:pattFill>','<a:pattFill><a:bgClr><a:srgbClr val="AABBCC"/></a:bgClr></a:pattFill>',''),theme='<a:pattFill prst="cross"/>',reference=1,hierarchy=True,expected=expect(pattern__preset__value='cross',pattern__foreground__color__value__rgb=[1,2,3],pattern__background__color__declaredBy__owner__part='/'+LAYOUT))
emit('image-inheritance',layers=('<a:blipFill><a:srcRect l="10%"/><a:tile tx="-0.000000001cm"/></a:blipFill>','<a:blipFill><a:blip r:link="layoutImage"/><a:srcRect t="20%"/><a:tile sy="-20%"/></a:blipFill>',''),theme='<a:blipFill dpi="72"><a:blip r:embed="themeImage"/><a:stretch/></a:blipFill>',reference=1,hierarchy=True,expected=expect(image__embed__declaredBy__part='/'+THEME,image__link__declaredBy__owner__part='/'+LAYOUT,image__sourceRect__top__value='20%',image__mode__kind='tile',image__mode__tile__translateX__value='-0.000000001cm',image__mode__tile__scaleY__value='-20%'))
emit('image-stretch',layers=('<a:blipFill><a:blip r:embed="owned"/><a:stretch><a:fillRect/></a:stretch></a:blipFill>','<a:blipFill><a:stretch><a:fillRect t="25%"/></a:stretch></a:blipFill>',''),hierarchy=True,expected=expect(image__mode__fillRect__top__value='0',image__mode__fillRect__top__declaredBy__kind='schemaDefault'))
emit('empty-image-ids',layers=('<a:blipFill><a:blip r:embed="" r:link=""/></a:blipFill>','',''),expected={'/styles/targets/0/outcome/reason/kind':'missingImage'})
emit('unknown-fill-attribute',layers=('<a:solidFill future="x"/>','',''),expected={'/styles/targets/0/outcome/reason/kind':'retainedContent'},conforming=False)
emit('image-effects',layers=('<a:blipFill><a:blip r:embed="owned"><a:blur rad="0"/></a:blip></a:blipFill>','',''),expected={'/styles/targets/0/outcome/reason/kind':'effectEvaluationRequired'})
def set_bg(trees, part, xml):
    c=trees[part].find('p:cSld',NS); bg=c.find('p:bg',NS)
    if bg is not None: c.remove(bg)
    if xml: c.insert(0,fragment(xml)[0])
def background_case(trees):
    for part in [SLIDE,LAYOUT,MASTER]: set_bg(trees,part,'')
    set_bg(trees,MASTER,'<p:bg><p:bgPr>'+solid('AABBCC')+'</p:bgPr></p:bg>')
def using_background(trees):
    background_case(trees)
    trees[SLIDE].find('p:cSld/p:spTree/p:sp',NS).set('useBgFill','true')
emit('shape-use-background',layers=('<a:noFill/>','',''),mutate=using_background,expected=expect(kind='solid',declaredBy__owner__part='/'+MASTER))
emit('background-inheritance',mutate=background_case,expected={'/styles/targets/4/outcome/fill/declaredBy/owner/part':'/'+MASTER})
def bg_ref(trees): set_bg(trees,SLIDE,'<p:bg><p:bgRef idx="1001"><a:srgbClr val="ABCDEF"/></p:bgRef></p:bg>')
emit('background-reference',mutate=bg_ref,expected={'/styles/targets/4/outcome/fill/color/color/value/rgb':[254,220,186]})
def shade(trees): set_bg(trees,SLIDE,'<p:bg><p:bgPr shadeToTitle="true"><a:noFill/></p:bgPr></p:bg>')
emit('background-shade-title',mutate=shade,expected={'/styles/targets/4/outcome/reason/kind':'unsupportedBackgroundMode'})
def group(trees, root=False):
    tree=trees[SLIDE].find('p:cSld/p:spTree',NS); shape=tree.find('p:sp',NS); tree.remove(shape)
    g=fragment('<p:grpSp><p:nvGrpSpPr><p:cNvPr id="9001" name="group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>'+('<a:grpFill/>' if root else solid('123456'))+'</p:grpSpPr></p:grpSp>')[0]
    g.append(shape);tree.append(g)
    props=tree.find('p:grpSpPr',NS)
    for n in list(props): props.remove(n)
    if root: props.append(fragment(solid('AABBCC'))[0])
emit('physical-group',layers=('<a:grpFill/>','',''),mutate=group,expected=expect(color__color__value__rgb=[18,52,86]))
emit('physical-root-group',layers=('<a:grpFill/>','',''),mutate=lambda t:group(t,True),expected=expect(color__color__value__rgb=[170,187,204],declaredBy__owner__target__kind='rootGroup'))
def line(trees):
    p=trees[SLIDE].find('p:cSld/p:spTree/p:sp/p:spPr',NS)
    p.append(fragment('<a:ln><a:pattFill prst="cross"/></a:ln>')[0])
emit('native-line-fill',mutate=line,expected={'/styles/targets/1/outcome/fill/pattern/preset/value':'cross'})
def picture(trees):
    pic=trees[SLIDE].find('p:cSld/p:spTree/p:pic',NS)
    pic.find('p:nvPicPr/p:nvPr',NS).append(fragment('<p:ph type="body" idx="7"/>')[0])
emit('picture-placeholder',hierarchy=True,mutate=picture,expected={'/styles/targets/2/outcome/fill/kind':'image'})
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':cases},indent=2)+'\n')
print(json.dumps({'ownedPptx':len(cases),'expectations':sum(len(c['expectations']) for c in cases)}))
