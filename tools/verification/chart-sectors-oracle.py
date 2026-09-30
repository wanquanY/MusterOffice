"""Independent rational endpoint oracle for chart-sectors-parity.mjs results.

Usage: python3 chart-sectors-oracle.py CASES_JSON PARITY_REPORT_JSON
Only reads selected inputs/results. Uses Python Fraction, not kernel arithmetic.
"""
from fractions import Fraction
from pathlib import Path
import hashlib
import json
import sys

cases_path, report_path = map(Path, sys.argv[1:])
cases = json.loads(cases_path.read_text())
report = json.loads(report_path.read_text())
assert len(cases) == len(report["cases"])
points = 0
computed = 0
for case, result in zip(cases, report["cases"]):
    assert case["name"] == result["name"]
    raw = Path(case["request"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == result["requestSha256"]
    response = result["response"]
    if response["status"] != "computed":
        assert case["expectedStatus"] == "error"
        continue
    request = json.loads(raw)
    source = request["weights"]
    values = [Fraction(point["value"]) for point in source]
    if request["negativeWeights"] == "absoluteMagnitude":
        values = list(map(abs, values))
    else:
        assert all(value >= 0 for value in values)
    total = sum(values)
    layout = response["layout"]
    assert layout["zeroTotal"] == (total == 0)
    assert int(layout["endpointErrorBound"]) == (0 if total == 0 else 1)
    assert len(layout["sectors"]) == len(source)
    assert layout["work"]["points"] == len(source)
    start = int(request["startTurn"])
    direction = 1 if request["direction"] == "clockwise" else -1
    prefix = Fraction(0)
    prior = start
    for point, value, sector in zip(source, values, layout["sectors"]):
        prefix += value
        exact = prefix * (1 << 32) / total if total else Fraction(0)
        rounded = (exact + Fraction(1, 2)).numerator // (exact + Fraction(1, 2)).denominator
        expected = start + direction * rounded
        assert int(sector["startTurn"]) == prior
        assert int(sector["endTurn"]) == expected
        assert sector["pointIndex"] == point["pointIndex"]
        assert sector["zeroWeight"] == (value == 0)
        assert sector["positiveBelowResolution"] == (value > 0 and prior == expected)
        prior = expected
        points += 1
    if total:
        assert prior == start + direction * (1 << 32)
    computed += 1
print(json.dumps(dict(computedCases=computed, checkedPoints=points, exactRationalOracle=True)))
