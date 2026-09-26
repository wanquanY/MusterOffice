"""Owned fill color inputs derived from owned native expression probes."""
import copy,hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P
ROOT=Path('.codex-work/fill-colors');ROOT.mkdir(exist_ok=True,parents=True)
NS={'a':A,'p':P};S='ppt/slides/slide1.xml';cases=[]
sha=lambda b:hashlib.sha256(b).hexdigest()
for c in json.loads(Path('.codex-work/line-colors/manifest.json').read_text())['cases']:
    assert sha(Path(c['path']).read_bytes())==c['sha256']
    with zipfile.ZipFile(c['path']) as z:original={n:z.read(n) for n in z.namelist()}
    variants=['solid']+(['gradient','pattern'] if not c['name'].startswith('color-') else [])
    for variant in variants:
        parts=dict(original);root=E.fromstring(parts[S])
        for shape in root.findall('p:cSld/p:spTree/p:sp',NS):
            props=shape.find('p:spPr',NS);color=props.find('a:ln/a:solidFill/*',NS);assert color is not None
            old=props.find('a:noFill',NS);assert old is not None
            fill=E.Element('{'+A+'}'+{'solid':'solidFill','gradient':'gradFill','pattern':'pattFill'}[variant])
            if variant=='solid':fill.append(copy.deepcopy(color))
            elif variant=='gradient':
                stops=E.SubElement(fill,'{'+A+'}gsLst')
                for position in ['50000','0','50000']:
                    stop=E.SubElement(stops,'{'+A+'}gs',pos=position);stop.append(copy.deepcopy(color))
            else:
                fill.set('prst','cross')
                for name in ['fgClr','bgClr']:E.SubElement(fill,'{'+A+'}'+name).append(copy.deepcopy(color))
            props.replace(old,fill)
            style=shape.find('p:style',NS)
            if style is not None:
                ref=style.find('a:lnRef',NS);target=style.find('a:fillRef',NS);assert ref is not None and target is not None
                target.attrib.clear();target.attrib.update(ref.attrib)
                for child in list(target):target.remove(child)
                for child in ref:target.append(copy.deepcopy(child))
        parts[S]=E.tostring(root,xml_declaration=True,encoding='UTF-8')
        name=variant+'-'+c['name'];path=ROOT/(name+'.pptx')
        with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
            for n,raw in parts.items():
                info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,raw)
        cases.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'objects':c['objects'],'context':c['context'],'expected':c['expected'],'base':{'path':c['path'],'sha256':c['sha256']},'intentionalMissingMap':c['name'] in ['color-missing-map','color-direct-slot-no-map']})
# Unknown native context must be diagnosed only when the selected color needs it.
base=next(c for c in cases if c['name']=='solid-subbyte-reference')
for location in ['reference','color']:
    for used in [False,True]:
        with zipfile.ZipFile(base['path']) as z:parts={n:z.read(n) for n in z.namelist()}
        dom=E.fromstring(parts[S]);shape=dom.find('p:cSld/p:spTree/p:sp',NS)
        ref=shape.find('p:style/a:fillRef',NS);(ref if location=='reference' else ref[0]).set('future','owned')
        if not used:
            fill=shape.find('p:spPr/a:solidFill',NS);fill.clear();E.SubElement(fill,'{'+A+'}srgbClr',val='123456')
        parts[S]=E.tostring(dom,xml_declaration=True,encoding='UTF-8')
        name='retained-'+location+('-used' if used else '-unused');path=ROOT/(name+'.pptx')
        with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
            for n,raw in parts.items():
                info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,raw)
        cases.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'objects':base['objects'],'context':base['context'],'expected':'evaluated','base':{'path':base['path'],'sha256':base['sha256']},'retainedContextProbe':True})
# Cross-owner color contexts and background/root-group targets use original
# authored package parts, not another renderer's output.
base=json.loads(Path('.codex-work/fill-resolution/manifest.json').read_text())['cases'][0]
L='ppt/slideLayouts/slideLayout2.xml';T='ppt/theme/theme2.xml'
def element(text):return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}">{text}</root>')[0]
for name in ['mixed-owner-pattern','background-reference-color','root-group-redirect']:
    assert sha(Path(base['path']).read_bytes())==base['sha256']
    with zipfile.ZipFile(base['path']) as z:parts={n:z.read(n) for n in z.namelist()}
    trees={n:E.fromstring(parts[n]) for n in [S,L,T]};shape=trees[S].find('p:cSld/p:spTree/p:sp',NS);native_id=int(shape.find('p:nvSpPr/p:cNvPr',NS).get('id'))
    targets=[{'kind':'object','nativeId':native_id}];context={'systemColors':{},'placeholder':None}
    if name=='mixed-owner-pattern':
        for part,key,rgb in [(S,'fgClr','FF0000'),(L,'bgClr','0000FF')]:
            node=trees[part].find('p:cSld/p:spTree/p:sp',NS);props=node.find('p:spPr',NS);props.clear()
            props.append(element(f'<a:pattFill prst="cross"><a:{key}><a:schemeClr val="phClr"/></a:{key}></a:pattFill>'))
            nv=node.find('p:nvSpPr/p:nvPr',NS)
            for old in list(nv):nv.remove(old)
            nv.append(element('<p:ph type="body" idx="7"/>'))
            for old in node.findall('p:style',NS):node.remove(old)
            style=element(f'<p:style><a:lnRef idx="0"/><a:fillRef idx="1"><a:srgbClr val="{rgb}"/></a:fillRef><a:effectRef idx="0"/><a:fontRef idx="minor"/></p:style>');node.insert(list(node).index(props)+1,style)
    elif name=='background-reference-color':
        common=trees[S].find('p:cSld',NS)
        for old in common.findall('p:bg',NS):common.remove(old)
        common.insert(0,element('<p:bg><p:bgRef idx="1001"><a:srgbClr val="123456"/></p:bgRef></p:bg>'))
        fills=trees[T].find('a:themeElements/a:fmtScheme/a:bgFillStyleLst',NS);fills.replace(fills[0],element('<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>'))
        targets=[{'kind':'background'}]
    else:
        props=shape.find('p:spPr',NS);props.clear();props.append(element('<a:grpFill/>'))
        props=trees[S].find('p:cSld/p:spTree/p:grpSpPr',NS);props.clear();props.append(element('<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>'))
        context['placeholder']=[44,55,66,255]
    for part,dom in trees.items():parts[part]=E.tostring(dom,xml_declaration=True,encoding='UTF-8')
    path=ROOT/(name+'.pptx')
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        for n,raw in parts.items():
            info=zipfile.ZipInfo(n,(2026,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,raw)
    cases.append({'name':name,'path':str(path),'sha256':sha(path.read_bytes()),'objects':[native_id],'targets':targets,'context':context,'expected':'evaluated','base':{'path':base['path'],'sha256':base['sha256']}})
(ROOT/'manifest.json').write_text(json.dumps({'format':'musteroffice.fill-color-fixtures/1','cases':cases},indent=2)+'\n')
print(json.dumps({'files':len(cases)}))
