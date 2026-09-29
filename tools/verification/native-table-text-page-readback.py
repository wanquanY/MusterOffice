"""Independent XML, clip and pixel checks for owned native table text layers.

The input is the private shared text compiler's output. This does not exercise
the full table page renderer or establish Office/WPS visual compatibility.
"""
import argparse
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import zipfile
from lxml import etree

NS = {'a': 'http://schemas.openxmlformats.org/drawingml/2006/main',
      'p': 'http://schemas.openxmlformats.org/presentationml/2006/main'}
UNIT = 1 << 32


def receipt(path):
    data = path.read_bytes()
    return {'path': str(path), 'sha256': hashlib.sha256(data).hexdigest(), 'byteLength': len(data)}


def rectangle(values):
    x, y, right, bottom = values
    return {'min': {'x': str(x * UNIT), 'y': str(y * UNIT)},
            'max': {'x': str(right * UNIT), 'y': str(bottom * UNIT)}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus-dir', type=Path, required=True)
    parser.add_argument('--schema-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert not args.output.exists(), 'do not replace existing evidence'
    schema = etree.XMLSchema(etree.parse(str(args.schema_dir / 'dml-main.xsd')))
    results = []
    for path in sorted(args.corpus_dir.glob('*.pptx')):
        with zipfile.ZipFile(path) as package:
            root = etree.fromstring(package.read('ppt/slides/slide1.xml'))
        ordinals = {node: n for n, node in enumerate(root.iter())}
        obj = root.find('.//p:graphicFrame', NS)
        native_id = int(obj.find('p:nvGraphicFramePr/p:cNvPr', NS).get('id'))
        table = obj.find('.//a:tbl', NS)
        schema.assertValid(table)
        rows = table.findall('a:tr', NS)
        widths = [int(c.get('w')) for c in table.findall('a:tblGrid/a:gridCol', NS)]
        heights = [int(r.get('h')) for r in rows]
        rtl = table.find('a:tblPr', NS).get('rtl', '0') in ('1', 'true')
        off = obj.find('p:xfrm/a:off', NS)
        xoff = int(off.get('x'))
        cells, expected, flat = {}, [], 0
        for r, row in enumerate(rows):
            for c, cell in enumerate(row.findall('a:tc', NS)):
                cells[r, c] = cell, flat
                flat += len(cell.findall('a:txBody/a:p', NS))
                if (cell.get('hMerge', '0') not in ('1', 'true')
                        and cell.get('vMerge', '0') not in ('1', 'true')
                        and cell.find('a:txBody', NS) is not None):
                    expected.append((r, c))
        inputs = {suffix: path.with_suffix('.'+suffix)
                  for suffix in ('content.json', 'scene.json', 'request.json', 'rgba')}
        content = json.loads(inputs['content.json'].read_bytes())
        scene = json.loads(inputs['scene.json'].read_bytes())
        request = json.loads(inputs['request.json'].read_bytes())
        frames = content['texts']
        assert [(f['frame']['text']['cell']['row'], f['frame']['text']['cell']['column'])
                for f in frames] == expected
        palette = {}
        clip_ids = set()
        glyphs = decorations = clipped = 0
        for n, frame in enumerate(frames):
            value = frame['frame']
            at = value['text']['cell']
            r, c = at['row'], at['column']
            cell, start = cells[r, c]
            sr, sc = int(cell.get('rowSpan', '1')), int(cell.get('gridSpan', '1'))
            x = sum(widths[c+sc:]) if rtl else sum(widths[:c])
            y = sum(heights[:r])
            outer = [x, y, x+sum(widths[c:c+sc]), y+sum(heights[r:r+sr])]
            inner = outer.copy()
            props = cell.find('a:tcPr', NS)
            for k, (name, default) in enumerate((('marL', 91440), ('marT', 45720),
                                                ('marR', 91440), ('marB', 45720))):
                inner[k] += int(props.get(name, str(default))) * (1 if k < 2 else -1)
            assert value['region']['outer'] == rectangle(outer)
            assert value['region']['inner'] == rectangle(inner)
            assert value['region']['source'] == {
                'kind': 'tableCell', 'cell': at, 'sourceOrdinal': ordinals[cell],
                'region': {'origin': at, 'rows': sr, 'columns': sc}}
            assert value['text'].get('paragraphStart', 0) == start
            assert value['text']['object'] == {'part': '/ppt/slides/slide1.xml', 'nativeId': native_id}
            assert value['text']['sourceSha256'] == receipt(path)['sha256']
            paragraphs = cell.findall('a:txBody/a:p', NS)
            assert [p['sourceOrdinal'] for p in value['text']['paragraphs']] == [ordinals[p] for p in paragraphs]
            assert value['work']['glyphs'] == len(value['glyphs'])
            horizontal = props.get('horzOverflow', 'clip') == 'clip'
            references = [s['instance'] for s in content['textSources'] if s['textBinding'] == n]
            references += [s['instance'] for s in content['decorationSources'] if s['textBinding'] == n]
            if horizontal:
                assert value['clip']['horizontal'] and not value['clip']['vertical']
                assert value['clip']['bounds'] == rectangle(outer)
                clipped += 1
                used = {scene['instances'][i]['clip'] for i in references}
                if references:
                    assert len(used) == 1 and None not in used
                    clip_id = used.pop()
                    assert clip_id not in clip_ids, 'cell masks must not be reused for a different region'
                    clip_ids.add(clip_id)
                    clip = scene['clips'][clip_id]
                    assert clip['transform'] is None and clip['parent'] is None
                    points = [c['to'] for c in scene['paths'][clip['path']]['commands'] if 'to' in c]
                    xs = [int(p['x']) for p in points]
                    assert min(xs) == (xoff+outer[0])*UNIT and max(xs) == (xoff+outer[2])*UNIT
            else:
                assert 'clip' not in value
                assert all(scene['instances'][i].get('clip') is None for i in references)
            colors = {tuple(c['rgba']) for c in frame['clusters'] if c['rgba'] is not None}
            for color in colors:
                assert color[3] == 255
                palette.setdefault(color[:3], []).append((xoff+outer[0], xoff+outer[2]) if horizontal else None)
            if cell.find('.//a:rPr/a:noFill', NS) is not None:
                assert not colors and not references and value['glyphs']
            glyphs += len(value['glyphs'])
            decorations += len(frame['decorations'])
        assert glyphs == content['textWork']['glyphs']
        viewport = request['viewport']
        width, height = viewport['width'], viewport['height']
        scale = Fraction(viewport['scale']['numerator'], viewport['scale']['denominator'])
        assert viewport['background'] == [255]*4 and viewport['origin'] == {'x': '0', 'y': '0'}
        raw = inputs['rgba'].read_bytes()
        assert len(raw) == width*height*4
        # Count a conservative number of overlapping same-color draws per
        # pixel. Glyph plus underline/strike composition can round more than
        # once; one universal color tolerance would hide or misclassify it.
        row_draws = [[] for _ in range(height)]
        for instance in scene['instances']:
            transform = scene['transforms'][instance['transform']] if instance['transform'] is not None else {
                'parent': None, 'affine': {'linear': [str(UNIT), '0', '0', str(UNIT)],
                                         'translation': {'x': '0', 'y': '0'}}}
            assert transform['parent'] is None
            affine = transform['affine']
            assert affine['linear'] == [str(UNIT), '0', '0', str(UNIT)]
            translation = affine['translation']
            commands = scene['paths'][instance['path']]['commands']
            assert all(c['kind'] in ('move', 'line', 'close') for c in commands)
            points = [c['to'] for c in commands if 'to' in c]
            axes = [[Fraction(int(p[axis])+int(translation[axis]), UNIT)*scale for p in points]
                    for axis in ('x', 'y')]
            left, right = math.floor(min(axes[0]))-1, math.ceil(max(axes[0]))+1
            top, bottom = math.floor(min(axes[1]))-1, math.ceil(max(axes[1]))+1
            color = tuple(instance['brush']['rgba'][:3])
            for row in range(max(0, top), min(height, bottom)):
                row_draws[row].append((left, right, color))
        ink, solid = 0, {color: 0 for color in palette}
        for p in range(width*height):
            rgba = raw[p*4:p*4+4]
            assert rgba[3] == 255
            if rgba[:3] == b'\xff\xff\xff':
                continue
            ink += 1
            col = p % width
            candidates = []
            # Each owned cell uses one opaque foreground. AA coverage blends it
            # with white. Intersect coverage intervals using at most one byte
            # of rounding per overlapping draw, separately for each channel.
            layers = {}
            for left, right, color in row_draws[p // width]:
                if left <= col < right:
                    layers[color] = layers.get(color, 0)+1
            for color, domains in palette.items():
                error = layers.get(color, 0)
                lower, upper = 0, 1
                for j in range(3):
                    assert color[j] != 255
                    lower = max(lower, (255-rgba[j]-error)/(255-color[j]))
                    upper = min(upper, (255-rgba[j]+error)/(255-color[j]))
                if error and upper > 0 and lower <= upper:
                    candidates.extend(domains)
                if tuple(rgba[:3]) == color:
                    solid[color] += 1
            assert candidates, (path.name, p, list(rgba), 'unexpected pigment')
            assert any(d is None or (col+1 > d[0]*scale and col < d[1]*scale) for d in candidates), (path.name, col, 'outside cell clip')
        assert ink > 0 and all(n > 0 for n in solid.values())
        results.append({'name': path.stem, 'source': receipt(path),
                        'inputs': [receipt(p) for p in inputs.values()], 'frames': len(frames),
                        'physicalCells': len(cells), 'sourceParagraphs': flat, 'rightToLeft': rtl,
                        'clippedFrames': clipped, 'glyphs': glyphs, 'decorations': decorations,
                        'inkPixels': ink, 'solidColorPixels': {str(k): v for k, v in solid.items()}})
    assert len(results) == 4
    report = {'scope': 'private shared native table text layer; no full page or application acceptance',
              'status': 'passed', 'xsdValidations': len(results), 'cases': results}
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'status': 'passed', 'cases': len(results), 'frames': sum(r['frames'] for r in results),
                      'inkPixels': sum(r['inkPixels'] for r in results)}))


if __name__ == '__main__':
    main()
