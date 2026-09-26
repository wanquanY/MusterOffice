"""Independent DOM projection of source fills; no streaming parser state."""
from lxml import etree as E
from drawingml_reference import local, color
from mce_reference import A, P

R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
NS = {'a': A, 'p': P}
FILL_NAMES = ['noFill', 'solidFill', 'gradFill', 'pattFill', 'blipFill', 'grpFill']

def declarations(node, ordinals, effect_nodes=None):
    from effect_reference import read as effect, BLIP_EFFECTS
    if effect_nodes is None: effect_nodes = {}
    retained = []
    def bind(n): return {'sourceOrdinal': ordinals[n]}
    def attr(n, k): return n.get(k).strip() if n.get(k) is not None else None
    def integer(n, k): return int(n.get(k)) if n.get(k) is not None else None
    def boolean(n, k): return None if n.get(k) is None else n.get(k).strip() in ['true', '1']
    def attributes(n, allowed):
        if any(k not in allowed for k in n.attrib): retained.append(ordinals[n])
    def colored(n):
        attributes(n, {'scrgbClr': ['r', 'g', 'b'], 'hslClr': ['hue', 'sat', 'lum'], 'sysClr': ['val', 'lastClr']}.get(local(n), ['val']))
        for t in n: attributes(t, [] if local(t) in ['comp', 'inv', 'gray', 'gamma', 'invGamma'] else ['val'])
        return color(n, ordinals)
    def optional_color(n): return colored(n[0]) if len(n) else None
    def rect(n):
        attributes(n, ['l', 't', 'r', 'b'])
        return {**bind(n), **{out: attr(n, key) for out, key in [('left', 'l'), ('top', 't'), ('right', 'r'), ('bottom', 'b')]}}
    def reference(n):
        start = len(retained); attributes(n, ['idx'])
        result = {**bind(n), 'index': int(n.get('idx')), 'color': optional_color(n)}
        result['retainedOrdinals'] = retained[start:]
        return result
    def fill(n):
        start = len(retained); name = local(n)
        allowed = {'gradFill': ['flip', 'rotWithShape'], 'pattFill': ['prst'], 'blipFill': ['dpi', 'rotWithShape']}.get(name, [])
        attributes(n, allowed)
        result = {**bind(n)}
        if name in ['noFill', 'grpFill']: value = {'kind': 'none' if name == 'noFill' else 'group'}
        elif name == 'solidFill': value = {'kind': 'solid', 'color': optional_color(n)}
        elif name == 'gradFill':
            value = {'kind': 'gradient', 'stops': None, 'shade': None, 'tileRect': None, 'flip': attr(n, 'flip'), 'rotateWithShape': boolean(n, 'rotWithShape')}
            for child in n:
                tag = local(child)
                if tag == 'gsLst':
                    attributes(child, []); entries = []
                    for stop in child:
                        attributes(stop, ['pos']); entries.append({**bind(stop), 'position': attr(stop, 'pos'), 'color': colored(stop[0])})
                    value['stops'] = {**bind(child), 'entries': entries}
                elif tag == 'lin':
                    attributes(child, ['ang', 'scaled']); value['shade'] = {'kind': 'linear', **bind(child), 'angle': integer(child, 'ang'), 'scaled': boolean(child, 'scaled')}
                elif tag == 'path':
                    attributes(child, ['path']); value['shade'] = {'kind': 'path', **bind(child), 'path': attr(child, 'path'), 'fillToRect': rect(child[0]) if len(child) else None}
                elif tag == 'tileRect': value['tileRect'] = rect(child)
                else: raise AssertionError(tag)
        elif name == 'pattFill':
            value = {'kind': 'pattern', 'preset': attr(n, 'prst'), 'foreground': None, 'background': None}
            for child in n:
                attributes(child, []); value[{'fgClr': 'foreground', 'bgClr': 'background'}[local(child)]] = {**bind(child), 'color': colored(child[0])}
        elif name == 'blipFill':
            value = {'kind': 'image', 'blip': None, 'sourceRect': None, 'mode': None, 'dpi': integer(n, 'dpi'), 'rotateWithShape': boolean(n, 'rotWithShape')}
            for child in n:
                tag = local(child)
                if tag == 'blip':
                    begin = len(retained); attributes(child, ['cstate', '{'+R+'}embed', '{'+R+'}link'])
                    effects = []
                    for c in child:
                        if E.QName(c).namespace == A and local(c) in BLIP_EFFECTS:
                            effect(c, ordinals, effect_nodes); effects.append(ordinals[c])
                        else: retained.append(ordinals[c])
                    value['blip'] = {**bind(child), 'embed': child.get('{'+R+'}embed'), 'link': child.get('{'+R+'}link'), 'compression': attr(child, 'cstate'), 'retainedOrdinals': retained[begin:]}
                    if effects: value['blip']['effectNodes'] = effects
                elif tag == 'srcRect': value['sourceRect'] = rect(child)
                elif tag == 'tile':
                    attributes(child, ['tx', 'ty', 'sx', 'sy', 'flip', 'algn'])
                    value['mode'] = {'kind': 'tile', **bind(child), **{out: attr(child, key) for out, key in [('translateX','tx'), ('translateY','ty'), ('scaleX','sx'), ('scaleY','sy'), ('flip','flip'), ('alignment','algn')]}}
                elif tag == 'stretch':
                    attributes(child, []); value['mode'] = {'kind': 'stretch', **bind(child), 'fillRect': rect(child[0]) if len(child) else None}
                else: raise AssertionError(tag)
        else: raise AssertionError(name)
        return {**result, 'definition': value, 'retainedOrdinals': retained[start:]}
    if local(node) in FILL_NAMES: return fill(node)
    if local(node) in ['fillRef', 'bgRef', 'effectRef', 'lnRef']: return reference(node)
    assert local(node) == 'bg'
    attributes(node, ['bwMode']); child = node[0]
    if local(child) == 'bgRef': value = {'kind': 'reference', **reference(child)}
    else:
        begin = len(retained); attributes(child, ['shadeToTitle'])
        value = {'kind': 'properties', **bind(child), 'shadeToTitle': boolean(child, 'shadeToTitle'), 'fill': fill(child[0])}
        for c in list(child)[1:]:
            if E.QName(c).namespace == A and local(c) in ['effectLst','effectDag']:
                value['effects'] = effect(c, ordinals, effect_nodes)
            else: retained.append(ordinals[c])
        value['retainedOrdinals'] = retained[begin:]
    return {**bind(node), 'blackWhiteMode': attr(node, 'bwMode'), 'definition': value, 'retainedOrdinals': retained}
