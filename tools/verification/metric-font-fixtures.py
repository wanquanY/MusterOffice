"""Owned MVAR/avar fixtures, reproducible with FontTools 4.61.1.
Only the project's original owned.ttf is used; no third-party outlines.
"""
import hashlib
import json
from pathlib import Path
from fontTools import version
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables import otTables as ot
from fontTools.varLib.builder import buildVarRegionList, buildVarData, buildVarStore
from fontTools.varLib.mvar import MVAR_ENTRIES

assert version == "4.61.1"
root = Path("fixtures/fonts")
records = []
for typo in [False, True]:
    font = TTFont(root / "owned.ttf", recalcTimestamp=False)
    font["OS/2"].fsSelection = (font["OS/2"].fsSelection & ~128) | (128 if typo else 0)
    font["OS/2"].version = 4
    font["OS/2"].sxHeight = 480
    font["OS/2"].sCapHeight = 700
    # Both axes remap nonlinearly. Binary-exact midpoint probes allow a separate
    # FontTools ItemVariationStore calculation without approximate tolerances.
    avar = font["avar"] = newTable("avar")
    avar.segments = {
        "wght": {-1: -1, 0: 0, 0.5: 0.75, 1: 1},
        "wdth": {-1: -1, 0: 0, 0.5: 0.25, 1: 1},
    }
    regions = buildVarRegionList([
        {"wght": (0, 1, 1)},
        {"wdth": (0, 1, 1)},
        {"wght": (-1, -1, 0), "wdth": (-1, -1, 0)},
    ], ["wght", "wdth"])
    tags = sorted(MVAR_ENTRIES)
    deltas = [[8 + i * 2, -4 - i, 3 + i] for i in range(len(tags))]
    mvar = newTable("MVAR")
    mvar.table = ot.MVAR()
    mvar.table.Version = 0x10000
    mvar.table.Reserved = 0
    mvar.table.ValueRecordSize = 8
    mvar.table.ValueRecordCount = len(tags)
    mvar.table.ValueRecord = []
    for i, tag in enumerate(tags):
        rec = ot.MetricsValueRecord()
        rec.ValueTag = tag
        rec.VarIdx = i
        mvar.table.ValueRecord.append(rec)
    mvar.table.VarStore = buildVarStore(regions, [buildVarData([0, 1, 2], deltas, optimize=False)])
    font["MVAR"] = mvar
    name = "owned-metrics-typo.ttf" if typo else "owned-metrics-hhea.ttf"
    font.save(root / name)
    data = (root / name).read_bytes()
    records.append({"name": name, "sha256": hashlib.sha256(data).hexdigest(), "byteLength": len(data)})
(root / "owned-metrics.json").write_text(json.dumps({"generator": "fonttools 4.61.1", "files": records}, indent=2) + "\n")
print(json.dumps(records))
