"""Owned caret font with script aliases and explicit GPOS offsets for interaction tests.
Aliases only test layout/index behavior; they are not authentic language glyphs.
"""
from pathlib import Path
import copy,hashlib,json
from fontTools import version
from fontTools.ttLib import TTFont
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
assert version=='4.61.1'
p=Path('fixtures/fonts');f=TTFont(p/'owned-carets.ttf',recalcTimestamp=False)
for table in f['cmap'].tables:
    if table.isUnicode() and table.format!=14:
        table.cmap.update({97:'A',0x5d0:'A',0x5d1:'alpha',0x5d2:'smile',0x4e2d:'A',0x6587:'alpha'})
gdef=copy.deepcopy(f['GDEF'])
addOpenTypeFeaturesFromString(f, '''
languagesystem DFLT dflt;
languagesystem latn dflt;
languagesystem hebr dflt;
feature liga { sub A A A by A.alt; sub alpha alpha by smile; } liga;
feature kern { pos A.alt <37 11 -23 0>; } kern;
''')
f['GDEF']=gdef
f.save(p/'owned-interaction.ttf')
data=(p/'owned-interaction.ttf').read_bytes()
(p/'owned-interaction.json').write_text(json.dumps({'generator':'fonttools 4.61.1','source':'owned-carets.ttf','sha256':hashlib.sha256(data).hexdigest(),'byteLength':len(data),'gpos':{'A.alt':{'xOffset':37,'yOffset':11,'advanceDelta':-23}},'languageQualityFixture':False},indent=2)+'\n')
print(hashlib.sha256(data).hexdigest())
