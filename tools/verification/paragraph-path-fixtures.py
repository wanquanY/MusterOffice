"""Owned derivatives test explicit unavailable/color prerequisites, not pixels."""
from fontTools.ttLib import TTFont
from fontTools.colorLib.builder import buildCPAL
from pathlib import Path
root=Path('.codex-work/paragraph-paths');root.mkdir(parents=True,exist_ok=True)
f=TTFont('fixtures/fonts/owned-outlines.ttf',recalcTimestamp=False)
for tag in ['glyf','loca','gvar']:del f[tag]
f.save(root/'no-outline.ttf')
f=TTFont('fixtures/fonts/owned-outlines.ttf',recalcTimestamp=False)
f['CPAL']=buildCPAL([[(1.0,0.0,0.0,1.0)]])
f.save(root/'palette.ttf')
