"""Check source numeric display against Python Decimal/Fraction, not Rust formatting.

Usage: python3 chart-number-display-oracle.py cli cases.json new-report.json
Only the explicitly enumerated formats below are mathematical-oracle coverage.
Unsupported codes and application differences are separate evidence.
"""
import copy
from decimal import Decimal, localcontext, ROUND_HALF_UP
from fractions import Fraction
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

spec = importlib.util.spec_from_file_location('labels', Path(__file__).with_name('chart-label-oracle.py'))
labels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(labels)

def reference(value, code, symbols):
    scientific = code == '0.00E+00'
    scaled = code == '0.0,,"M"'
    fixed = re.fullmatch(r'(#,##)?0(?:\.(0+))?(%)?', code)
    general = code == 'General'
    if not (scientific or scaled or fixed or general):
        return None
    with localcontext() as ctx:
        ctx.prec = 10000
        ctx.rounding = ROUND_HALF_UP
        ctx.Emax = 20000
        ctx.Emin = -20000
        v = Decimal(value.numerator) / Decimal(value.denominator)
        factor = Fraction(100 if fixed and fixed[3] else 1, 1000000 if scaled else 1)
        v *= Decimal(factor.numerator) / Decimal(factor.denominator)
        if scientific:
            shown = format(v, '.2E')
            mantissa, exponent = shown.split('E')
            sign = '+' if int(exponent) >= 0 else '-'
            shown = mantissa + 'E' + sign + str(abs(int(exponent))).zfill(2)
            if not v: shown = '0.00E+00'
            numeric = Fraction(Decimal(shown))
        elif general:
            if value.denominator != 1:
                d = value.denominator
                for prime in (2, 5):
                    while d % prime == 0: d //= prime
                if d != 1: return None
            shown = format(v, 'f')
            if '.' in shown: shown = shown.rstrip('0').rstrip('.')
            if len(shown.lstrip('-')) > 11: return None
            if not v: shown = '0'
            numeric = Fraction(Decimal(shown))
        else:
            places = 1 if scaled else len(fixed[2] or '')
            rounded = v.quantize(Decimal(1).scaleb(-places))
            if not rounded: rounded = abs(rounded)
            numeric = Fraction(rounded)
            shown = format(rounded, (',' if fixed and fixed[1] else '') + f'.{places}f')
        # The oracle uses no host locale. Translate only numeric punctuation;
        # suffix literals are appended after replacing placeholders.
        shown = shown.replace(',', '\x00').replace('.', symbols['decimalSeparator']).replace('\x00', symbols['groupSeparator'])
        if scaled: shown += 'M'
        if fixed and fixed[3]: shown += '%'
        return shown, numeric / factor != value

def check(source, request, response):
    labels.check(source, request, response)
    plan = response['labels']
    checked = 0
    for label in plan['labels']:
        displays = label['formattedComponents']
        assert len(displays) == len(label['components'])
        for component, display in zip(label['components'], displays, strict=True):
            if component['kind'] == 'text':
                assert display == dict(kind='text', value=component['value'])
                continue
            fmt = component['format']
            if fmt is None:
                assert display == dict(kind='missingFormat')
                continue
            if component['kind'] == 'number':
                value = Fraction(Decimal(component['value']))
            else:
                ratios = plan['normalizations'][component['normalization']]['ratios']
                n = next(n['numerator'] for n in ratios['numerators'] if n['pointIndex'] == component['pointIndex'])
                value = Fraction(int(n), int(ratios['denominator']))
            expected = reference(value, fmt['code'], request['numberSymbols'])
            if expected is None: continue
            assert display['kind'] == 'number'
            number = display['display']
            assert number['section'] == 0 and number['color'] is None
            assert all(f['kind'] == 'text' for f in number['fragments'])
            assert ''.join(f['value'] for f in number['fragments']) == expected[0], (component, display, expected)
            assert number['rounded'] == expected[1]
            checked += 1
    return checked

def main():
    cli, cases, output = sys.argv[1:]
    records = []; mutations = 0
    for case in json.loads(Path(cases).read_text()):
        source = Path(case['source']).read_bytes()
        request = json.loads(Path(case['request']).read_text())
        response = json.loads(subprocess.check_output([cli, 'pptx-chart-labels', case['request'], case['source']]))
        assert response['status'] == 'planned', (case['name'], response)
        count = check(source, request, response)
        if count:
            bad = copy.deepcopy(response)
            for label in bad['labels']['labels']:
                for component, display in zip(label['components'], label['formattedComponents'], strict=True):
                    if display['kind'] == 'number' and component['format'] and component['format']['code'] in {'General','0','0.00','#,##0.00','0.00%','0.00000000%','0%','0.0,,"M"','0.00E+00'}:
                        display['display']['fragments'] = [dict(kind='text', value='tampered')]
            try: check(source, request, bad)
            except AssertionError: mutations += 1
            else: raise AssertionError('altered numeric display admitted')
        records.append(dict(name=case['name'], numbersChecked=count))
    report = dict(profile='chart-numeric-independent-decimal-fraction/1', cases=records,
                  numbersChecked=sum(r['numbersChecked'] for r in records), rejectedMutations=mutations,
                  checkedScope='Enumerated fixed, percent, scale, scientific and exact-small General formats; source binding also checked against raw XML',
                  officeWpsProven=False, layoutOrPixelsProven=False)
    with Path(output).open('x') as f: json.dump(report, f, indent=2); f.write('\n')
    print(json.dumps({k: report[k] for k in ['numbersChecked','rejectedMutations']}))

if __name__ == '__main__': main()
