"""Original decoration font: preserve the owned outlines/avar/MVAR, add base metrics."""
import hashlib
import json
from pathlib import Path
from fontTools import version
from fontTools.ttLib import TTFont

assert version == '4.61.1'
root = Path('fixtures/fonts')
font = TTFont(root / 'owned-metrics-hhea.ttf', recalcTimestamp=False)
font['post'].underlinePosition = -120
font['post'].underlineThickness = 60
font['OS/2'].yStrikeoutPosition = 300
font['OS/2'].yStrikeoutSize = 70
# Original triangle/alpha outlines also exercise bidi placement. These are
# synthetic codepoint mappings, not Hebrew glyph designs or quality samples.
for cmap in font['cmap'].tables:
    if cmap.isUnicode():
        cmap.cmap.update({0x05D0: 'A', 0x05D1: 'alpha'})
path = root / 'owned-decorations.ttf'
font.save(path)
data = path.read_bytes()
manifest = json.loads((root / 'manifest-paragraph.json').read_text())['manifest']
manifest['fonts'][0].update(expectedSha256=hashlib.sha256(data).hexdigest(), byteLength=str(len(data)))
manifest['typefaces'][0]['bold'] = dict(face=0, variations=[dict(tag='wght', value1616=700 * 65536)])
(root / 'decoration-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(dict(path=str(path), sha256=hashlib.sha256(data).hexdigest(), byteLength=len(data))))
