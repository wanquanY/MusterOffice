"""Owned PPTX fill cases, assembled independently with ZIP and XML tooling."""
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A, P, MC
from fill_reference import FILL_NAMES, R

ROOT = Path('.codex-work/source-fills'); ROOT.mkdir(parents=True, exist_ok=True)
base = next(c for c in json.loads(Path('.codex-work/pptx-color-map-fixtures/manifest.json').read_text())['cases'] if c['name'] == 'authored')
assert hashlib.sha256(Path(base['path']).read_bytes()).hexdigest() == base['sha256']
with zipfile.ZipFile(base['path']) as z: parts = {n: z.read(n) for n in z.namelist()}
NS = {'a': A, 'p': P}; cases = []
sample_shape = E.fromstring(parts['ppt/slides/slide1.xml']).find('p:cSld/p:spTree/p:sp', NS)
def fragment(xml):
    return E.fromstring(f'<root xmlns:a="{A}" xmlns:p="{P}" xmlns:r="{R}" xmlns:mc="{MC}" xmlns:u="urn:owned:future">{xml}</root>')
def emit(name, xml='', scope='shape', error=None, conforming=True, utf16=False):
    owned = dict(parts)
    part = {'theme': 'ppt/theme/theme2.xml', 'theme-background': 'ppt/theme/theme2.xml', 'theme-line': 'ppt/theme/theme2.xml', 'layout': 'ppt/slideLayouts/slideLayout2.xml', 'master': 'ppt/slideMasters/slideMaster2.xml', 'master-background': 'ppt/slideMasters/slideMaster2.xml', 'layout-background': 'ppt/slideLayouts/slideLayout2.xml'}.get(scope, 'ppt/slides/slide1.xml')
    original = E.fromstring(owned[part]); root = E.Element(original.tag, nsmap={**original.nsmap, 'mc': MC, 'u': 'urn:owned:future', 'r': R})
    root.attrib.update(original.attrib); root.text = original.text
    for c in original: root.append(c)
    nodes = fragment(xml)
    if scope.startswith('theme'):
        list_name = {'theme': 'fillStyleLst', 'theme-background': 'bgFillStyleLst', 'theme-line': 'lnStyleLst'}[scope]
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
        elif scope == 'picture':
            target = tree.find('p:pic/p:blipFill', NS); assert target is not None
            target.getparent().replace(target, nodes[0])
        else:
            shape = tree.find('p:sp', NS)
            if shape is None: shape = copy.deepcopy(sample_shape); tree.append(shape)
            props = shape.find('p:spPr', NS)
            if scope == 'reference':
                old = shape.find('p:style', NS)
                if old is not None: shape.remove(old)
                style = fragment('<p:style><a:lnRef idx="0"/>'+xml+'<a:effectRef idx="0"/><a:fontRef idx="minor"/></p:style>')[0]
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

emit('absent')
for kind in FILL_NAMES: emit('empty-'+kind, '<a:'+kind+'/>')
COLORS = ['<a:srgbClr val="12abEF"/>', '<a:scrgbClr r="50.123456789%" g="+0001" b="-20000"/>', '<a:hslClr hue="60000" sat="150%" lum="-20000"/>', '<a:sysClr val="windowText" lastClr="1a2B3c"/>', '<a:prstClr val="aliceBlue"/>', '<a:schemeClr val="accent3"><a:alpha val="50%"/><a:alpha val="+050000"/></a:schemeClr>']
for i, c in enumerate(COLORS): emit('solid-color-'+str(i), '<a:solidFill>'+c+'</a:solidFill>')
stops = '<a:gsLst><a:gs pos="+0050000">'+COLORS[-1]+'</a:gs><a:gs pos="50.00%">'+COLORS[0]+'</a:gs><a:gs pos="0">'+COLORS[1]+'</a:gs></a:gsLst>'
gradient = '<a:gradFill flip="xy" rotWithShape="false">'+stops+'<a:path path="circle"><a:fillToRect l="-25.123456789%" r="150000"/></a:path><a:tileRect t="0"/></a:gradFill>'
emit('gradient-native-order', gradient); emit('gradient-utf16', gradient, utf16=True)
for shade in ['shape', 'rect', 'circle']:
    emit('gradient-path-'+shade, f'<a:gradFill><a:path path="{shade}"><a:fillToRect/></a:path></a:gradFill>')
