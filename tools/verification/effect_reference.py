"""Independent DOM effect declarations, with one physical-node catalog per part.

This oracle checks explicit declarations, not native effect graph evaluation.
Its input is the independently selected MCE tree and physical ordinals.
"""
from lxml import etree as E
import re
from drawingml_reference import local, color
from mce_reference import A

BLIP_EFFECTS = set('alphaBiLevel alphaCeiling alphaFloor alphaInv alphaMod alphaModFix alphaRepl biLevel blur clrChange clrRepl duotone fillOverlay grayscl hsl lum tint'.split())
# XML -> wire fields. Types: lexical string (s), integer (i), EMU (e), boolean (b), token (t).
FIELDS = {
 'cont': ('container', 'type:containerType:s name:name:t'),
 'effect': ('reference', 'ref:reference:t'),
 'alphaBiLevel': ('alphaBiLevel','thresh:threshold:s'), 'alphaCeiling': ('alphaCeiling',''),
 'alphaFloor': ('alphaFloor',''), 'alphaInv': ('alphaInverse',''), 'alphaMod': ('alphaModulate',''),
 'alphaModFix': ('alphaModulateFixed','amt:amount:s'), 'alphaOutset': ('alphaOutset','rad:radius:s'),
 'alphaRepl': ('alphaReplace','a:alpha:s'), 'biLevel': ('biLevel','thresh:threshold:s'),
 'blend': ('blend','blend:blend:s'), 'blur': ('blur','rad:radius:e grow:grow:b'),
 'clrChange': ('colorChange','useA:useAlpha:b'), 'clrRepl': ('colorReplace',''),
 'duotone': ('duotone',''), 'fill': ('fill',''), 'fillOverlay': ('fillOverlay','blend:blend:s'),
 'glow': ('glow','rad:radius:e'), 'grayscl': ('grayscale',''),
 'hsl': ('hsl','hue:hue:i sat:saturation:s lum:luminance:s'),
 'innerShdw': ('innerShadow','blurRad:blurRadius:e dist:distance:e dir:direction:i'),
 'lum': ('luminance','bright:brightness:s contrast:contrast:s'),
 'outerShdw': ('outerShadow','blurRad:blurRadius:e dist:distance:e dir:direction:i sx:scaleX:s sy:scaleY:s kx:skewX:i ky:skewY:i algn:alignment:s rotWithShape:rotateWithShape:b'),
 'prstShdw': ('presetShadow','prst:preset:s dist:distance:e dir:direction:i'),
 'reflection': ('reflection','blurRad:blurRadius:e stA:startAlpha:s stPos:startPosition:s endA:endAlpha:s endPos:endPosition:s dist:distance:e dir:direction:i fadeDir:fadeDirection:i sx:scaleX:s sy:scaleY:s kx:skewX:i ky:skewY:i algn:alignment:s rotWithShape:rotateWithShape:b'),
 'relOff': ('relativeOffset','tx:translateX:s ty:translateY:s'), 'softEdge': ('softEdge','rad:radius:e'),
 'tint': ('tint','hue:hue:i amt:amount:s'),
 'xfrm': ('transform','sx:scaleX:s sy:scaleY:s kx:skewX:i ky:skewY:i tx:translateX:s ty:translateY:s'),
}

