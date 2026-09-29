"""Independent native XML, table receiver and pixel checks for full page output.

This is a structural/mathematical oracle for owned fixtures, not an Office/WPS
acceptance result. Border conflict selection and automatic row fitting are open.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree

NS = {'a': 'http://schemas.openxmlformats.org/drawingml/2006/main',
      'p': 'http://schemas.openxmlformats.org/presentationml/2006/main'}
UNIT = 1 << 32


def receipt(path):
    data = path.read_bytes()
    return {'path': str(path), 'sha256': hashlib.sha256(data).hexdigest(), 'byteLength': len(data)}


def bounds(x, y, w, h):
    return {'min': {'x': str(x*UNIT), 'y': str(y*UNIT)},
            'max': {'x': str((x+w)*UNIT), 'y': str((y+h)*UNIT)}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus-dir', type=Path, required=True)
    parser.add_argument('--schema-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--extended-retention', action='store_true', help='Require the additional circle-motion and hidden-table-reveal cases')
    args = parser.parse_args()
    assert not args.output.exists()
    schema = etree.XMLSchema(etree.parse(str(args.schema_dir/'dml-main.xsd')))
    presentation = etree.XMLSchema(etree.parse(str(args.schema_dir/'pml.xsd')))
    cases = []
    for path in sorted(args.corpus_dir.glob('*.pptx')):
        with zipfile.ZipFile(path) as package:
            root = etree.fromstring(package.read('ppt/slides/slide1.xml'))
        presentation.assertValid(root)
        ordinals = {node: n for n, node in enumerate(root.iter())}
        objects = root.findall('p:cSld/p:spTree/p:graphicFrame', NS)
        assert len(root.findall('p:cSld/p:spTree/p:sp', NS)) == 0
        assert len(objects) in (1, 2)
        for obj in objects:
            schema.assertValid(obj.find('a:graphic/a:graphicData/a:tbl', NS))
        if path.name.startswith('playback-'):
            assert root.find('p:timing', NS) is not None
            if path.stem == 'playback-circle':
                assert args.extended_retention
                assert root.find('.//a:gradFill/a:path[@path="circle"]', NS) is not None
                assert root.find('.//p:animMotion', NS) is not None
            if path.stem == 'playback-reveal':
                assert args.extended_retention
                assert objects[0].find('p:nvGraphicFramePr/p:cNvPr', NS).get('hidden') == '1'
                assert root.find('.//p:set/p:to/p:strVal[@val="visible"]', NS) is not None
            cases.append({'source': receipt(path), 'nativeTables': len(objects), 'timing': True})
            continue
        plan_path = path.with_suffix('.plan.json')
        plan = json.loads(plan_path.read_bytes())
        request = json.loads(path.with_suffix('.request.json').read_bytes())
        pixels = path.with_suffix('.rgba').read_bytes()
        assert len(pixels) == request['viewport']['width']*request['viewport']['height']*4
        expected_cells = []
        for obj in objects:
            native_id = int(obj.find('p:nvGraphicFramePr/p:cNvPr', NS).get('id'))
            table = obj.find('a:graphic/a:graphicData/a:tbl', NS)
            widths = [int(c.get('w')) for c in table.findall('a:tblGrid/a:gridCol', NS)]
            rows = table.findall('a:tr', NS)
            heights = [int(r.get('h')) for r in rows]
            rtl = table.find('a:tblPr', NS).get('rtl') in ('1', 'true')
            bindings = [b for b in plan['page']['bindings'] if b['location']['object'] == native_id]
            cells = [b for b in bindings if b['fill']['target']['kind'] == 'tableCell']
            borders = [b for b in bindings if b.get('tableStroke') is not None]
            assert len(cells) == 6 and len(borders) == 20
            for row_index, row in enumerate(rows):
                for column, cell in enumerate(row.findall('a:tc', NS)):
                    if cell.get('hMerge') in ('1', 'true') or cell.get('vMerge') in ('1', 'true'):
                        continue
                    row_span, col_span = int(cell.get('rowSpan', 1)), int(cell.get('gridSpan', 1))
                    x = sum(widths[column+col_span:]) if rtl else sum(widths[:column])
                    y = sum(heights[:row_index])
                    rect = bounds(x, y, sum(widths[column:column+col_span]), sum(heights[row_index:row_index+row_span]))
                    address = {'row': row_index, 'column': column}
                    binding = next(b for b in cells if b['fill']['target']['cell'] == address)
                    assert binding['region'] == {'bounds': rect, 'coordinateErrorBound': '0'}
                    frame = next(t for t in plan['texts'] if t['frame']['text']['object']['nativeId'] == native_id and t['frame']['text']['cell'] == address)
                    assert frame['frame']['region']['outer'] == rect
                    expected_cells.append((native_id, row_index, column))
                    binding_id = plan['page']['bindings'].index(binding)
                    sources = [p for p in plan['page']['paintSources'] if p['binding'] == binding_id]
                    assert len(sources) == 1
                    assert sources[0]['path'] == {'kind': 'document', 'sourceOrdinal': ordinals[cell]}
                    assert sources[0]['fillTarget'] == binding['fill']['target']
        assert len(plan['texts']) == len(expected_cells)
        checks = 0
        if path.stem.startswith('linear-'):
            left = 0 if path.stem.endswith('-true') else 288
            # Independent XML dimensions and endpoint rule. No compiled gradient
            # coordinates or generated expected image enters this reference.
            for x in range(3, 141):
                t = (x+.5)/144
                expected = (255*(1-t**1.875), 0, 255*(1-(1-t)**1.875), 255)
                pos = (10*request['viewport']['width']+left+x)*4
                assert all(abs(a-b) <= 1 for a, b in zip(pixels[pos:pos+4], expected)), (path, x)
                checks += 1
        cases.append({'source': receipt(path), 'plan': receipt(plan_path),
                      'pixels': receipt(path.with_suffix('.rgba')), 'nativeTables': len(objects),
                      'cellFrames': len(expected_cells), 'independentGradientPixels': checks})
    expected_playback = {'playback-motion', 'playback-fade', 'playback-hidden'}
    if args.extended_retention:
        expected_playback |= {'playback-circle', 'playback-reveal'}
    assert {Path(c['source']['path']).stem for c in cases if c.get('timing')} == expected_playback
    assert len(cases) == 7 + len(expected_playback)
    report = {'profile': 'musteroffice.native-table-page-readback/1', 'status': 'passed',
              'xsdSlideValidations': len(cases), 'cases': cases, 'officeAccepted': False}
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'status': 'passed', 'cases': len(cases),
                      'gradientPixels': sum(c.get('independentGradientPixels', 0) for c in cases)}))


if __name__ == '__main__':
    main()
