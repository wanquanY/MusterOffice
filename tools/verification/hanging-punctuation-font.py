"""Extend our synthetic test font with original glyph aliases for line-end tests.

No system fonts, third-party outlines or production assets are used. Existing
owned.ttf and its evidence stay unchanged. Run with FontTools 4.61.1.
"""
from pathlib import Path
import hashlib
import json
import fontTools
from fontTools.ttLib import TTFont

assert fontTools.__version__ == '4.61.1'
root = Path('fixtures/fonts')
source = root / 'owned.ttf'
font = TTFont(source, recalcTimestamp=False)
for table in font['cmap'].tables:
    if table.format in (4, 12):
        for char in '。，？！）אב中!':
            table.cmap[ord(char)] = 'A'
target = root / 'owned-hanging.ttf'
font.save(target)
manifest = json.loads((root / 'manifest-paragraph.json').read_text())
data = target.read_bytes()
manifest['manifest']['fonts'][0].update(
    expectedSha256=hashlib.sha256(data).hexdigest(), byteLength=str(len(data)))
(root / 'manifest-hanging.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(dict(font=str(target), sha256=hashlib.sha256(data).hexdigest(),
                     byteLength=len(data), sourceSha256=hashlib.sha256(source.read_bytes()).hexdigest())))
