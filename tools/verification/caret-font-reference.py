"""Independent GDEF table/gvar/ItemVariationStore oracle (FontTools 4.61.1).
Does not call HarfBuzz or the production caret implementation.
"""
from pathlib import Path
import argparse
import copy
import json
from fontTools import version
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.varLib.models import normalizeLocation
from fontTools.varLib.varStore import VarStoreInstancer

assert version == '4.61.1'
p = argparse.ArgumentParser()
p.add_argument('output', type=Path)
args = p.parse_args()
cases = []
for name in ['owned-carets.ttf', 'owned-carets-points.ttf']:
    source = TTFont(Path('fixtures/fonts') / name, recalcTimestamp=False)
    axes = {a.axisTag: (a.minValue, a.defaultValue, a.maxValue) for a in source['fvar'].axes}
    locations = [{}, {'wght': 650}, {'wght': 900}]
    if 'wdth' in axes:
        locations += [{'wdth': 125}, {'wght': 900, 'wdth': 125}]
    for loc in locations:
        effective = {tag: loc.get(tag, axis[1]) for tag, axis in axes.items()}
        f = instantiateVariableFont(copy.deepcopy(source), effective, inplace=True)
        gdef = source['GDEF'].table
        carets = dict(zip(gdef.LigCaretList.Coverage.glyphs, gdef.LigCaretList.LigGlyph))
        for direction in ['leftToRight', 'rightToLeft', 'topToBottom', 'bottomToTop']:
            positions = []
            for gid, glyph in enumerate(f.getGlyphOrder()):
                result = []
                for caret in getattr(carets.get(glyph), 'CaretValue', []):
                    if caret.Format in [1, 3]:
                        value = caret.Coordinate
                        if caret.Format == 3:
                            device = caret.DeviceTable
                            assert device.DeltaFormat == 0x8000
                            store = VarStoreInstancer(gdef.VarStore, source['fvar'].axes, normalizeLocation(loc, axes))
                            value += store[(device.StartSize << 16) | device.EndSize]
                    else:
                        assert caret.Format == 2
                        outline = f['glyf'][glyph]
                        coords = outline.getCoordinates(f['glyf'])[0]
                        x, y = coords[caret.CaretValuePoint]
                        advance, bearing = f['hmtx'].metrics[glyph]
                        x -= outline.xMin - bearing
                        value = x if direction in ['leftToRight', 'rightToLeft'] else y - (outline.yMax + f['vmtx'].metrics[glyph][1])
                    scaled = value * 64
                    assert scaled == int(scaled), (name, glyph, loc, value)
                    result.append(int(scaled))
                positions.append({'glyphId': gid, 'positions': result})
            cases.append({'name': name, 'variations': [{'tag': k, 'value1616': int(v*65536)} for k,v in loc.items()], 'direction': direction, 'glyphs': positions})
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps({'oracle': 'fonttools 4.61.1', 'cases': cases}, indent=2)+'\n')
print(json.dumps({'cases': len(cases), 'caretValues': sum(len(g['positions']) for c in cases for g in c['glyphs'])}))
