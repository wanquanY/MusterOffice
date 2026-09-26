"""Owned numerical-color corpus built as real, editable OPC/PPTX files."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import A, P

NS = {'a': A, 'p': P}
SLOTS = ['dk1', 'lt1', 'dk2', 'lt2', 'accent1', 'accent2', 'accent3', 'accent4', 'accent5', 'accent6', 'hlink', 'folHlink']
SLIDE = 'ppt/slides/slide1.xml'
THEME = 'ppt/theme/theme2.xml'
MASTER = 'ppt/slideMasters/slideMaster2.xml'


def color(kind, attrs, transforms=()):
    node = E.Element(E.QName(A, kind), {k: str(v) for k, v in attrs.items()})
    for name, value in transforms:
        E.SubElement(node, E.QName(A, name), {} if value is None else {'val': str(value)})
    return node


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('source', type=Path); parser.add_argument('presets', type=Path); parser.add_argument('directory', type=Path)
    args = parser.parse_args(); args.directory.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.source) as package:
        original = {name: package.read(name) for name in package.namelist()}
    presets = json.loads(args.presets.read_text()); assert len(presets) == 190
    cases = []

    def emit(name, definitions, *, slots=None, context=None, expected=None, probe=False, mutation=None):
        parts = copy.deepcopy(original)
        theme = E.fromstring(parts[THEME]); scheme = theme.find('a:themeElements/a:clrScheme', NS)
        for slot, definition in definitions.items():
            parent = scheme.find('a:' + slot, NS)
            for child in list(parent): parent.remove(child)
            parent.append(copy.deepcopy(definition))
        parts[THEME] = E.tostring(theme, encoding='UTF-8', xml_declaration=True)
        root = E.fromstring(parts[SLIDE]); shape = root.xpath('.//p:sp[p:nvSpPr/p:cNvPr/@name="round:1"]', namespaces=NS)[0]
        fill = shape.find('p:spPr/a:solidFill', NS); fill.clear()
        E.SubElement(fill, E.QName(A, 'schemeClr'), val='accent1')
        parts[SLIDE] = E.tostring(root, encoding='UTF-8', xml_declaration=True)
        if mutation: mutation(parts)
        path = args.directory / (name + '.pptx')
        with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as package:
            for part, raw in parts.items():
                info = zipfile.ZipInfo(part, (2026, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
                package.writestr(info, raw)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        request = {'expectedSourceSha256': digest, 'surface': '/' + SLIDE, 'profile': 'ecma376-2016-draft-v1',
                   'colors': slots or ['accent1'], 'context': context or {'systemColors': {}, 'placeholder': None}}
        case = {'name': name, 'path': str(path), 'sha256': digest, 'request': request}
        if expected is not None: case['expectedFirst'] = expected
        if probe: case['externalColorProbe'] = {'slide': 0, 'object': 'round:1'}
        cases.append(case)

    emit('color-srgb', {'accent1': color('srgbClr', {'val': '4080C0'})}, expected=[64,128,192,255])
    emit('color-scrgb', {'accent1': color('scrgbClr', {'r': '50000', 'g': '50.0000000000000000000%', 'b': '50000'})}, expected=[188,188,188,255], probe=True)
    emit('color-scrgb-integer-control', {'accent1': color('scrgbClr', {'r': '50000', 'g': '50000', 'b': '50000'})}, expected=[188,188,188,255], probe=True)
    emit('color-scrgb-extended', {'accent1': color('scrgbClr', {'r': '-50%', 'g': '150%', 'b': '0%'})}, expected=[0,255,0,255])
    emit('color-hsl', {'accent1': color('hslClr', {'hue': 14400000, 'sat': '100%', 'lum': '50%'})}, expected=[0,0,255,255], probe=True)
    emit('color-hsl-integer-control', {'accent1': color('hslClr', {'hue': 14400000, 'sat': '100000', 'lum': '50000'})}, expected=[0,0,255,255], probe=True)
    emit('color-hsl-achromatic-chain', {'accent1': color('hslClr', {'hue': 14400000, 'sat': '0', 'lum': '50000'}, [('sat',100000)])}, expected=[0,0,255,255], probe=True)
    emit('color-hsl-black-chain', {'accent1': color('hslClr', {'hue': 7200000, 'sat': '100000', 'lum': '50000'}, [('lum',0),('lum',50000)])}, expected=[0,255,0,255], probe=True)
    emit('color-hsl-clamp-chain', {'accent1': color('srgbClr', {'val':'0000FF'}, [('hueOff',10800000),('hueOff',-5400000)])}, expected=[128,0,255,255], probe=True)
    emit('color-hsl-reference-chain', {'accent1': color('schemeClr', {'val':'accent2'}, [('sat',100000)]), 'accent2':color('hslClr', {'hue':14400000,'sat':'0','lum':'50000'})}, expected=[0,0,255,255], probe=True)
    emit('color-system-cache', {'accent1': color('sysClr', {'val': 'windowText', 'lastClr': '123456'})}, expected=[18,52,86,255])
    emit('color-system-host', {'accent1': color('sysClr', {'val': 'windowText', 'lastClr': '123456'})}, context={'systemColors': {'windowText': [1,2,3]}, 'placeholder': None}, expected=[1,2,3,255])
    emit('color-system-missing', {'accent1': color('sysClr', {'val': 'windowText'})}, expected='missingSystemColor')
    emit('color-placeholder-missing', {'accent1': color('schemeClr', {'val': 'phClr'})}, expected='missingPlaceholder')
    emit('color-placeholder-context', {'accent1': color('schemeClr', {'val': 'phClr'}, [('alphaMod',50000)])}, context={'systemColors': {}, 'placeholder': [10,20,30,128]}, expected=[10,20,30,64])
    emit('color-scheme-chain', {'accent1': color('schemeClr', {'val': 'accent2'}, [('alphaMod',50000)]), 'accent2': color('srgbClr', {'val': '4080C0'}, [('alpha',50000)])}, expected=[64,128,192,64])
    emit('color-scheme-cycle', {'accent1': color('schemeClr', {'val': 'accent2'}), 'accent2': color('schemeClr', {'val': 'accent1'})}, expected='schemeCycle')
    emit('color-numeric-overflow', {'accent1': color('scrgbClr', {'r': '9'*400+'%', 'g': '0', 'b': '0'})}, expected='numericRange')
    emit('color-percentage-budget', {'accent1': color('scrgbClr', {'r': '0.'+'0'*65536+'1%', 'g': '0', 'b': '0'})}, expected='LIMIT_EXCEEDED')

    transforms = {'tint':50000, 'shade':50000, 'comp':None, 'inv':None, 'gray':None,
                  'alpha':50000, 'alphaOff':-25000, 'alphaMod':50000, 'hue':10800000, 'hueOff':-10800000,
                  'hueMod':200000, 'sat':50000, 'satOff':-25000, 'satMod':200000,
                  'lum':50000, 'lumOff':25000, 'lumMod':50000,
                  'red':50000, 'redOff':-20000, 'redMod':50000, 'green':50000, 'greenOff':-20000,
                  'greenMod':50000, 'blue':50000, 'blueOff':-20000, 'blueMod':50000, 'gamma':None, 'invGamma':None}
    assert len(transforms) == 28
    for name, value in transforms.items():
        emit('color-transform-' + name, {'accent1': color('srgbClr', {'val':'4080C0'}, [(name,value)])}, probe=name in ['redMod','inv','gamma','gray','hueOff'])
    for name, base, transform, expected in [
        ('red-half','FF0000',('redMod',50000),[128,0,0,255]),
        ('red-offset','FF0000',('redOff',-20000),[204,0,0,255]),
        ('green-tint','00FF00',('tint',50000),[188,255,188,255]),
        ('red-hue-clamp','FF0000',('hueOff',-5400000),[255,0,0,255]),
    ]:
        emit('color-example-'+name, {'accent1': color('srgbClr', {'val':base}, [transform])}, expected=expected, probe=True)
    emit('color-alpha-order-a', {'accent1': color('srgbClr', {'val':'4080C0'}, [('alphaMod',50000),('alphaOff',50000)])}, expected=[64,128,192,255])
    emit('color-alpha-order-b', {'accent1': color('srgbClr', {'val':'4080C0'}, [('alphaOff',50000),('alphaMod',50000)])}, expected=[64,128,192,128])
    emit('color-repeated-transforms', {'accent1': color('srgbClr', {'val':'4080C0'}, [('lumMod',80000),('satOff',10000),('lumMod',80000),('tint',25000),('alpha',25000)])})
    emit('color-transform-budget', {'accent1': color('srgbClr', {'val':'4080C0'}, [('alphaMod',100000)]*65536)}, expected='LIMIT_EXCEEDED')
    for offset in range(0, len(presets), 12):
        names = list(presets)[offset:offset+12]
        emit('color-preset-batch-'+str(offset//12), {slot: color('prstClr', {'val':name}) for slot,name in zip(SLOTS,names)}, slots=SLOTS[:len(names)])
    emit('color-preset-short-goldenrod', {'accent1': color('prstClr', {'val':'ltGoldenrodYellow'})}, expected=[250,250,120,255], probe=True)

    def no_map(parts):
        root = E.fromstring(parts[MASTER]); root.remove(root.find('p:clrMap',NS)); parts[MASTER] = E.tostring(root)
    emit('color-missing-map', {}, mutation=no_map, expected='missingColorMap')
    # Direct dk/lt names do not require a role map.
    emit('color-direct-slot-no-map', {'dk1':color('srgbClr',{'val':'112233'})}, slots=['dk1'], mutation=no_map, expected=[17,34,51,255])
    manifest = {'format':'musteroffice.color-fixtures/1', 'baseSha256': hashlib.sha256(args.source.read_bytes()).hexdigest(),
                'presetFactsSha256':hashlib.sha256(args.presets.read_bytes()).hexdigest(), 'cases':cases}
    (args.directory/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps({'cases':len(cases), 'manifest':str(args.directory/'manifest.json')}))


if __name__ == '__main__': main()
