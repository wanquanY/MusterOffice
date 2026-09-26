"""Original synthetic geometry/metadata, generated reproducibly with FontTools 4.61.1.
No outlines or metadata copied from third-party fonts. These are test fonts, not visual assets.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.ttLib import TTCollection, newTable
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.ttLib.tables._n_a_m_e import NameRecord
import hashlib
import json

root = Path('fixtures/fonts')
root.mkdir(parents=True, exist_ok=True)
order = ['.notdef', 'space', 'A', 'acutecomb', 'alpha', 'smile', 'A.alt']
codes = {32:'space',65:'A',0x301:'acutecomb',0x3b1:'alpha',0x1f600:'smile'}
def create(cff=False):
    fb = FontBuilder(1000,isTTF=not cff)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap(codes)
    glyphs = {}
    for i,name in enumerate(order):
        pen = T2CharStringPen(600,None) if cff else TTGlyphPen(None)
        if name != 'space':
            pen.moveTo((50+i,0));pen.lineTo((300,700-i));pen.lineTo((550,0));pen.closePath()
        glyphs[name] = pen.getCharString() if cff else pen.glyph()
    if cff:
        fb.setupCFF('MusterOfficeSyntheticCFF', {'FullName':'MusterOffice Synthetic CFF','FamilyName':'MusterOffice Synthetic CFF','Weight':'Regular'}, glyphs, {})
    else:
        fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({name:(600,0) for name in order})
    fb.setupHorizontalHeader(ascent=800,descent=-200,lineGap=25)
    fb.setupVerticalMetrics({name:(1000,100) for name in order})
    fb.setupVerticalHeader(ascent=500,descent=-500,lineGap=0)
    fb.setupNameTable({'familyName':'MusterOffice Synthetic CFF' if cff else 'MusterOffice Synthetic', 'styleName':'Regular','uniqueFontIdentifier':'MusterOffice owned fixture 1','fullName':'MusterOffice Synthetic Fixture','psName':'MusterOfficeSynthetic','version':'Version 1.000','copyright':'Original synthetic test geometry and metadata by MusterOffice'})
    unknown = NameRecord();unknown.platformID=4;unknown.platEncID=9;unknown.langID=0;unknown.nameID=256;unknown.string=b'opaque';fb.font['name'].names.append(unknown)
    fb.setupOS2(sTypoAscender=750,sTypoDescender=-250,sTypoLineGap=40,usWinAscent=850,usWinDescent=250,fsType=0x0104)
    fb.setupPost()
    fb.setupMaxp()
    if not cff:
        fb.setupFvar([('wght',100,400,900,'Weight'),('wdth',75,100,125,'Width')],
            [{'location':{'wght':400,'wdth':100},'stylename':'Regular'}, {'location':{'wght':700,'wdth':100},'stylename':'Bold'}])
        uv = CmapSubtable.newSubtable(14);uv.platformID=0;uv.platEncID=5;uv.language=0;uv.cmap={}
        uv.uvsDict={0xFE00:[(65,None)],0xFE01:[(65,'A.alt')],0xE0100:[(0x3b1,'alpha')]}
        fb.font['cmap'].tables.append(uv)
    fb.font['head'].created=fb.font['head'].modified=2082844800
    fb.font.recalcTimestamp=False
    return fb.font
fonts=[]
for cff, name in [(False,'owned.ttf'),(True,'owned.otf')]:
    font=create(cff);font.save(root/name);fonts.append(font)
collection=TTCollection();collection.fonts=fonts;collection.save(root/'owned.ttc')
receipt=[]
for path in [root/'owned.ttf',root/'owned.otf',root/'owned.ttc']:
    data=path.read_bytes();receipt.append({'name':path.name,'sha256':hashlib.sha256(data).hexdigest(),'byteLength':len(data)})
(root/'owned.json').write_text(json.dumps({'generator':'fonttools 4.61.1','files':receipt},indent=2)+'\n')
print(json.dumps(receipt))
