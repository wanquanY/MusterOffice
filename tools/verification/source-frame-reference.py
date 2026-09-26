"""Independent native XML/Fraction check of the owned horizontal frame corpus.

Re-executes the real Native test host, including isolated failure cases. Checks
source paragraph identity, exact insets, first/continuation widths, line geometry,
spacing, alignment, glyph origins and polygon bounds. This corpus has only A and
empty paragraphs; it is not an Office/WPS layout oracle or visual certification.
"""
import argparse
from fractions import Fraction as F
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile
from line_geometry_math import UNIT, nearest, quantize, reference
from mce_reference import project, A, P

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--executable', required=True, type=Path)
args = parser.parse_args()
ROOT = Path('.codex-work/source-frame')
FIXTURES = ROOT / 'fixtures'
NS = dict(a=A, p=P)
manifest_path = Path('fixtures/fonts/manifest-paragraph.json')
manifest = json.loads(manifest_path.read_text())['manifest']


def entry(path):
    path = Path(path)
    data = path.read_bytes()
    return dict(path=str(path), byteLength=len(data), sha256=hashlib.sha256(data).hexdigest())


def coordinate(s):
    match = re.fullmatch(r'([+-]?[0-9]+(?:\.[0-9]+)?)(mm|cm|in|pt|pc|pi)?', s)
    assert match, s
    factor = dict(mm=36000, cm=360000, **{'in': 914400}, pt=12700, pc=152400, pi=152400)
    return F(match[1]) * factor.get(match[2], 1)


def point(x, y):
    assert (x * UNIT).denominator == (y * UNIT).denominator == 1
    return dict(x=str(int(x * UNIT)), y=str(int(y * UNIT)))


def bounds(points):
    if not points:
        return None
    return dict(min=point(min(x for x, _ in points), min(y for _, y in points)),
                max=point(max(x for x, _ in points), max(y for _, y in points)))


def paragraph_spacing(p, name):
    item = p.find('a:pPr/a:' + name, NS)
    if item is None:
        return None if name == 'lnSpc' else F(0)
    assert len(item) == 1 and item[0].tag == '{' + A + '}spcPts'
    return F(int(item[0].get('val')) * 127)


env = dict(os.environ, MO_SOURCE_FRAME_EVIDENCE_DIR=str(FIXTURES.resolve()))
for name in ['MO_FRAME_TEST_CHILD', 'MO_FRAME_TEST_CASE']:
    env.pop(name, None)
run = subprocess.run([str(args.executable), '--test-threads=1'], env=env,
                     capture_output=True, timeout=120)
(ROOT / 'reference-native.log').write_bytes(run.stdout + run.stderr)
assert run.returncode == 0, run.stdout + run.stderr
assert b'test result: ok. 7 passed;' in run.stdout
counts = dict(packages=0, paragraphs=0, lines=0, glyphs=0, originCoordinates=0,
              pathBounds=0, fractionalRegions=0, fractionalLineHeights=0)
