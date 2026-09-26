"""Owned font with explicit optional and required ligatures for tracking probes."""
import hashlib
import json
from pathlib import Path
from fontTools import version
from fontTools.ttLib import TTFont
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString

assert version == '4.61.1'
root = Path('fixtures/fonts')
font = TTFont(root / 'owned-decorations.ttf', recalcTimestamp=False)
# All outlines remain original. These synthetic ligatures exercise boundaries;
# they are not typographic or language-quality specimens.
addOpenTypeFeaturesFromString(font, '''
languagesystem DFLT dflt;
languagesystem latn dflt;
languagesystem grek dflt;
feature liga { sub A A by alpha; } liga;
feature rlig { sub alpha alpha by A; } rlig;
''')
path = root / 'owned-tracking.ttf'
font.save(path)
data = path.read_bytes()
manifest = json.loads((root / 'decoration-manifest.json').read_text())
manifest['fonts'][0].update(expectedSha256=hashlib.sha256(data).hexdigest(), byteLength=str(len(data)))
(root / 'tracking-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(dict(path=str(path), sha256=hashlib.sha256(data).hexdigest(), byteLength=len(data))))
