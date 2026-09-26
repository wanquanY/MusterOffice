"""Owned real PPTX probes of property layers, choices and theme selection."""
import copy,hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P,MC,R
ROOT=Path('.codex-work/line-style');ROOT.mkdir(exist_ok=True,parents=True)
NS={'a':A,'p':P};REL='http://schemas.openxmlformats.org/package/2006/relationships';CT='http://schemas.openxmlformats.org/package/2006/content-types'
base=next(c for c in json.loads(Path('.codex-work/pptx-color-map-fixtures/manifest.json').read_text())['cases'] if c['name']=='authored')
assert hashlib.sha256(Path(base['path']).read_bytes()).hexdigest()==base['sha256']
with zipfile.ZipFile(base['path']) as z:parts={n:z.read(n) for n in z.namelist()}
S='ppt/slides/slide1.xml';L='ppt/slideLayouts/slideLayout2.xml';M='ppt/slideMasters/slideMaster2.xml';T='ppt/theme/theme2.xml'
CASES=[]
def parse(s):return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}">{s}</root>')
def emit(name,local='',style=None,layout='',layout_style=None,master='',master_style=None,theme=None,hierarchy=False,override=None,defaults=None,mutation=None):
    owned=copy.deepcopy(parts)
    for part,objname,line,ref in [(S,'title:1',local,style),(L,'rule:layout',layout,layout_style),(M,'footer:master',master,master_style)]:
        root=E.fromstring(owned[part]);tree=root.find('p:cSld/p:spTree',NS)
        obj=next(n for n in tree.findall('p:sp',NS) if n.find('p:nvSpPr/p:cNvPr',NS).get('name')==objname)
        for node in list(tree):
            if node is not obj and E.QName(node).localname not in ['nvGrpSpPr','grpSpPr']:tree.remove(node)
        props=obj.find('p:spPr',NS);props.clear()
        props.extend(list(parse('<a:xfrm><a:off x="914400" y="914400"/><a:ext cx="2743200" cy="1371600"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/>'+line)))
        for old in obj.findall('p:style',NS):obj.remove(old)
        if ref is not None:
            style_node=E.Element(f'{{{P}}}style')
            style_node.extend(list(parse(f'<a:lnRef idx="{ref}"><a:srgbClr val="1256EF"/></a:lnRef><a:fillRef idx="0"/><a:effectRef idx="0"/><a:fontRef idx="minor"/>')))
            obj.insert(list(obj).index(props)+1,style_node)
        if hierarchy:E.SubElement(obj.find('p:nvSpPr/p:nvPr',NS),f'{{{P}}}ph',type='body',idx='7')
        tx=obj.find('p:txBody',NS)
        if tx is not None:
            tx.clear();tx.extend(list(parse('<a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="1000"/><a:t>Owned line probe</a:t></a:r></a:p>')))
        owned[part]=E.tostring(root,xml_declaration=True,encoding='UTF-8')
    root=E.fromstring(owned[T]);lst=root.find('a:themeElements/a:fmtScheme/a:lnStyleLst',NS);lst.clear()
    line=theme if theme is not None else '<a:ln w="50800" cap="rnd"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="dash"/><a:miter lim="400000"/></a:ln>'
    lst.extend(list(parse(line+'<a:ln w="76200"><a:solidFill><a:srgbClr val="00AA55"/></a:solidFill></a:ln><a:ln/>')))
    if defaults is not None:
        node=E.SubElement(root,f'{{{A}}}objectDefaults');sp=E.SubElement(node,f'{{{A}}}spDef');sp.extend(list(parse('<a:spPr>'+defaults+'</a:spPr><a:bodyPr/><a:lstStyle/>')))
    owned[T]=E.tostring(root,xml_declaration=True,encoding='UTF-8')
    if override is not None:
        t=E.fromstring(owned[T]);fm=t.find('a:themeElements/a:fmtScheme',NS);ls=fm.find('a:lnStyleLst',NS);ls.clear();ls.extend(list(parse(override+'<a:ln/><a:ln/>')))
        o=E.Element(f'{{{A}}}themeOverride',nsmap={'a':A});o.append(fm);owned['ppt/theme/lineOverride.xml']=E.tostring(o,xml_declaration=True,encoding='UTF-8')
        rel='ppt/slides/_rels/slide1.xml.rels';r=E.fromstring(owned[rel]);E.SubElement(r,f'{{{REL}}}Relationship',Id='lineOverride',Type=R+'/themeOverride',Target='../theme/lineOverride.xml');owned[rel]=E.tostring(r)
        c=E.fromstring(owned['[Content_Types].xml']);E.SubElement(c,f'{{{CT}}}Override',PartName='/ppt/theme/lineOverride.xml',ContentType='application/vnd.openxmlformats-officedocument.themeOverride+xml');owned['[Content_Types].xml']=E.tostring(c)
    if mutation:mutation(owned)
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for part,raw in owned.items():
            info=zipfile.ZipInfo(part,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,raw)
    obj=E.fromstring(owned[S]).find('p:cSld/p:spTree/p:sp/p:nvSpPr/p:cNvPr',NS)
    CASES.append({'name':name,'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'object':int(obj.get('id'))})

emit('defaults')
emit('theme',style=1)
emit('theme-no-color',style=1,theme='<a:ln><a:solidFill/></a:ln>')
emit('partial',local='<a:ln w="0"><a:solidFill/><a:miter/><a:headEnd type="triangle"/></a:ln>',style=1,layout='<a:ln cmpd="dbl"><a:headEnd w="lg"/></a:ln>',master='<a:ln algn="in"><a:tailEnd len="lg"/></a:ln>',hierarchy=True)
emit('parent-before-default',layout='<a:ln w="114300"><a:solidFill><a:srgbClr val="AA0022"/></a:solidFill></a:ln>',hierarchy=True)
emit('local-style-before-parent',style=1,layout='<a:ln w="114300"><a:solidFill><a:srgbClr val="AA0022"/></a:solidFill></a:ln>',hierarchy=True)
emit('parent-direct-before-parent-style',layout='<a:ln w="114300"/>',layout_style=1,hierarchy=True)
emit('zero-reference-parent',style=0,layout='<a:ln w="114300"><a:solidFill><a:srgbClr val="AA0022"/></a:solidFill></a:ln>',hierarchy=True)
emit('zero-reference-alone',style=0)
emit('no-fill-block',local='<a:ln><a:noFill/></a:ln>',style=1)
emit('empty-dash-block',local='<a:ln><a:custDash/></a:ln>',style=1)
emit('preset-dash-inherit',local='<a:ln><a:prstDash/></a:ln>',style=1)
emit('different-dash-kind',local='<a:ln><a:prstDash/></a:ln>',style=1,theme='<a:ln><a:custDash><a:ds d="200000" sp="100000"/></a:custDash></a:ln>')
emit('round-block',local='<a:ln><a:round/></a:ln>',style=1)
emit('miter-default',local='<a:ln><a:miter/></a:ln>')
emit('miter-zero',local='<a:ln><a:miter lim="0"/></a:ln>',style=1)
emit('miter-exact',local='<a:ln><a:miter/></a:ln>',style=1,theme='<a:ln><a:miter lim="800.123456789%"/></a:ln>')
emit('miter-same-kind-parent',local='<a:ln><a:miter/></a:ln>',style=1,theme='<a:ln><a:round/></a:ln>',layout='<a:ln><a:miter lim="500000"/></a:ln>',hierarchy=True)
emit('empty-solid-does-not-take-no-fill',local='<a:ln><a:solidFill/></a:ln>',style=1,theme='<a:ln><a:noFill/></a:ln>')
emit('color-order',local='<a:ln><a:solidFill><a:schemeClr val="phClr"><a:tint val="12.34%"/><a:alphaMod val="12.3456789%"/><a:alpha val="+0050000"/></a:schemeClr></a:solidFill></a:ln>',style=1)
emit('direct-color-inherits-reference',local='<a:ln><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>',layout_style=1,hierarchy=True)
emit('theme-override-inherited-reference',layout_style=1,hierarchy=True,override='<a:ln w="127000"><a:solidFill><a:srgbClr val="00CC44"/></a:solidFill></a:ln>')
emit('initial-object-default-not-cascade',defaults='<a:ln w="152400"><a:solidFill><a:srgbClr val="00CC44"/></a:solidFill></a:ln>')
emit('unresolved-index',style=4)
emit('unresolved-gradient',local='<a:ln><a:gradFill/></a:ln>')
emit('unused-gradient',local='<a:ln><a:noFill/></a:ln>',style=1,theme='<a:ln><a:gradFill/></a:ln>')
emit('retained-attribute',local='<a:ln owned="unknown"/>')
emit('retained-extension',local='<a:ln><a:extLst><a:ext uri="owned"/></a:extLst></a:ln>')
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':CASES},indent=2)+'\n')
print(json.dumps({'cases':len(CASES)}))
