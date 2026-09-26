"""Owned PPTX effect cases, assembled independently with ZIP and XML tooling."""
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A, P, MC
from fill_reference import FILL_NAMES, R

ROOT = Path('.codex-work/source-effects'); ROOT.mkdir(parents=True, exist_ok=True)
base = next(c for c in json.loads(Path('.codex-work/pptx-color-map-fixtures/manifest.json').read_text())['cases'] if c['name'] == 'authored')
assert hashlib.sha256(Path(base['path']).read_bytes()).hexdigest() == base['sha256']
with zipfile.ZipFile(base['path']) as z: parts = {n: z.read(n) for n in z.namelist()}
NS = {'a': A, 'p': P}; cases = []
sample_shape = E.fromstring(parts['ppt/slides/slide1.xml']).find('p:cSld/p:spTree/p:sp', NS)
def fragment(xml):
    return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}" xmlns:r="{R}" xmlns:mc="{MC}" xmlns:u="urn:owned:future">{xml}</root>')
def emit(name, xml='', scope='shape', error=None, conforming=True, utf16=False):
    owned = dict(parts)
    part = {'theme': 'ppt/theme/theme2.xml', 'theme-background': 'ppt/theme/theme2.xml', 'theme-line': 'ppt/theme/theme2.xml', 'theme-effects':'ppt/theme/theme2.xml', 'layout': 'ppt/slideLayouts/slideLayout2.xml', 'master': 'ppt/slideMasters/slideMaster2.xml', 'master-background': 'ppt/slideMasters/slideMaster2.xml', 'layout-background': 'ppt/slideLayouts/slideLayout2.xml'}.get(scope, 'ppt/slides/slide1.xml')
    original = E.fromstring(owned[part]); root = E.Element(original.tag, nsmap={**original.nsmap, 'mc': MC, 'u': 'urn:owned:future', 'r': R})
    root.attrib.update(original.attrib); root.text = original.text
    for c in original: root.append(c)
    nodes = fragment(xml)
    if scope.startswith('theme'):
        list_name = {'theme': 'fillStyleLst', 'theme-background': 'bgFillStyleLst', 'theme-line': 'lnStyleLst', 'theme-effects':'effectStyleLst'}[scope]
        target = root.find('a:themeElements/a:fmtScheme/a:'+list_name, NS)
        target.replace(target[0], nodes[0])
    else:
        tree = root.find('p:cSld/p:spTree', NS)
        if scope.endswith('background'):
            common = root.find('p:cSld', NS); old = common.find('p:bg', NS)
            if old is not None: common.remove(old)
            for i, n in enumerate(nodes): common.insert(i, n)
        elif scope == 'root-group':
            target = tree.find('p:grpSpPr', NS)
            for n in list(target): target.remove(n)
            for n in nodes: target.append(n)
        elif scope == 'group':
            group = fragment('<p:grpSp><p:nvGrpSpPr><p:cNvPr id="9001" name="owned group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:grpSp>')[0]
            for n in nodes: group.find('p:grpSpPr', NS).append(n)
            tree.append(group)
        elif scope == 'picture-fill':
            target = tree.find('p:pic/p:blipFill', NS); assert target is not None
            target.getparent().replace(target, nodes[0])
        else:
            shape = tree.find('p:pic' if scope == 'picture' else 'p:sp', NS)
            if shape is None: shape = copy.deepcopy(sample_shape); tree.append(shape)
            props = shape.find('p:spPr', NS)
            if scope == 'reference':
                old = shape.find('p:style', NS)
                if old is not None: shape.remove(old)
                style = fragment('<p:style><a:lnRef idx="0"/><a:fillRef idx="0"/>'+xml+'<a:fontRef idx="minor"/></p:style>')[0]
                shape.insert(list(shape).index(props)+1, style)
            else:
                for n in list(props): props.remove(n)
                for n in nodes: props.append(n)
    owned[part] = E.tostring(root, xml_declaration=True, encoding='UTF-16' if utf16 else 'UTF-8')
    path = ROOT/(name+'.pptx')
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
        for n, b in owned.items():
            info = zipfile.ZipInfo(n, (2026, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED; z.writestr(info, b)
    cases.append({'name': name, 'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'part': '/'+part, 'scope': scope, 'error': error, 'xsdConforming': conforming and error is None})

C = '<a:srgbClr val="123456"><a:alphaMod val="12.123456789%"/></a:srgbClr>'
EFFECTS = {
 'cont':'<a:cont type="tree" name="  child   name  "/>', 'effect':'<a:effect ref="  child   name  "/>',
 'alphaBiLevel':'<a:alphaBiLevel thresh="50.01%"/>', 'alphaCeiling':'<a:alphaCeiling/>',
 'alphaFloor':'<a:alphaFloor/>','alphaInv':f'<a:alphaInv>{C}</a:alphaInv>',
 'alphaMod':'<a:alphaMod><a:cont name="alpha"><a:blur/></a:cont></a:alphaMod>',
 'alphaModFix':'<a:alphaModFix amt="150.123456789%"/>', 'alphaOutset':'<a:alphaOutset rad="-0.000000001cm"/>',
 'alphaRepl':'<a:alphaRepl a="100000"/>', 'biLevel':'<a:biLevel thresh="0"/>',
 'blend':'<a:blend blend="screen"><a:cont type="sib"/></a:blend>',
 'blur':'<a:blur rad="+00042" grow="0"/>',
 'clrChange':f'<a:clrChange useA="false"><a:clrFrom>{C}</a:clrFrom><a:clrTo>{C}</a:clrTo></a:clrChange>',
 'clrRepl':f'<a:clrRepl>{C}</a:clrRepl>', 'duotone':f'<a:duotone>{C}{C}</a:duotone>',
 'fill':'<a:fill><a:gradFill><a:lin ang="0"/></a:gradFill></a:fill>',
 'fillOverlay':'<a:fillOverlay blend="mult"><a:blipFill><a:blip r:embed="owned"><a:grayscl/></a:blip></a:blipFill></a:fillOverlay>',
 'glow':f'<a:glow rad="27273042316900">{C}</a:glow>', 'grayscl':'<a:grayscl/>',
 'hsl':'<a:hsl hue="21599999" sat="-100%" lum="100000"/>',
 'innerShdw':f'<a:innerShdw blurRad="1234" dist="123" dir="0">{C}</a:innerShdw>',
 'lum':'<a:lum bright="-100000" contrast="12.34%"/>',
 'outerShdw':f'<a:outerShdw blurRad="1" dist="2" dir="3" sx="-20%" sy="150.123456789%" kx="-5399999" ky="5399999" algn="br" rotWithShape="false">{C}</a:outerShdw>',
 'prstShdw':f'<a:prstShdw prst="shdw20" dist="1" dir="2">{C}</a:prstShdw>',
 'reflection':'<a:reflection blurRad="0" stA="100.00%" stPos="0" endA="0" endPos="100000" dist="1" dir="2" fadeDir="0" sx="-1%" sy="123000" kx="5399999" ky="-5399999" algn="tl" rotWithShape="true"/>',
 'relOff':'<a:relOff tx="123456789.0001%" ty="-200000"/>', 'softEdge':'<a:softEdge rad="0"/>',
 'tint':'<a:tint hue="60000" amt="-12.34%"/>',
 'xfrm':'<a:xfrm tx="123.000000001pt" ty="-27273042329600" sx="0" sy="100000" kx="-5399999" ky="5399999"/>',
}
emit('absent'); emit('empty-list','<a:effectLst/>'); emit('empty-dag','<a:effectDag/>')
for name,xml in EFFECTS.items(): emit('kind-'+name,'<a:effectDag>'+xml+'</a:effectDag>')
for name in ['alphaModFix','alphaOutset','blur','hsl','lum','reflection','relOff','tint','xfrm','alphaInv']:
    emit('defaults-'+name,f'<a:effectDag><a:{name}/></a:effectDag>')
for n in range(1,21): emit('shadow-'+str(n),f'<a:effectLst><a:prstShdw prst="shdw{n}">{C}</a:prstShdw></a:effectLst>')
for mode in ['over','mult','screen','darken','lighten']: emit('blend-'+mode,f'<a:effectDag><a:blend blend="{mode}"><a:cont/></a:blend></a:effectDag>')
LIST = '<a:effectLst>'+''.join(EFFECTS[n] for n in ['blur','fillOverlay','glow','innerShdw','outerShdw','prstShdw','reflection','softEdge'])+'</a:effectLst>'
for scope in ['shape','layout','master','root-group','group','picture']:
    emit('scope-'+scope,LIST,scope=scope)
for scope in ['background','layout-background','master-background']:
    emit('scope-'+scope,'<p:bg><p:bgPr><a:solidFill>'+C+'</a:solidFill>'+LIST+'</p:bgPr></p:bg>',scope=scope)
emit('scope-theme','<a:effectStyle>'+LIST+'</a:effectStyle>',scope='theme-effects')
for n in [0,1,1000,4294967295]: emit('reference-'+str(n),f'<a:effectRef idx="{n}">{C}</a:effectRef>',scope='reference')
from effect_reference import BLIP_EFFECTS
for name in sorted(BLIP_EFFECTS): emit('blip-'+name,'<p:blipFill><a:blip r:embed="owned">'+EFFECTS[name]+'</a:blip></p:blipFill>',scope='picture-fill')
xml=EFFECTS['blur']
for _ in range(24): xml='<a:fillOverlay blend="over"><a:blipFill><a:blip r:embed="owned">'+xml+'</a:blip></a:blipFill></a:fillOverlay>'
emit('nested-fill-blip','<a:effectDag>'+xml+'</a:effectDag>')
emit('utf16',LIST,utf16=True)
emit('fallback','<mc:AlternateContent><mc:Choice Requires="u"><a:effectDag><a:softEdge rad="invalid"/></a:effectDag></mc:Choice><mc:Fallback>'+LIST+'</mc:Fallback></mc:AlternateContent>')
emit('opaque-extension',LIST+'<a:extLst><a:ext uri="owned"><u:data><a:effectDag><a:softEdge rad="invalid"/></a:effectDag></u:data></a:ext></a:extLst>')
emit('unknown-effect-attribute','<a:effectDag><a:blur future="x"/><a:glow>'+C.replace('val="123456"','val="123456" future="x"')+'</a:glow></a:effectDag>',conforming=False)
emit('unknown-list-attribute','<a:effectLst future="x"/>',conforming=False)
emit('background-empty-list','<p:bg><p:bgPr><a:noFill/><a:effectLst/></p:bgPr></p:bg>',scope='background')
emit('background-empty-dag','<p:bg><p:bgPr><a:noFill/><a:effectDag/></p:bgPr></p:bg>',scope='background')
emit('three-dimensional-style','<a:effectStyle><a:effectLst/><a:scene3d><a:camera prst="orthographicFront"/><a:lightRig rig="threePt" dir="t"/></a:scene3d><a:sp3d/></a:effectStyle>',scope='theme-effects')
for name,xml in [
 ('missing-threshold','<a:alphaBiLevel/>'), ('missing-alpha','<a:alphaRepl/>'),('missing-radius','<a:softEdge/>'),
 ('missing-reference','<a:effect/>'),('missing-color','<a:glow/>'),('one-duotone',f'<a:duotone>{C}</a:duotone>'),
 ('three-duotone',f'<a:duotone>{C}{C}{C}</a:duotone>'),('missing-container','<a:blend blend="over"/>'),
 ('wrong-alpha-context','<a:alphaMod val="10%"/>'), ('missing-fill','<a:fill/>'),
 ('two-fills','<a:fill><a:noFill/><a:grpFill/></a:fill>'), ('bad-preset',f'<a:prstShdw prst="shdw21">{C}</a:prstShdw>'),
 ('bad-blend','<a:blend blend="xor"><a:cont/></a:blend>'),('bad-container','<a:cont type="both"/>'),
 ('positive-coordinate','<a:blur rad="-1"/>'),('coordinate-overflow','<a:blur rad="27273042316901"/>'),
 ('positive-coordinate-unit','<a:blur rad="1pt"/>'),('fixed-angle','<a:xfrm kx="5400000"/>'),
 ('positive-angle','<a:tint hue="21600000"/>'),('fixed-percentage','<a:tint amt="-100001"/>'),
 ('positive-percentage','<a:alphaModFix amt="-1"/>'),('bad-boolean','<a:blur grow="yes"/>'),
 ('positive-fixed','<a:alphaRepl a="100001"/>'),('bad-color-change',f'<a:clrChange><a:clrTo>{C}</a:clrTo><a:clrFrom>{C}</a:clrFrom></a:clrChange>'),
]: emit('reject-'+name,'<a:effectDag>'+xml+'</a:effectDag>',error='INPUT_INVALID')
for name,xml in [('list-order','<a:effectLst><a:reflection/><a:blur/></a:effectLst>'),('list-duplicate','<a:effectLst><a:blur/><a:blur/></a:effectLst>'),('list-wrong-kind','<a:effectLst><a:grayscl/></a:effectLst>'),('properties-choice','<a:effectLst/><a:effectDag/>')]:
    emit('reject-'+name,xml,error='INPUT_INVALID')
emit('reject-blip-kind','<p:blipFill><a:blip><a:reflection/></a:blip></p:blipFill>',scope='picture-fill',error='INPUT_INVALID')
emit('reject-style-empty','<a:effectStyle/>',scope='theme-effects',error='INPUT_INVALID')
emit('reject-semantic-percent','<a:effectDag><a:alphaRepl a="100.01%"/></a:effectDag>',error='INPUT_INVALID')
cases[-1]['semanticRangeRejection']=True
(ROOT/'manifest.json').write_text(json.dumps({'base':base,'cases':cases},indent=2)+'\n')
print(json.dumps({'cases':len(cases),'valid':sum(c['error'] is None for c in cases)}))