for flip in ['none', 'x', 'y', 'xy']:
    emit('gradient-flip-'+flip, f'<a:gradFill flip="{flip}"><a:lin ang="21599999" scaled="true"/></a:gradFill>')
emit('gradient-linear-zero', '<a:gradFill rotWithShape="0"><a:lin ang="0" scaled="0"/></a:gradFill>')
patterns = 'pct5 pct10 pct20 pct25 pct30 pct40 pct50 pct60 pct70 pct75 pct80 pct90 horz vert ltHorz ltVert dkHorz dkVert narHorz narVert dashHorz dashVert cross dnDiag upDiag ltDnDiag ltUpDiag dkDnDiag dkUpDiag wdDnDiag wdUpDiag dashDnDiag dashUpDiag diagCross smCheck lgCheck smGrid lgGrid dotGrid smConfetti lgConfetti horzBrick diagBrick solidDmnd openDmnd dotDmnd plaid sphere weave divot shingle wave trellis zigZag'.split()
for i, pattern in enumerate(patterns): emit('pattern-'+pattern, f'<a:pattFill prst="{pattern}"><a:fgClr>{COLORS[i%6]}</a:fgClr><a:bgClr>{COLORS[(i+1)%6]}</a:bgClr></a:pattFill>')
for c in ['none', 'email', 'screen', 'print', 'hqprint']:
    emit('image-compression-'+c, f'<a:blipFill dpi="4294967295" rotWithShape="1"><a:blip r:embed="owned" r:link="ownedLink" cstate="{c}"/><a:srcRect l="-5.123456789%"/><a:stretch><a:fillRect b="0"/></a:stretch></a:blipFill>')
for align in ['tl', 't', 'tr', 'l', 'ctr', 'r', 'bl', 'b', 'br']:
    emit('image-tile-'+align, f'<a:blipFill dpi="0"><a:tile tx="-0.000000001cm" ty="+00127" sx="0" sy="-20%" flip="y" algn="{align}"/></a:blipFill>')
for unit in ['mm', 'cm', 'in', 'pt', 'pc', 'pi']:
    emit('image-unit-'+unit, f'<a:blipFill><a:tile tx="123.000000000000000001{unit}" ty="-27273042329600"/></a:blipFill>')
for scope in ['shape', 'layout', 'master', 'root-group', 'group', 'theme', 'theme-background']:
    emit('scope-'+scope, gradient, scope=scope)
emit('scope-theme-line', '<a:ln>'+gradient+'</a:ln>', scope='theme-line')
emit('scope-line-pattern', '<a:ln><a:pattFill prst="cross"><a:fgClr>'+COLORS[0]+'</a:fgClr></a:pattFill></a:ln>')
emit('scope-picture', '<p:blipFill><a:blip r:embed="owned"/><a:stretch/></p:blipFill>', scope='picture')
for idx in ['0', '1', '999', '1000', '1001', '4294967295']:
    emit('fill-reference-'+idx, f'<a:fillRef idx="{idx}">{COLORS[-1]}</a:fillRef>', scope='reference')
    emit('background-reference-'+idx, f'<p:bg><p:bgRef idx="{idx}"/></p:bg>', scope='background')
for scope in ['background', 'master-background', 'layout-background']:
    emit('scope-'+scope, '<p:bg bwMode="gray"><p:bgPr shadeToTitle="0">'+gradient+'<a:effectLst/></p:bgPr></p:bg>', scope=scope)
