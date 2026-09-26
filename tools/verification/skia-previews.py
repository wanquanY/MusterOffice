"""Encode actual raster bytes as diagnostic PNGs; no image renderer dependency."""
import hashlib
import argparse
import json
from pathlib import Path
import struct
import zlib

parser = argparse.ArgumentParser()
parser.add_argument('--directory', type=Path, default=Path('.codex-work/skia/verification'))
parser.add_argument('--name-prefix', default='paragraph-')
args = parser.parse_args()
root = args.directory
report = json.loads((root / 'parity.json').read_text())
selected = {args.name_prefix + n for n in ['latin', 'bidi-brackets', 'devanagari',
            'cjk-punctuation', 'emoji', 'variable-composite', 'cubic']}


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))


artifacts = []
for case in report['cases']:
    if case['name'] not in selected:
        continue
    request = Path(case['requestPath']).read_bytes()
    if 'pixelsPath' in case:
        viewport = json.loads(request)['viewport']
        width, height = viewport['width'], viewport['height']
        data = Path(case['pixelsPath']).read_bytes()
        digest = case['pixelsSha256']
    else:
        width, height = struct.unpack_from('<II', request, 8)
        data = (root / (case['name'] + '.rgba')).read_bytes()
        digest = case['nativeSha256']
    assert hashlib.sha256(data).hexdigest() == digest
    assert len(data) == width * height * 4
    rgba = bytearray(data)
    for i in range(0, len(rgba), 4):
        alpha = rgba[i + 3]
        for k in range(3):
            rgba[i + k] = min(255, (rgba[i + k] * 255 + alpha // 2) // alpha) if alpha else 0
    rows = b''.join(b'\0' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
    png += chunk(b'sRGB', b'\0') + chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b'')
    path = root / (case['name'] + '.png')
    path.write_bytes(png)
    artifacts.append({'path': str(path), 'sha256': hashlib.sha256(png).hexdigest(),
                      'byteLength': len(png), 'pixelsSha256': digest})
assert len(artifacts) == len(selected)
(root / 'previews.json').write_text(json.dumps(artifacts, indent=2) + '\n')
print(json.dumps({'actualRasterPreviews': len(artifacts)}))
