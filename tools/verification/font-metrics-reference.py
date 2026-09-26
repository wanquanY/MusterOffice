"""Independent FontTools table/avar/ItemVariationStore evidence for metric fixtures.
No HarfBuzz or production code is called. Run --prepare before the JS corpus,
then without arguments to compare actual Rust Native/WASM results.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
from fontTools import version
from fontTools.ttLib import TTFont
from fontTools.varLib.mvar import MVAR_ENTRIES
from fontTools.varLib.models import normalizeLocation, piecewiseLinearMap
from fontTools.varLib.varStore import VarStoreInstancer

assert version == '4.61.1'
root = Path('.codex-work/font-metrics')
parser = argparse.ArgumentParser()
parser.add_argument('--prepare', action='store_true')
args = parser.parse_args()
if args.prepare:
    variants = []
    for name, tables, missing, zero in [
        ('no-vertical', ['vhea', 'vmtx'], ['verticalAscender', 'verticalDescender', 'verticalLineGap', 'verticalCaretRise', 'verticalCaretRun', 'verticalCaretOffset'], []),
        ('no-os2', ['OS/2'], ['xHeight', 'capHeight', 'horizontalClippingAscent', 'horizontalClippingDescent', 'subscriptXSize', 'strikeoutOffset'], []),
        ('os2-v1', [], ['xHeight', 'capHeight'], ['horizontalCaretOffset', 'verticalLineGap']),
    ]:
        f = TTFont('fixtures/fonts/owned.ttf', recalcTimestamp=False)
        for table in tables: del f[table]
        if name == 'os2-v1': f['OS/2'].version = 1
        path = root / (name + '.ttf'); f.save(path)
        variants.append({'name': name, 'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'missing': missing, 'zero': zero})
    (root / 'font-variants.json').write_text(json.dumps(variants, indent=2) + '\n')
else:
    report = json.loads((root / 'parity.json').read_text())
    results = []
    # This enumerates independently from the production enum/ABI mapping.
    names = ['horizontalAscender', 'horizontalDescender', 'horizontalLineGap', 'horizontalClippingAscent', 'horizontalClippingDescent', 'verticalAscender', 'verticalDescender', 'verticalLineGap', 'horizontalCaretRise', 'horizontalCaretRun', 'horizontalCaretOffset', 'verticalCaretRise', 'verticalCaretRun', 'verticalCaretOffset', 'xHeight', 'capHeight', 'subscriptXSize', 'subscriptYSize', 'subscriptXOffset', 'subscriptYOffset', 'superscriptXSize', 'superscriptYSize', 'superscriptXOffset', 'superscriptYOffset', 'strikeoutSize', 'strikeoutOffset', 'underlineSize', 'underlineOffset']
    tags = 'hasc hdsc hlgp hcla hcld vasc vdsc vlgp hcrs hcrn hcof vcrs vcrn vcof xhgt cpht sbxs sbys sbxo sbyo spxs spys spxo spyo strs stro unds undo'.split()
    for case in report['cases']:
        if not case['name'].startswith('mvar-'): continue
        f = TTFont(case['fontPath'])
        request = json.loads(Path(case['requestPath']).read_text())
        response = json.loads(Path(case['responsePath']).read_text())
        axes = {a.axisTag: (a.minValue, a.defaultValue, a.maxValue) for a in f['fvar'].axes}
        records = {r.ValueTag: r.VarIdx for r in f['MVAR'].table.ValueRecord}
        # First five instances are binary-exact coordinates and region weights.
        for index, instance in enumerate(request['instances'][:5]):
            loc = normalizeLocation({v['tag']: v['value1616'] / 65536 for v in instance['variations']}, axes)
            loc = {k: piecewiseLinearMap(v, f['avar'].segments[k]) for k, v in loc.items()}
            var = VarStoreInstancer(f['MVAR'].table.VarStore, f['fvar'].axes, loc)
            expected = []
            for tag in tags:
                table, attr = MVAR_ENTRIES[tag]
                if tag in ['hasc', 'hdsc', 'hlgp'] and not f['OS/2'].fsSelection & 128:
                    table = 'hhea'; attr = {'hasc': 'ascent', 'hdsc': 'descent', 'hlgp': 'lineGap'}[tag]
                value = getattr(f[table], attr) + var[records[tag]]
                if tag in ['hasc', 'vasc']: value = abs(value)
                if tag in ['hdsc', 'vdsc']: value = -abs(value)
                # C roundf is ties away from zero; binary-exact fixture values.
                scaled = value * 64
                expected.append(math.floor(scaled + 0.5) if scaled >= 0 else math.ceil(scaled - 0.5))
            actual = response['metrics']['instances'][index]['values']
            assert [v['metric'] for v in actual] == names
            assert [v['position'] for v in actual] == expected, (case['name'], index)
            results.append({'case': case['name'], 'instance': index, 'normalizedLocation': loc, 'positions': expected})
    assert len(results) == 10
    output = {'fontTools': version, 'instances': len(results), 'valuesCompared': len(results) * 28, 'tolerance': 0, 'results': results}
    (root / 'fonttools-reference.json').write_text(json.dumps(output, indent=2) + '\n')
    print(json.dumps({'instances': len(results), 'valuesCompared': len(results) * 28}))
