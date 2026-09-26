"""Independent DOM projection of DrawingML geometry source declarations.

This deliberately does not evaluate formulas or paths. It derives values and
physical XML bindings from the input package, never from the kernel response.
"""
from lxml import etree as E
from mce_reference import A

def geometry(root, ordinals):
    retained = []
    def record(node, allowed=()):
        if any(k not in allowed for k in node.attrib): retained.append(ordinals[node])
        return {'sourceOrdinal': ordinals[node]}
    def point(node):
        return record(node, ('x', 'y')) | {'x': node.get('x'), 'y': node.get('y')}
    def guide(node):
        return record(node, ('name', 'fmla')) | {'name': node.get('name'), 'formula': node.get('fmla')}
    def handle(node):
        polar = E.QName(node).localname == 'ahPolar'
        mapping = dict(zip(['gdRefR', 'minR', 'maxR', 'gdRefAng', 'minAng', 'maxAng'] if polar else
                           ['gdRefX', 'minX', 'maxX', 'gdRefY', 'minY', 'maxY'],
                           ['guideRadius', 'minRadius', 'maxRadius', 'guideAngle', 'minAngle', 'maxAngle'] if polar else
                           ['guideX', 'minX', 'maxX', 'guideY', 'minY', 'maxY']))
        return record(node, mapping) | {'kind': 'polar' if polar else 'xy', 'position': point(node[0])} | {v: node.get(k) for k, v in mapping.items()}
    def connection(node):
        return record(node, ('ang',)) | {'angle': node.get('ang'), 'position': point(node[0])}
    def rect(node):
        return record(node, ('l', 't', 'r', 'b')) | {v: node.get(k) for k, v in zip(['l', 't', 'r', 'b'], ['left', 'top', 'right', 'bottom'])}
    def command(node):
        local = E.QName(node).localname
        if local == 'arcTo':
            attrs = ['wR', 'hR', 'stAng', 'swAng']
            return record(node, attrs) | {'kind': 'arc'} | {v: node.get(k) for k, v in zip(attrs, ['widthRadius', 'heightRadius', 'startAngle', 'sweepAngle'])}
        fields = {'moveTo': ('move', ['to']), 'lnTo': ('line', ['to']), 'quadBezTo': ('quadratic', ['control', 'to']),
                  'cubicBezTo': ('cubic', ['control1', 'control2', 'to']), 'close': ('close', [])}
        kind, names = fields[local]
        return record(node) | {'kind': kind} | {name: point(child) for name, child in zip(names, node)}
    def path(node):
        return record(node, ('w', 'h', 'fill', 'stroke', 'extrusionOk')) | {
            'width': None if node.get('w') is None else str(int(node.get('w'))),
            'height': None if node.get('h') is None else str(int(node.get('h'))), 'fill': node.get('fill'),
            'stroke': None if node.get('stroke') is None else node.get('stroke').strip() in ('1', 'true'),
            'extrusionOk': None if node.get('extrusionOk') is None else node.get('extrusionOk').strip() in ('1', 'true'),
            'commands': [command(child) for child in node]}
    def lst(local, fn):
        node = root.find(f'{{{A}}}{local}')
        return None if node is None else record(node) | {'entries': [fn(child) for child in node]}
    preset = E.QName(root).localname == 'prstGeom'
    result = record(root, ('prst',) if preset else ())
    if preset:
        definition = {'kind': 'preset', 'preset': root.get('prst').strip(), 'adjustments': lst('avLst', guide)}
    else:
        text = root.find(f'{{{A}}}rect')
        definition = {'kind': 'custom', 'adjustments': lst('avLst', guide), 'guides': lst('gdLst', guide),
                      'handles': lst('ahLst', handle), 'connections': lst('cxnLst', connection),
                      'textRect': None if text is None else rect(text), 'paths': lst('pathLst', path)}
    return result | {'definition': definition, 'retainedOrdinals': sorted(set(retained))}
