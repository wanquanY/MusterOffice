"""Check owned PPTX line-end placement from native XML and independent font metrics.

This checks arithmetic and retained glyphs, not Office/WPS visual equivalence.
Input is the MO_HANGING_EVIDENCE_DIR from the real source_hanging test binary.
"""
import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
from xml.etree import ElementTree as ET
from zipfile import ZipFile
from fontTools.ttLib import TTFont

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('directory', type=Path)
args = p.parse_args()
A = '{http://schemas.openxmlformats.org/drawingml/2006/main}'
P = '{http://schemas.openxmlformats.org/presentationml/2006/main}'
font_path = Path('fixtures/fonts/owned-hanging.ttf')
font = TTFont(font_path)
cmap = font.getBestCmap()
unit = 1 << 32
cases = []
for source in sorted(args.directory.glob('*.pptx')):
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    assert digest == source.stem
    result = json.loads(source.with_suffix('.json').read_text())
    with ZipFile(source) as z:
        root = ET.fromstring(z.read('ppt/slides/slide1.xml'))
    shape = root.find(f'{P}cSld/{P}spTree/{P}sp')
    paragraph = shape.find(f'{P}txBody/{A}p')
    attrs = paragraph.find(f'{A}pPr').attrib
    text = ''.join(t.text or '' for t in paragraph.iter(f'{A}t'))
    size = int(paragraph.find(f'{A}r/{A}rPr').get('sz')) * 127
    advance = [Fraction(font['hmtx'][cmap[c]][0] * size, font['head'].unitsPerEm) for c in map(ord, text)]
    width = int(shape.find(f'{P}spPr/{A}xfrm/{A}ext').get('cx'))
    body = sum(advance[:-1])
    full = sum(advance)
    rtl = attrs.get('rtl') == '1'
    enabled = attrs.get('hangingPunct', '1') == '1'
    hangs = enabled and body <= width < full
    lo, hi = (advance[-1], full) if rtl else (Fraction(0), body)
    align_lo, align_hi = (lo, hi) if hangs else (Fraction(0), full)
    alignment = attrs['algn']
    offset = (-align_lo if hangs else 0) if alignment == 'l' else (
        width-align_hi if alignment == 'r' else (width-align_lo-align_hi)/2)
    q32 = lambda value: str(int(value * unit))
    actual = result['paragraphs'][0]
    decisions = actual['computed']['geometry']['paths']['layout']['decisions']
    assert len(decisions) == 1
    decision = decisions[0]
    assert decision['overflows'] == (full > width and not hangs)
    assert decision['end']['scalarOffset'] == len(text)
    if hangs:
        h = decision['hanging']
        assert (h['start']['scalarOffset'], h['end']['scalarOffset']) == (len(text)-1, len(text))
        assert (h['bodyPenMin'], h['bodyPenMax']) == (q32(lo), q32(hi))
    else:
        assert 'hanging' not in decision
    assert actual['lineOffsets'][0]['x'] == q32(offset)
    precise = actual['computed']['geometry']['precise']['lines'][0]
    assert precise['advance'] == q32(full)
    assert len(result['glyphs']) == len(text)
    cases.append(dict(sourceSha256=digest, sourceCharacters=len(text), hanging=hangs,
                      alignment=alignment, rtl=rtl, offsetQ32=q32(offset)))
assert len(cases) == 14 and sum(c['hanging'] for c in cases) == 12
report = dict(format='musteroffice.hanging-punctuation-reference/1',
              fontSha256=hashlib.sha256(font_path.read_bytes()).hexdigest(), cases=cases,
              scope='Owned source XML and independent font-metric placement arithmetic; not target-application certification')
(args.directory.parent / 'reference.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(cases=len(cases), hanging=sum(c['hanging'] for c in cases))))