emit('retained-image-effects', '<a:blipFill><a:blip><a:duotone>'+COLORS[0]+COLORS[4]+'</a:duotone><a:extLst><a:ext uri="owned"><u:data><a:tile tx="bad"/></u:data></a:ext></a:extLst></a:blip></a:blipFill>')
emit('retained-attributes', '<a:gradFill future="1"><a:gsLst><a:gs pos="0">'+COLORS[0].replace('/>', ' future="2"/>')+'</a:gs><a:gs pos="100000">'+COLORS[1]+'</a:gs></a:gsLst></a:gradFill>', conforming=False)
emit('selected-fallback', '<mc:AlternateContent><mc:Choice Requires="u"><a:gradFill flip="bad"/></mc:Choice><mc:Fallback>'+gradient+'</mc:Fallback></mc:AlternateContent>')
for name, bad in [
    ('duplicate', '<a:noFill/><a:grpFill/>'), ('nested-leaf', '<a:noFill><a:solidFill/></a:noFill>'),
    ('empty-stops', '<a:gradFill><a:gsLst/></a:gradFill>'), ('one-stop', '<a:gradFill><a:gsLst><a:gs pos="0">'+COLORS[0]+'</a:gs></a:gsLst></a:gradFill>'),
    ('empty-color', '<a:pattFill><a:fgClr/></a:pattFill>'), ('pattern', '<a:pattFill prst="invalid"/>'),
    ('gradient-choice', '<a:gradFill><a:lin/><a:path/></a:gradFill>'), ('gradient-order', '<a:gradFill><a:tileRect/><a:lin/></a:gradFill>'),
    ('gradient-angle', '<a:gradFill><a:lin ang="21600000"/></a:gradFill>'), ('gradient-rotate', '<a:gradFill rotWithShape="yes"/>'),
    ('gradient-flip', '<a:gradFill flip="both"/>'), ('gradient-path', '<a:gradFill><a:path path="ellipse"/></a:gradFill>'),
    ('image-dpi', '<a:blipFill dpi="4294967296"/>'), ('image-choice', '<a:blipFill><a:tile/><a:stretch/></a:blipFill>'),
    ('image-order', '<a:blipFill><a:srcRect/><a:blip/></a:blipFill>'), ('image-cstate', '<a:blipFill><a:blip cstate="high"/></a:blipFill>'),
    ('image-effect-order', '<a:blipFill><a:blip><a:extLst/><a:blur/></a:blip></a:blipFill>'),
]: emit('reject-'+name, bad, error='INPUT_INVALID')
for i, bad in enumerate(['-1', '100001', '-0.01%', '100.01%', '12.345%', 'NaN', '1e4', '']): emit('reject-stop-'+str(i), gradient.replace('+0050000', bad), error='INPUT_INVALID')
for i, bad in enumerate(['+1cm', '.1mm', '1.px', '1e4', 'NaN', '27273042316901', '-27273042329601', '1 px']): emit('reject-coordinate-'+str(i), f'<a:blipFill><a:tile tx="{bad}"/></a:blipFill>', error='INPUT_INVALID')
for i, bad in enumerate(['<p:bg/>', '<p:bg><p:bgPr/></p:bg>', '<p:bg><p:bgRef idx="-1"/></p:bg>', '<p:bg><p:bgRef idx="0"/><p:bgPr><a:noFill/></p:bgPr></p:bg>']): emit('reject-background-'+str(i), bad, scope='background', error='INPUT_INVALID')
# The ECMA regex accepts 100.01%; normative 22.9.2.10 still limits 0..100%.
next(c for c in cases if c['name'] == 'reject-stop-3')['semanticRangeRejection'] = True
(ROOT/'manifest.json').write_text(json.dumps({'base': base, 'cases': cases}, indent=2)+'\n')
print(json.dumps({'cases': len(cases), 'valid': sum(c['error'] is None for c in cases)}))
