"""Check a real Native HarfBuzz manifest probe against owned SFNT bytes.

Only the owned unkerned Latin/space probe is covered. Parses name, cmap12,
hmtx/head tables with Python stdlib, without using Rust or HarfBuzz to obtain
expected font names, glyph indices or design advances.
"""
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path('.codex-work/text-fonts')
request_path = Path('fixtures/fonts/manifest-paragraph.json')
font_path = Path('fixtures/fonts/owned.ttf')
result_path = ROOT / 'native-manifest.json'
request = json.loads(request_path.read_text())
result = json.loads(result_path.read_text())
raw = font_path.read_bytes()
sha = lambda b: hashlib.sha256(b).hexdigest()


def entry(path):
    b = path.read_bytes()
    return dict(path=str(path), byteLength=len(b), sha256=sha(b))


assert raw[:4] == b'\0\1\0\0'
tables = {}
for i in range(struct.unpack_from('>H', raw, 4)[0]):
    tag, _, offset, length = struct.unpack_from('>4sIII', raw, 12 + 16 * i)
    assert offset + length <= len(raw) and tag not in tables
    tables[tag] = raw[offset:offset + length]
manifest = request['manifest']
assert manifest['profile'] == result['profile'] == 'explicit-font-resource-manifest-draft-v1'
assert manifest['fonts'] == [dict(expectedSha256=sha(raw), faceIndex=0, offset='0', byteLength=str(len(raw)))]
assert len(manifest['faces']) == len(manifest['typefaces']) == 1
face, typeface = manifest['faces'][0], manifest['typefaces'][0]
name = tables[b'name']
_, count, start = struct.unpack_from('>HHH', name)
name_evidence = []
for key, allowed in [('family', [1, 16, 21]), ('subfamily', [2, 17, 22]), ('postscript', [6])]:
    binding = face[key]
    assert binding['record'] < count
    platform, encoding, language, ident, length, offset = struct.unpack_from('>HHHHHH', name, 6 + binding['record'] * 12)
    b = name[start + offset:start + offset + length]
    assert len(b) == length and ident in allowed
    assert platform in [0, 1, 3]
    actual = b.decode('mac_roman' if platform == 1 else 'utf-16-be')
    assert actual == binding['expected']
    name_evidence.append(dict(kind=key, record=binding['record'], platform=platform,
                              encoding=encoding, language=language, nameId=ident, text=actual))
assert typeface['typeface'] == face['family']['expected'] == request['styles'][0]['typeface']
assert typeface['policy'] == dict(kind='exactFamily')
assert typeface['regular'] == dict(face=0, variations=[])
assert all(typeface[k] is None for k in ['bold', 'italic', 'boldItalic'])
assert result['bindings'] == [dict(style=0, typeface=0, fontStyle='regular', face=0,
                                   candidate=dict(font=0, variations=[]), policy=dict(kind='exactFamily'))]
assert request['text'] == 'A A' and request['direction'] == 'leftToRight'
assert len(request['styles']) == 1 and request['styles'][0]['features'] == []
assert request['spans'] == [dict(end=3, style=0)]
# Owned font has no substitutions, kerning or positioning tables affecting this
# probe, so cmap plus horizontal metrics gives an independent shaping reference.
assert not any(t in tables for t in [b'GSUB', b'GPOS', b'kern', b'gvar', b'HVAR'])
cmap = tables[b'cmap']
offsets = {struct.unpack_from('>I', cmap, 8 + 8 * i)[0]
           for i in range(struct.unpack_from('>H', cmap, 2)[0])}
cmaps = [cmap[o:] for o in offsets if struct.unpack_from('>H', cmap, o)[0] == 12]
assert len(cmaps) == 1
groups = [struct.unpack_from('>III', cmaps[0], 16 + i * 12)
          for i in range(struct.unpack_from('>I', cmaps[0], 12)[0])]
metrics_count = struct.unpack_from('>H', tables[b'hhea'], 34)[0]
units = struct.unpack_from('>H', tables[b'head'], 18)[0]
expected = []
for cluster, char in enumerate(request['text']):
    matches = [glyph + ord(char) - first for first, last, glyph in groups if first <= ord(char) <= last]
    assert len(matches) == 1
    glyph = matches[0]
    advance = struct.unpack_from('>H', tables[b'hmtx'], 4 * min(glyph, metrics_count - 1))[0]
    expected.append(dict(glyphId=glyph, cluster=cluster, xAdvance=advance * 64,
                         yAdvance=0, xOffset=0, yOffset=0))
fallback = result['shaping']['fallback']
assert fallback['verifiedFaces'] == fallback['shapingRuns'] == fallback['componentCalls'] == 1
assert result['shaping']['shapedItemIndices'] == [0]
assert len(fallback['items']) == len(fallback['items'][0]['fragments']) == 1
fragment = fallback['items'][0]['fragments'][0]
assert fragment['status'] == 'selected' and fragment['font'] == fragment['candidate'] == 0
shaped = fragment['shaped']
assert shaped['fontSha256'] == sha(raw) and shaped['faceIndex'] == 0
assert shaped['unitsPerEm'] == units and shaped['positionUnitsPerEm'] == 64 * units
assert shaped['profile'] == 'harfbuzz-14.5.0-ot-ucd18-design64-v1'
assert len(shaped['runs']) == 1 and shaped['runs'][0]['missingGlyphClusters'] == []
glyphs = shaped['runs'][0]['glyphs']
assert len(glyphs) == len(expected)
for glyph, want in zip(glyphs, expected):
    assert {k: glyph[k] for k in want} == want
report = dict(format='musteroffice.explicit-manifest-native-reference/1', scope=__doc__,
              inputs=[entry(request_path), entry(font_path)], result=entry(result_path),
              names=name_evidence, glyphs=expected, unitsPerEm=units,
              verifiedFaces=1, shapingRuns=1, componentCalls=1)
(ROOT / 'manifest-reference.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(dict(nameRecords=len(name_evidence), glyphs=len(expected), verifiedFaces=1, unitsPerEm=units)))
