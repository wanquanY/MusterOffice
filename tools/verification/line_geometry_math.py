"""Independent Fraction geometry and L2 reference shared by verification hosts."""
from fractions import Fraction as F
UNIT = 1 << 32

def baseline(value):
    return F(int(value['q32']), UNIT) if isinstance(value, dict) else F(value)

def nearest(x):
    sign = -1 if x < 0 else 1
    x = abs(x)
    return sign * ((2 * x.numerator + x.denominator) // (2 * x.denominator))


def quantize(x):
    return F(nearest(x * UNIT), UNIT)


def l2(levels, start):
    indices = [i for i, v in enumerate(levels) if v is not None]
    odd = [v for v in levels if v is not None and v % 2]
    if odd:
        for depth in range(max(v for v in levels if v is not None), min(odd) - 1, -1):
            a = 0
            while a < len(indices):
                b = a
                while b < len(indices) and levels[indices[b]] >= depth:
                    b += 1
                indices[a:b] = reversed(indices[a:b])
                a = b + 1
    return [i + start for i in indices]


def font_key(paragraph, font, variations):
    f = paragraph['fonts'][font]
    return (f['expectedSha256'], f['faceIndex'], tuple(sorted((v['tag'], v['value1616']) for v in variations)))


def reference(q, r, rounded):
    """Build coordinate expressions with Python arbitrary precision fractions."""
    quant = quantize if rounded else lambda x: x
    shaped = r['shaping']
    p = q['shaping']['paragraph']
    metrics = {font_key(p, m['font'], m['variations']): m for m in r['metricInstances']}

    def extents(style, candidate):
        m = metrics[font_key(p, candidate['font'], candidate['variations'])]
        s = q['styles'][style]
        a, d, g = [quant(F(v['position'] * int(s['fontSize']), m['positionUnitsPerEm'])) for v in m['measured']['values']]
        shift = baseline(s['baselineShift'])
        return a + shift, -d - shift, g

    strut = q['strutStyle']
    minimum = extents(strut, p['styles'][strut]['candidates'][0])
    lines = []
    for index, line in enumerate(shaped['lines']):
        fragments = {}
        scalar_owner = {}
        extent_list = [minimum]
        for i in range(line['fallbackStart'], line['fallbackEnd']):
            style = shaped['items'][shaped['shapedItemIndices'][i]]['style']
            for j, fragment in enumerate(shaped['fallback']['items'][i]['fragments']):
                assert fragment['status'] == 'selected'
                key = (i, j)
                fragments[key] = (fragment, style)
                extent_list.append(extents(style, p['styles'][style]['candidates'][fragment['candidate']]))
                for scalar in range(fragment['start'], fragment['end']):
                    assert scalar not in scalar_owner
                    scalar_owner[scalar] = key
        bidi = shaped['bidi']['lines'][index]
        order = l2(bidi['levels'], line['start']['scalarOffset'])
        assert order == bidi['visualOrder']
        visual = list(dict.fromkeys(scalar_owner[s] for s in order if s in scalar_owner))
        removed = [key for key in fragments if key not in visual]
        ascent, descent, gap = [max(v[i] for v in extent_list) for i in range(3)]
        natural = ascent + descent + gap
        spacing = q['spacing']
        if spacing['kind'] == 'styleMaximum':
            values = [F(v)/UNIT for v in spacing['heights']]
            participating = [values[item['style']] for item in shaped['items'][line['itemStart']:line['itemEnd']] if item['kind'] == 'text']
            height = max(participating) if participating else values[strut]
        else:
            height = natural if spacing['kind'] == 'natural' else F(int(spacing['height']))
        if spacing['kind'] == 'atLeast':
            height = max(height, natural)
        top = lines[-1]['bottom'] if lines else F(0)
        line_baseline = top + ascent + quant((height - ascent - descent) / 2)
        # Each fragment has an exact design-unit prefix array. Coordinates use
        # sums of preceding fragment displacements, not per-glyph wire advances.
        widths, rises, glyphs, pens = [], [], [], [F(0)]
        for key in visual:
            f, style = fragments[key]
            s = q['styles'][style]
            scale = F(int(s['fontSize']), f['shaped']['positionUnitsPerEm'])
            raw = f['shaped']['runs'][0]['glyphs']
            ref = {'fallbackItem': key[0], 'fragment': key[1]}
            prefix_x, prefix_y, tracking = [0], [0], [F(0)]
            extra = F(s.get('clusterSpacing', '0')) / UNIT
            for k, g in enumerate(raw):
                prefix_x.append(prefix_x[-1] + g['xAdvance'])
                prefix_y.append(prefix_y[-1] + g['yAdvance'])
                cluster_end = k+1 == len(raw) or raw[k+1]['cluster'] != g['cluster']
                tracking.append(tracking[-1] + (extra if cluster_end else 0))
            for k, g in enumerate(raw):
                glyphs.append({'source': ref, 'glyph': k,
                               'x': sum(widths) + quant((prefix_x[k] + g['xOffset']) * scale) + tracking[k],
                               'y': line_baseline - baseline(s['baselineShift']) - sum(rises) - quant((prefix_y[k] + g['yOffset']) * scale)})
            pens.extend(sum(widths) + quant(prefix_x[k+1] * scale) + tracking[j]
                        for k in range(len(raw)) for j in [k, k+1])
            widths.append(quant(prefix_x[-1] * scale) + tracking[-1])
            rises.append(quant(prefix_y[-1] * scale))
        lines.append({'top': top, 'baseline': line_baseline, 'bottom': top + height,
                      'height': height, 'advance': sum(widths), 'advanceY': -sum(rises),
                      'penMin': min(pens), 'penMax': max(pens),
                      'visualFragments': [{'fallbackItem': i, 'fragment': j} for i, j in visual],
                      'removedByX9': [{'fallbackItem': i, 'fragment': j} for i, j in removed], 'glyphs': glyphs})
    return {'height': sum(line['height'] for line in lines), 'lines': lines}
