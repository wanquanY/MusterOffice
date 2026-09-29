"""Extend owned synthetic glyphs for deterministic native stress execution.

No third-party outlines, metadata or system fonts. These geometric marks verify
code-point coverage/computation only; they are not typography quality specimens.
"""
import argparse
import hashlib
import json
from pathlib import Path
from fontTools import version
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert version == '4.61.1'
    base = (args.source / 'font.bin').read_bytes()
    assert hashlib.sha256(base).hexdigest() == '5e182a1675e0255bce1ff90e6a00d1fee0c12acee868987af1b4f39bf27cabc9'
    args.output.mkdir(exist_ok=False)
    font = TTFont(args.source / 'font.bin', recalcTimestamp=False)
    order = list(font.getGlyphOrder())
    cmap = font.getBestCmap()
    added = []
    for code in sorted(set(map(ord, 'Wi演示文稿مرحباA\u0301 ')) - cmap.keys()):
        name = f'owned{code:04X}'
        width = 900 if code == ord('W') else 220 if code == ord('i') else 1000 if code >= 0x4e00 else 650
        pen = TTGlyphPen(None)
        pen.moveTo((30, 0))
        pen.lineTo((width // 2, 700))
        pen.lineTo((width - 30, 0))
        pen.closePath()
        font['glyf'][name] = pen.glyph()
        font['hmtx'][name] = (width, 30)
        font['vmtx'][name] = (1000, 100)
        order.append(name)
        for table in font['cmap'].tables:
            if table.isUnicode() and table.format in (4, 12):
                table.cmap[code] = name
        added.append(code)
    font.setGlyphOrder(order)
    font['head'].modified = font['head'].created = 2082844800
    font.save(args.output / 'font.bin')
    data = (args.output / 'font.bin').read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    request = json.loads((args.source / 'request.json').read_text())
    descriptor = request['action']['settings']['delivery']['fonts']['fonts'][0]
    descriptor.update(byteLength=str(len(data)), expectedSha256=digest)
    (args.output / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
    receipt = dict(generator='FontTools 4.61.1', parentSha256=hashlib.sha256(base).hexdigest(),
                   sha256=digest, byteLength=len(data), addedCodePoints=added,
                   provenance='Original MusterOffice synthetic triangles and metadata; no external font material',
                   scope='Computation fixture only; not language shaping or typography acceptance')
    (args.output / 'provenance.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
