"""Check actual source->Native glyph probes with XML and Fraction references.

This corpus exercises an owned synthetic font. It is not Office/WPS visual
certification or a general reference implementation of native font classification.
"""
import hashlib
import json
from fractions import Fraction as F
from pathlib import Path
import subprocess
import zipfile
from line_geometry_math import reference
from mce_reference import project, A, P

ROOT = Path('.codex-work/source-glyphs')
NATIVE = ROOT / 'native'
NATIVE.mkdir(exist_ok=True)
NS = dict(a=A, p=P)
manifest = json.loads(Path('fixtures/fonts/manifest-paragraph.json').read_text())['manifest']
U = 1 << 32


def entry(path):
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


def bounds(points):
    if not points:
        return None
    return dict(min={k: str(min(p[i] for p in points)) for i, k in enumerate(['x', 'y'])},
                max={k: str(max(p[i] for p in points)) for i, k in enumerate(['x', 'y'])})


counts = dict(packages=0, paragraphs=0, sourceRanges=0, fontBindings=0, shapeOperations=0,
              pathOperations=0, shapeErrors=0, pathErrors=0, scenes=0, glyphs=0,
              originCoordinates=0, pathBounds=0)
inputs, outputs = [], []
for pptx in sorted((ROOT / 'fixtures').glob('*.pptx')):
    plan_path = pptx.with_suffix('.json')
    plan = json.loads(plan_path.read_text())
    sha = entry(pptx)['sha256']
    assert sha == pptx.stem == plan['source']['sourceSha256']
    target = plan['source']['object']
    # Invoke the actual current development executable, not a Python rendering
    # model. Previous release artifacts and their reports are never overwritten.
    run = subprocess.run(['target/debug/examples/source_glyphs', str(pptx), target['part'],
                          str(target['nativeId'])], check=True, capture_output=True)
    native_path = NATIVE / (pptx.stem + '.json')
    native_path.write_bytes(run.stdout)
    native = json.loads(run.stdout)
    assert native['paragraphs'] == plan['paragraphs']
    assert native['source'] == plan['source']
    assert native['verifiedFaces'] == 1
    with zipfile.ZipFile(pptx) as z:
        documents = {('/' + n): project(z.read(n), True) for n in z.namelist() if n.endswith('.xml')}
    root, _, ordinals = documents[target['part']]
    shape = next(s for s in root.findall('p:cSld/p:spTree/p:sp', NS)
                 if int(s.find('p:nvSpPr/p:cNvPr', NS).get('id')) == target['nativeId'])
    paragraphs = shape.findall('p:txBody/a:p', NS)
    assert len(paragraphs) == len(plan['paragraphs'])
    counts['packages'] += 1
    for i, (xml, p) in enumerate(zip(paragraphs, plan['paragraphs'])):
        cascade = plan['source']['paragraphs'][i]
        assert p['sourceOrdinal'] == cascade['sourceOrdinal'] == ordinals[xml]
        elements = [n for n in xml if n.tag in ['{'+A+'}r', '{'+A+'}br', '{'+A+'}fld']]
        assert len(elements) == len(p['sources'])
        text = ''
        for index, (element, source) in enumerate(zip(elements, p['sources'])):
            value = '\u2028' if element.tag == '{'+A+'}br' else ''.join(element.find('a:t', NS).itertext())
            assert source == dict(start=len(text), end=len(text)+len(value), run=index,
                                  sourceOrdinal=ordinals[element],
                                  kind='break' if element.tag == '{'+A+'}br' else 'text')
            text += value
            counts['sourceRanges'] += 1
        assert text == p['text']
        position = 0
        for span in p['spans']:
            assert position < span['end'] <= len(text)
            position = span['end']
        assert position == len(text)
        assert len(p['styles']) == len(p['geometry'])
        for font in p['fonts']:
            source_style = cascade['endStyle'] if font['run'] is None else cascade['runs'][font['run']]['style']
            attrs = source_style['attributes']
            style = p['styles'][font['style']]
            assert style['language'] == (attrs['language'] or 'und').lower()
            assert style['fontStyle'] == ('boldItalic' if attrs['bold'] and attrs['italic'] else
                                          'bold' if attrs['bold'] else 'italic' if attrs['italic'] else 'regular')
            assert style['features'] == [dict(tag='kern', start=0, end=None,
                                              value=int(attrs['kerning'] is not None and attrs['size'] >= attrs['kerning']))]
            assert p['geometry'][font['style']] == dict(fontSize=str(attrs['size'] * 127), baselineShift='0')
            assert style['typeface'] == font['font']['typeface']
            # Physical declaration reference must identify the actual XML node.
            origin = font['font']['declaredBy']['origin']
            owner = origin['object']['part'] if origin['kind'] == 'object' else origin['part']
            node = next(n for n, at in documents[owner][2].items() if at == origin['sourceOrdinal'])
            assert node.get('typeface') == font['font']['authoredFont']['typeface']
            theme = font['font']['theme']
            if theme is None:
                assert node.get('typeface') == style['typeface']
            else:
                tr = documents[theme['scheme']['part']][0]
                collection = tr.find('a:themeElements/a:fontScheme/a:'+theme['collection']+'Font', NS)
                supplement = collection.findall('a:font', NS)[theme['supplemental']]
                assert supplement.get('script') == font['themeScript']
                assert supplement.get('typeface') == style['typeface']
            counts['fontBindings'] += 1
        assert p['endStyle'] == next(f['style'] for f in p['fonts'] if f['run'] is None)
        counts['paragraphs'] += 1
    seen_shape = set()
    for op in native['results']:
        paragraph = op['paragraph']
        p = plan['paragraphs'][paragraph]
        if paragraph not in seen_shape:
            seen_shape.add(paragraph)
            counts['shapeOperations'] += 1
            counts['shapeErrors'] += int('error' in op['shape'])
        counts['pathOperations'] += 1
        for operation in [op['shape'], op['paths']]:
            if 'result' in operation:
                binding = operation['result']
                assert binding['sourceSha256'] == sha and binding['object'] == target
                assert binding['paragraph'] == paragraph and binding['sourceOrdinal'] == p['sourceOrdinal']
        if 'error' in op['paths']:
            assert op['paths']['error'] in ['invalid text request: manifest typeface unavailable',
                                            'invalid text request: manifest font style unavailable']
            counts['pathErrors'] += 1
            continue
        result = op['paths']['result']['computation']
        r = result['paths']
        assert r['layout']['work']['verifiedFaces'] == 1
        scene = r['scene']
        assert scene is not None, r['issues']
        counts['scenes'] += 1
        style_inputs = [dict(language=s['language'], features=s['features'], candidates=[b['candidate']],
                             suppressDottedCircle=s['suppressDottedCircle'], maxGlyphs=s['maxGlyphs'])
                        for s, b in zip(p['styles'], result['bindings'])]
        q = dict(shaping=dict(paragraph=dict(text=p['text'], fonts=manifest['fonts'], styles=style_inputs)),
                 styles=p['geometry'], strutStyle=p['endStyle'], spacing=dict(kind='natural'))
        exact = reference(q, r['layout']['geometry'], True)
        expected_positions = [dict(g, line=i) for i, line in enumerate(exact['lines']) for g in line['glyphs']]
        assert len(expected_positions) == len(scene['glyphs'])
        path_points = []
        for path in scene['paths']:
            assert all(c['kind'] in ['move', 'line', 'close'] for c in path['commands'])
            points = [tuple(int(c['to'][k]) for k in ['x', 'y']) for c in path['commands'] if 'to' in c]
            assert path['bounds'] == bounds(points)
            path_points.append(points)
            counts['pathBounds'] += 1
        all_points = []
        for actual, expected in zip(scene['glyphs'], expected_positions):
            assert all(actual[k] == expected[k] for k in ['line', 'source', 'glyph'])
            assert all(F(int(actual['origin'][k]), U) == expected[k] for k in ['x', 'y'])
            dx, dy = [int(actual['origin'][k]) for k in ['x', 'y']]
            all_points.extend((x+dx, y+dy) for x, y in path_points[actual['path']])
            counts['glyphs'] += 1
            counts['originCoordinates'] += 2
        assert scene['bounds'] == bounds(all_points)
        counts['pathBounds'] += 1
    inputs.extend([entry(pptx), entry(plan_path)])
    outputs.append(entry(native_path))
assert counts['packages'] and counts['scenes'] and counts['fontBindings']
report = dict(format='musteroffice.source-glyph-native-reference/1', scope=__doc__, counts=counts,
              inputs=inputs, outputs=outputs, executable=entry(Path('target/debug/examples/source_glyphs')),
              fontInputs=[entry(Path(p)) for p in ['fixtures/fonts/owned.ttf', 'fixtures/fonts/manifest-paragraph.json']])
(ROOT / 'reference.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(counts))
