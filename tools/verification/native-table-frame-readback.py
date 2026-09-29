"""Check owned native cell frames against independent XML/grid arithmetic.

Declared row extents and unclipped glyph capacity are checked, not row autofit
or visual equality with Office/WPS. The computation files come from real HB.
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


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rectangle(values):
    left, top, right, bottom = values
    return {'min': {'x': str(left * UNIT), 'y': str(top * UNIT)},
            'max': {'x': str(right * UNIT), 'y': str(bottom * UNIT)}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus-dir', type=Path, required=True)
    parser.add_argument('--schema-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert not args.output.exists(), 'do not overwrite evidence'
    xsd = etree.XMLSchema(etree.parse(str(args.schema_dir / 'dml-main.xsd')))
    results, frames, clipped = [], 0, 0
    for path in sorted(args.corpus_dir.glob('*.pptx')):
        with zipfile.ZipFile(path) as package:
            root = etree.fromstring(package.read('ppt/slides/slide1.xml'))
        ordinals = {node: n for n, node in enumerate(root.iter())}
        obj = root.find('.//p:graphicFrame', NS)
        object_id = int(obj.find('p:nvGraphicFramePr/p:cNvPr', NS).get('id'))
        table = obj.find('.//a:tbl', NS)
        xsd.assertValid(table)
        rows = table.findall('a:tr', NS)
        widths = [int(c.get('w')) for c in table.findall('a:tblGrid/a:gridCol', NS)]
        heights = [int(r.get('h')) for r in rows]
        rtl = table.find('a:tblPr', NS).get('rtl', '0') in ['1', 'true']
        flat, cells = 0, {}
        for r, row in enumerate(rows):
            for c, cell in enumerate(row.findall('a:tc', NS)):
                cells[r, c] = cell, flat
                flat += len(cell.findall('a:txBody/a:p', NS))
        computed_path = path.with_suffix('.json')
        records = json.loads(computed_path.read_bytes())
        for record in records:
            at = record['text']['cell']
            r, c = at['row'], at['column']
            cell, paragraph_start = cells[r, c]
            assert cell.get('hMerge', '0') not in ['1', 'true']
            assert cell.get('vMerge', '0') not in ['1', 'true']
            span_r, span_c = int(cell.get('rowSpan', '1')), int(cell.get('gridSpan', '1'))
            left = sum(widths[c+span_c:]) if rtl else sum(widths[:c])
            top = sum(heights[:r])
            outer = [left, top, left+sum(widths[c:c+span_c]), top+sum(heights[r:r+span_r])]
            inner = outer.copy()
            props = cell.find('a:tcPr', NS)
            attrs = record['body']['attributes']
            for n, (xml_name, name, default) in enumerate([
                ('marL', 'leftInset', 91440), ('marT', 'topInset', 45720),
                ('marR', 'rightInset', 91440), ('marB', 'bottomInset', 45720),
            ]):
                value = int(props.get(xml_name, str(default)))
                assert int(attrs[name]) == value
                inner[n] += value * (1 if n < 2 else -1)
            assert record['region']['outer'] == rectangle(outer)
            assert record['region']['inner'] == rectangle(inner)
            assert record['region']['source'] == {
                'kind': 'tableCell', 'cell': at, 'sourceOrdinal': ordinals[cell],
                'region': {'origin': at, 'rows': span_r, 'columns': span_c}}
            assert record['text'].get('paragraphStart', 0) == paragraph_start
            assert record['text']['sourceSha256'] == sha(path)
            assert record['text']['object'] == {'part': '/ppt/slides/slide1.xml', 'nativeId': object_id}
            paragraphs = cell.findall('a:txBody/a:p', NS)
            assert [p['sourceOrdinal'] for p in record['text']['paragraphs']] == [ordinals[p] for p in paragraphs]
            assert attrs['anchor'] == props.get('anchor', 't')
            horizontal = props.get('horzOverflow', 'clip') == 'clip'
            if horizontal:
                assert record['clip']['horizontal'] and not record['clip']['vertical']
                assert record['clip']['bounds'] == rectangle(outer)
                clipped += 1
            else:
                assert 'clip' not in record
            assert record['work']['glyphs'] == len(record['glyphs'])
            frames += 1
        results.append({'name': path.name, 'sourceSha256': sha(path),
                        'computationSha256': sha(computed_path), 'frames': len(records),
                        'physicalCells': len(cells), 'paragraphs': flat, 'rightToLeft': rtl})
    assert len(results) == 8 and frames == 9 and clipped == 7
    report = {'scope': 'XML/grid/body/native frame consistency, not full table rendering or application acceptance',
              'status': 'passed', 'files': results, 'frames': frames,
              'xsdValidations': len(results), 'clippedFrames': clipped}
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'status': 'passed', 'files': len(results), 'frames': frames, 'clippedFrames': clipped}))


if __name__ == '__main__':
    main()
