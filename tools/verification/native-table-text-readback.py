"""Independently bind Native table text computation evidence to owned PPTX XML.

Checks physical cells, flat paragraph positions and actual style ordinals. This
is structural evidence; it does not assert Office/WPS appearance or table layout.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile
from lxml import etree

A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
NS = {'a': A, 'p': P}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus-dir', type=Path, required=True)
    parser.add_argument('--schema-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert not args.output.exists(), 'do not overwrite verification evidence'
    pptx = args.corpus_dir / 'native-cell-text.pptx'
    computations = args.corpus_dir / 'native-cell-text.json'
    records = json.loads(computations.read_bytes())
    source_sha = sha(pptx)
    xsd = etree.XMLSchema(etree.parse(str(args.schema_dir / 'dml-main.xsd')))
    with zipfile.ZipFile(pptx) as package:
        root = etree.fromstring(package.read('ppt/slides/slide1.xml'))
    positions = {node: n for n, node in enumerate(root.iter())}
    frame = root.find('.//p:graphicFrame', NS)
    object_id = int(frame.find('p:nvGraphicFramePr/p:cNvPr', NS).get('id'))
    owner = {'part': '/ppt/slides/slide1.xml', 'nativeId': object_id}
    table = frame.find('.//a:tbl', NS)
    xsd.assertValid(table)
    font = table.find('a:tblPr/a:tableStyle/a:wholeTbl/a:tcTxStyle/a:font', NS)
    assert font is not None
    typeface = font.find('a:latin', NS).get('typeface')
    expected_font = {
        'element': 'font',
        'origin': {'kind': 'tableStyle', 'source': {'kind': 'inline', 'object': owner},
                   'region': 'wholeTbl', 'sourceOrdinal': positions[font]},
    }
    flat, record_at, run_count = 0, 0, 0
    native_reference = None
    paths_reference = None
    for row, row_node in enumerate(table.findall('a:tr', NS)):
        for column, cell in enumerate(row_node.findall('a:tc', NS)):
            paragraphs = cell.findall('a:txBody/a:p', NS)
            for local, paragraph in enumerate(paragraphs):
                record = records[record_at]
                record_at += 1
                source = record['source']
                assert record['cell'] == source['cell'] == {'row': row, 'column': column}
                assert source['object'] == owner
                assert source['sourceSha256'] == source_sha
                assert source.get('paragraphStart', 0) == flat
                assert len(source['paragraphs']) == len(paragraphs)
                native = source['paragraphs'][local]
                assert native['sourceOrdinal'] == positions[paragraph]
                runs = paragraph.findall('a:r', NS)
                assert len(native['runs']) == len(runs)
                for n, run in enumerate(runs):
                    actual = native['runs'][n]
                    assert actual['sourceOrdinal'] == positions[run]
                    assert actual['run'] == n
                    assert actual['style']['declarations']['latin'] == expected_font
                    assert run.find('a:t', NS).text == 'A A'
                    run_count += 1
                assert native['endStyle']['declarations']['latin'] == expected_font
                for key in ['result', 'paths']:
                    result = record[key]
                    assert result['object'] == owner
                    assert result['sourceSha256'] == source_sha
                    assert result['paragraph'] == flat + local
                    assert result['sourceOrdinal'] == positions[paragraph]
                # These inputs are deliberately identical in the owned fixture;
                # duplicated XML content must keep distinct physical identities.
                if runs:
                    result, paths = record['result']['computation'], record['paths']['computation']
                    if native_reference is None:
                        native_reference, paths_reference = result, paths
                    else:
                        assert result == native_reference and paths == paths_reference
            flat += len(paragraphs)
    assert flat == record_at == len(records) == 10
    assert run_count == 9 and typeface == 'MusterOffice Synthetic'
    missing = json.loads((args.corpus_dir / 'missing-font.json').read_bytes())
    assert missing['paragraph'] == 6 and missing['object'] == owner
    assert all(u['font']['declaredBy'] == expected_font for u in missing['uses'])
    report = dict(scope='physical native table text scope and declaration provenance; not page rendering or external application acceptance',
                  pptxSha256=source_sha, computationsSha256=sha(computations),
                  physicalCells=9, paragraphs=flat, textRuns=run_count,
                  nativeShapingComparisons=8, nativePathComparisons=8,
                  xsdValidations=1, missingFontParagraph=missing['paragraph'])
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
