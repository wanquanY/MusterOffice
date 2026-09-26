"""Owned curves and composite geometry; FontTools 4.61.1, no copied outlines."""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.ttLib import newTable
from fontTools.ttLib.tables.TupleVariation import TupleVariation
import hashlib,json
root=Path('fixtures/fonts');order=['.notdef','space','curves','component']
files=[]
for cff in [False,True]:
    fb=FontBuilder(1000,isTTF=not cff);fb.setupGlyphOrder(order);fb.setupCharacterMap({32:'space',65:'curves',66:'component'})
    glyphs={}
    for name in order:
        pen=T2CharStringPen(1000,None) if cff else TTGlyphPen(glyphs)
        if name=='component' and not cff:
            pen.addComponent('curves',(0.5,0,0,0.5,100,200));pen.addComponent('curves',(1,0,0,1,-200,-300))
        elif name!='space':
            pen.moveTo((0,0))
            if cff:pen.curveTo((-200,1200),(800,-300),(600,600))
            else:pen.qCurveTo((-200,1000),(400,1200),(600,600))
            pen.lineTo((600,-200));pen.lineTo((0,0));pen.closePath()
            pen.moveTo((200,100));pen.lineTo((400,100));pen.lineTo((300,300));pen.closePath()
        glyphs[name]=pen.getCharString() if cff else pen.glyph()
    if cff:fb.setupCFF('MusterOfficeOwnedCurves',{'FullName':'MusterOffice Owned Curves','FamilyName':'MusterOffice Owned Curves','Weight':'Regular'},glyphs,{})
    else:fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics({name:(1000,0) for name in order});fb.setupHorizontalHeader(ascent=1000,descent=-400)
    fb.setupNameTable({'familyName':'MusterOffice Owned Curves','styleName':'Regular','uniqueFontIdentifier':'MusterOffice owned outline fixture 1','fullName':'MusterOffice Owned Curves','psName':'MusterOfficeOwnedCurves','version':'Version 1.000','copyright':'Original synthetic test geometry by MusterOffice'})
    fb.setupOS2(sTypoAscender=1000,sTypoDescender=-400,usWinAscent=1400,usWinDescent=600);fb.setupPost();fb.setupMaxp()
    if not cff:
        fb.setupFvar([('wght',100,400,900,'Weight')],[])
        gvar=newTable('gvar');gvar.variations={}
        for name in order:
            coords=glyphs[name].getCoordinates(fb.font['glyf'])[0] if name!='component' else None
            if coords and name!='component':gvar.variations[name]=[TupleVariation({'wght':(0,1,1)},[(i*10,50 if i%2 else -20) for i in range(len(coords))]+[(0,0)]*4)]
            else:gvar.variations[name]=[]
        fb.font['gvar']=gvar
    fb.font['head'].created=fb.font['head'].modified=2082844800;fb.font.recalcTimestamp=False
    path=root/('owned-outlines.otf' if cff else 'owned-outlines.ttf');fb.font.save(path)
    data=path.read_bytes();files.append({'name':path.name,'sha256':hashlib.sha256(data).hexdigest(),'byteLength':len(data)})
(root/'owned-outlines.json').write_text(json.dumps({'generator':'fonttools 4.61.1','files':files},indent=2)+'\n')
print(json.dumps(files))