def read(node, ordinals, catalog):
    from fill_reference import declarations
    name = local(node); retained = []
    def unknown(n, allowed):
        if any(a not in allowed for a in n.attrib): retained.append(ordinals[n])
    def colored(n):
        unknown(n, {'scrgbClr':['r','g','b'], 'hslClr':['hue','sat','lum'], 'sysClr':['val','lastClr']}.get(local(n), ['val']))
        for c in n: unknown(c, [] if local(c) in ['comp','inv','gray','gamma','invGamma'] else ['val'])
        return color(n, ordinals)
    def effect(n):
        read(n, ordinals, catalog)
        return ordinals[n]
    if name == 'effectStyle':
        unknown(node, [])
        props = read(node[0], ordinals, catalog)
        retained += [ordinals[c] for c in list(node)[1:]]
        return {'sourceOrdinal':ordinals[node], 'effects':props, 'retainedOrdinals':retained}
    if name == 'effectRef': return declarations(node, ordinals, catalog)
    if name == 'effectLst':
        unknown(node, [])
        nodes = []
        for c in node:
            if E.QName(c).namespace == A: nodes.append(effect(c))
            else: retained.append(ordinals[c])
        return {'sourceOrdinal':ordinals[node], 'definition':{'kind':'list','nodes':nodes}, 'retainedOrdinals':retained}
    kind, fields = FIELDS['cont' if name == 'effectDag' else name]
    attrs = [f.split(':') for f in fields.split()]; unknown(node, [a[0] for a in attrs])
    value = {'kind':kind}
    for xml, wire, typ in attrs:
        raw = node.get(xml)
        raw = raw.strip() if raw is not None and typ != 't' else raw
        value[wire] = None if raw is None else {'i':lambda:int(raw), 'e':lambda:str(int(raw)), 'b':lambda:raw in ['true','1'], 't':lambda:re.sub(r'[ \t\r\n]+', ' ', raw).strip(' '), 's':lambda:raw}[typ]()
    children = [c for c in node if E.QName(c).namespace == A]
    retained += [ordinals[c] for c in node if E.QName(c).namespace != A]
    if name in ['cont','effectDag']: value['nodes'] = [effect(c) for c in children]
    elif name in ['alphaMod','blend']: value['container'] = effect(children[0])
    elif name in ['alphaInv','clrRepl','glow','innerShdw','outerShdw','prstShdw']: value['color'] = colored(children[0]) if children else None
    elif name == 'duotone': value.update(first=colored(children[0]), second=colored(children[1]))
    elif name == 'clrChange':
        for key,c in zip(['from','to'],children): unknown(c, []); value[key]=colored(c[0])
    elif name in ['fill','fillOverlay']:
        value['fill'] = declarations(children[0], ordinals, catalog)
        retained += value['fill']['retainedOrdinals']
    result = {'sourceOrdinal':ordinals[node], 'definition':value, 'retainedOrdinals':retained}
    assert str(ordinals[node]) not in catalog, ordinals[node]
    catalog[str(ordinals[node])] = result
    if name == 'effectDag': return {'sourceOrdinal':ordinals[node], 'definition':{'kind':'dag','root':ordinals[node]}, 'retainedOrdinals':[]}
    return result

def verify_part(actual, dom, ordinals, theme=False):
    """Check every active declaration and catalog, including effects inside fills."""
    from fill_reference import declarations, FILL_NAMES
    from mce_reference import P
    ns={'a':A,'p':P}; catalog={}; count=0
    def check(value, node, is_effect=True):
        nonlocal count
        expected = None if node is None else (read(node,ordinals,catalog) if is_effect else declarations(node,ordinals,catalog))
        assert value == expected, (value,expected)
        if node is not None: count+=1
    def first(parent, names):
        return None if parent is None else next((n for n in parent if E.QName(n).namespace==A and local(n) in names),None)
    if theme:
        fmt=actual['formatScheme']
        if fmt:
            for family in ['fills','backgroundFills','effects']:
                for entry in fmt[family]:
                    n=next(n for n in dom.iter() if ordinals[n]==entry['sourceOrdinal'])
                    check(entry.get('effectStyle' if family=='effects' else 'fill'), n, family=='effects')
    else:
        check(actual.get('background'),dom.find('p:cSld/p:bg',ns),False)
        props=dom.find('p:cSld/p:spTree/p:grpSpPr',ns)
        check(actual.get('rootGroupEffects'),first(props,['effectLst','effectDag']))
        check(actual.get('rootGroupFill'),first(props,FILL_NAMES),False)
        for n in dom.iter():
            if E.QName(n).namespace!=P or local(n) not in ['sp','pic','cxnSp','grpSp']:continue
            if any(local(p)=='extLst' for p in n.iterancestors()):continue
            identity=n.find('./*/p:cNvPr',ns);assert identity is not None
            obj=next(o for o in actual['objects'] if o['nativeId']==int(identity.get('id')))
            props=n.find('p:grpSpPr' if local(n)=='grpSp' else 'p:spPr',ns)
            check(obj.get('effects'),first(props,['effectLst','effectDag']))
            check(obj.get('effectReference'),n.find('p:style/a:effectRef',ns))
            check(obj.get('fill'),first(props,FILL_NAMES),False)
            check(obj.get('fillReference'),n.find('p:style/a:fillRef',ns),False)
            check(obj.get('pictureFill'),n.find('p:blipFill',ns),False)
    assert actual.get('effectNodes',{})==catalog, (actual.get('effectNodes',{}),catalog)
    return count,len(catalog)
