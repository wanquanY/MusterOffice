"""Original GDEF fixtures derived only from MusterOffice owned.ttf.
Reproduce with FontTools 4.61.1. Format 1, contour point format 2, and variable
format 3 exercise carets that cannot be inferred by equal character spacing.
"""
from pathlib import Path
import copy
import hashlib
import json
from fontTools import version
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables import otTables as ot
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.otlLib.builder import buildLigCaretList
from fontTools.varLib.builder import buildVarRegionList, buildVarData, buildVarStore

assert version == '4.61.1'
root = Path('fixtures/fonts')
font = TTFont(root / 'owned.ttf', recalcTimestamp=False)
addOpenTypeFeaturesFromString(font, '''
languagesystem DFLT dflt;
languagesystem latn dflt;
feature liga { sub A A A by A.alt; sub alpha alpha by smile; } liga;
''')
gdef = newTable('GDEF')
gdef.table = ot.GDEF()
gdef.table.Version = 0x00010003
gdef.table.GlyphClassDef = None
gdef.table.AttachList = None
gdef.table.MarkAttachClassDef = None
gdef.table.MarkGlyphSetsDef = None
variation = ot.Device()
variation.StartSize = 0
variation.EndSize = 0
variation.DeltaFormat = 0x8000
# A's zero and negative carets are deliberate ABI edge cases, not layout advice.
gdef.table.LigCaretList = buildLigCaretList(
    {'A.alt': [173, 431], 'A': [-50, 0], 'acutecomb': [(125, variation)]},
    {'smile': [1]}, font.getReverseGlyphMap())
gdef.table.VarStore = buildVarStore(
    buildVarRegionList([{'wght': (0, 1, 1)}, {'wdth': (0, 1, 1)}], ['wght', 'wdth']),
    [buildVarData([0, 1], [[80, -40]], optimize=False)])
font['GDEF'] = gdef
records = []
for name, oversized in [('owned-carets.ttf', False), ('owned-carets-limit.ttf', True)]:
    if oversized:
        font['GDEF'].table.LigCaretList = buildLigCaretList(
            {'A.alt': list(range(65))}, {}, font.getReverseGlyphMap())
    font.save(root / name)
    data = (root / name).read_bytes()
    records.append({'name': name, 'sha256': hashlib.sha256(data).hexdigest(), 'byteLength': len(data)})
# Point fixtures include off-curve points, composite transforms and gvar deltas.
for name, source, points in [
    ('owned-carets-points.ttf', 'owned-outlines.ttf', {'curves': list(range(8)), 'component': list(range(16))}),
    ('owned-carets-invalid-point.ttf', 'owned.ttf', {'smile': [999]}),
    ('owned-carets-cff-point.otf', 'owned-outlines.otf', {'curves': [1]}),
]:
    point_font = TTFont(root / source, recalcTimestamp=False)
    point_gdef = copy.deepcopy(gdef)
    point_gdef.table.Version = 0x00010000
    point_gdef.table.VarStore = None
    point_gdef.table.LigCaretList = buildLigCaretList({}, points, point_font.getReverseGlyphMap())
    point_font['GDEF'] = point_gdef
    if source == 'owned-outlines.ttf':
        original = TTFont(root / 'owned.ttf', recalcTimestamp=False)
        point_font['vhea'] = copy.deepcopy(original['vhea'])
        point_font['vmtx'] = newTable('vmtx')
        point_font['vmtx'].metrics = {g: (1500, 100) for g in point_font.getGlyphOrder()}
    point_font.save(root / name)
    data = (root / name).read_bytes()
    records.append({'name': name, 'sha256': hashlib.sha256(data).hexdigest(), 'byteLength': len(data)})
(root / 'owned-carets.json').write_text(json.dumps({
    'generator': 'fonttools 4.61.1', 'source': 'owned.ttf', 'files': records,
    'designUnitCoordinates': {'A.alt': [173, 431], 'A': [-50, 0],
        'acutecomb': {'default': [125], 'wght900': [205], 'wdth125': [85], 'wght900wdth125': [165]},
        'smile': {'contourPoint': 1}},
    'limitFixture': {'glyph': 'A.alt', 'caretCount': 65}
}, indent=2) + '\n')
print(json.dumps(records))
