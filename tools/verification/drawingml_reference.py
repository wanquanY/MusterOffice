"""Independent DOM color declaration oracle shared by theme and line checks."""
from lxml import etree as E

def local(node):
    return E.QName(node).localname


def color(node, ordinals):
    name = local(node)
    rgb = lambda value: list(bytes.fromhex(value))
    if name == 'srgbClr':
        value = {'kind': 'srgb', 'rgb': rgb(node.get('val'))}
    elif name == 'scrgbClr':
        value = {'kind': 'scRgb', **dict(zip(['red', 'green', 'blue'], (node.get(a) for a in ['r', 'g', 'b'])))}
    elif name == 'hslClr':
        value = {'kind': 'hsl', 'hue': int(node.get('hue')), 'saturation': node.get('sat'), 'luminance': node.get('lum')}
    elif name == 'sysClr':
        value = {'kind': 'system', 'color': node.get('val'), 'lastColor': rgb(node.get('lastClr')) if node.get('lastClr') else None}
    elif name == 'schemeClr':
        value = {'kind': 'scheme', 'slot': node.get('val')}
    else:
        assert name == 'prstClr'
        value = {'kind': 'preset', 'color': node.get('val')}
    transforms = []
    for transform in node:
        name = local(transform)
        item = {'kind': name}
        if transform.get('val') is not None:
            item['value'] = int(transform.get('val')) if name in ['hue', 'hueOff'] else transform.get('val')
        transforms.append(item)
    return {'sourceOrdinal': ordinals[node], 'value': value, 'transforms': transforms}