inputs, outputs = [], []
for pptx in sorted(FIXTURES.glob('*.pptx')):
    output = pptx.with_suffix('.json')
    plan = json.loads(output.read_text())
    assert entry(pptx)['sha256'] == pptx.stem == plan['text']['sourceSha256']
    target = plan['text']['object']
    with zipfile.ZipFile(pptx) as z:
        xml, _, ordinals = project(z.read(target['part'].lstrip('/')), True)
    shape = next(s for s in xml.findall('p:cSld/p:spTree/p:sp', NS)
                 if int(s.find('p:nvSpPr/p:cNvPr', NS).get('id')) == target['nativeId'])
    body = shape.find('p:txBody/a:bodyPr', NS)
    assert body is not None and body.find('a:noAutofit', NS) is not None
    extent = shape.find('p:spPr/a:xfrm/a:ext', NS)
    width, height = [F(extent.get(k)) for k in ['cx', 'cy']]
    preset = shape.find('p:spPr/a:prstGeom', NS)
    if preset is not None:
        assert preset.get('prst') == 'rect'
        assert plan['region']['source']['kind'] == 'declaration'
    else:
        assert shape.find('p:spPr/a:custGeom/a:rect', NS) is None
        assert plan['region']['source']['kind'] == 'shapeBounds'
    left, top, right, bottom = [coordinate(body.get(k, default)) for k, default in
                               [('lIns', '91440'), ('tIns', '45720'),
                                ('rIns', '91440'), ('bIns', '45720')]]
    native_region = [left, top, width-right, height-bottom]
    region = [quantize(v) for v in native_region]
    assert plan['region']['inner'] == dict(min=point(*region[:2]), max=point(*region[2:]))
    error = max(abs(a-b) for a, b in zip(native_region, region))
    reported_error = F(int(plan['region']['conversionErrorBound']), UNIT)
    assert error <= reported_error <= F(1, UNIT)
    counts['fractionalRegions'] += int(any(v.denominator != 1 for v in native_region))
    outer = body.get('spcFirstLastPara', '0') in ['true', '1']
    anchor = body.get('anchor', 't')
    paragraphs = shape.findall('p:txBody/a:p', NS)
    assert len(paragraphs) == len(plan['paragraphs']) == len(plan['inputs'])
    accumulated, local_glyphs, all_points = F(0), [], []
    paragraph_offsets = []
    any_center = anchor == 'ctr'
    for pi, (xml_p, p, source) in enumerate(zip(paragraphs, plan['paragraphs'], plan['inputs'])):
        assert source['sourceOrdinal'] == p['spec']['sourceOrdinal'] == ordinals[xml_p]
        value = ''.join(xml_p.xpath('./a:r/a:t/text()', namespaces=NS))
        assert value == source['text'] and set(value) <= {'A'}
        attrs_node = xml_p.find('a:pPr', NS)
        attrs = {} if attrs_node is None else attrs_node.attrib
        mar_l, mar_r, indent = [F(attrs.get(k, '0')) for k in ['marL', 'marR', 'indent']]
        rtl = attrs.get('rtl', '0') in ['true', '1']
        align = attrs.get('algn', 'l')
        any_center |= align == 'ctr'
        rest_width = region[2] - region[0] - mar_l - mar_r
        first_width = rest_width - indent
        space = paragraph_spacing(xml_p, 'lnSpc')
        spacing = dict(kind='natural') if space is None else dict(kind='exact', height=str(int(space)))
        before, after = [paragraph_spacing(xml_p, name) for name in ['spcBef', 'spcAft']]
        assert p['spec'] == dict(sourceOrdinal=ordinals[xml_p],
                                 widths=dict(first=str(int(first_width*UNIT)), rest=str(int(rest_width*UNIT))),
                                 left=str(int(mar_l*UNIT)), indent=str(int(indent*UNIT)), indentFromRight=rtl,
                                 spacing=spacing, before=str(int(before*UNIT)), after=str(int(after*UNIT)),
                                 alignment=align, overflow='emergencyGrapheme' if attrs.get('latinLnBrk') == '1' else 'keepUnbreakable')
        result = p['computed']['geometry']
        scene = result['paths']['scene']
        geometry = result['paths']['layout']['geometry']
        assert scene is not None and not result['paths']['issues'] and not geometry['issues']
        q = dict(shaping=dict(paragraph=dict(text=source['text'], fonts=manifest['fonts'],
                     styles=[dict(candidates=[b['candidate']]) for b in p['computed']['bindings']])),
                 styles=source['geometry'], strutStyle=source['endStyle'], spacing=spacing)
        expected = reference(q, geometry, True)
        numeric = ['top', 'baseline', 'bottom', 'height', 'advance', 'advanceY', 'penMin', 'penMax']
        assert result['precise'] == dict(height=str(int(expected['height']*UNIT)), lines=[
            {k: str(int(line[k]*UNIT)) for k in numeric} for line in expected['lines']])
        assert geometry['layout']['height'] == str(nearest(expected['height']))
        for line, wire in zip(expected['lines'], geometry['layout']['lines']):
            for k in numeric:
                assert wire[k] == str(nearest(line[k]))
        # This corpus uses a non-ligating owned A with an advance of 600/1000 em.
        # Independently determine the farthest fitting scalar boundary.
        ends, start = [], 0
        while start < len(value):
            available = first_width if start == 0 else rest_width
            if attrs.get('latinLnBrk') == '1':
                size = int(source['geometry'][0]['fontSize'])
                end = max(n for n in range(start+1, len(value)+1)
                          if quantize(F((n-start)*size*600, 1000)) <= available)
            else:
                end = len(value)
            ends.append(end)
            start = end
        if not value:
            ends.append(0)
        assert [d['end']['scalarOffset'] for d in result['paths']['layout']['decisions']] == ends
        if pi > 0 or outer:
            accumulated += before
        offsets = []
        for li, line in enumerate(expected['lines']):
            available = first_width if li == 0 else rest_width
            shift = F(0) if align == 'l' else available-line['penMax']
            if align == 'ctr':
                shift = quantize((available-line['penMax']-line['penMin'])/2)
            x = region[0]+mar_l+(indent if li == 0 and not rtl else 0)+shift
            offsets.append((x, accumulated))
            counts['fractionalLineHeights'] += int(line['height'].denominator != 1)
        paragraph_offsets.append(offsets)
        expected_glyphs = [(li, glyph) for li, line in enumerate(expected['lines']) for glyph in line['glyphs']]
        assert len(expected_glyphs) == len(scene['glyphs'])
        path_points = []
        for path in scene['paths']:
            assert all(c['kind'] in ['move', 'line', 'close'] for c in path['commands'])
            points = [(F(int(c['to']['x']), UNIT), F(int(c['to']['y']), UNIT))
                      for c in path['commands'] if 'to' in c]
            assert path['bounds'] == bounds(points)
            path_points.append(points)
            counts['pathBounds'] += 1
        paragraph_points = []
        for gi, ((li, glyph), actual) in enumerate(zip(expected_glyphs, scene['glyphs'])):
            assert actual['line'] == li and actual['source'] == glyph['source'] and actual['glyph'] == glyph['glyph']
            assert actual['origin'] == point(glyph['x'], glyph['y'])
            paragraph_points.extend((x+glyph['x'], y+glyph['y']) for x, y in path_points[actual['path']])
            x, y = glyph['x']+offsets[li][0], glyph['y']+offsets[li][1]
            local_glyphs.append((pi, gi, x, y))
            all_points.extend((px+x, py+y) for px, py in path_points[actual['path']])
        assert scene['bounds'] == bounds(paragraph_points)
        counts['pathBounds'] += 1
        accumulated += expected['height']
        if pi+1 < len(paragraphs) or outer:
            accumulated += after
        counts['paragraphs'] += 1
        counts['lines'] += len(expected['lines'])
    assert plan['contentHeight'] == str(int(accumulated*UNIT))
    free = region[3]-region[1]-accumulated
    vertical = region[1] + (F(0) if anchor == 't' else quantize(free/2) if anchor == 'ctr' else free)
    for actual, offsets in zip(plan['paragraphs'], paragraph_offsets):
        assert actual['lineOffsets'] == [point(x, y+vertical) for x, y in offsets]
    assert plan['glyphs'] == [dict(paragraph=p, glyph=g, origin=point(x, y+vertical)) for p, g, x, y in local_glyphs]
    assert plan['bounds'] == bounds([(x, y+vertical) for x, y in all_points])
    assert plan['alignmentRoundingBound'] == ('1' if any_center else '0')
    assert plan['work']['glyphs'] == len(local_glyphs)
    counts['packages'] += 1
    counts['glyphs'] += len(local_glyphs)
    counts['originCoordinates'] += 2*len(local_glyphs)
    counts['pathBounds'] += 1
    inputs.append(entry(pptx))
    outputs.append(entry(output))
assert counts['packages'] == 15 and counts['fractionalRegions'] and counts['fractionalLineHeights']
report = dict(format='musteroffice.source-frame-native-reference/1', scope=__doc__, counts=counts,
              executable=entry(args.executable), executionLog=entry(ROOT / 'reference-native.log'),
              inputs=inputs, outputs=outputs,
              fontInputs=[entry(p) for p in [manifest_path, Path('fixtures/fonts/owned.ttf')]])
(ROOT / 'reference.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(counts))
