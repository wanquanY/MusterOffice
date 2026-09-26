"""Independent FontTools outline decomposition on project-owned curve fixtures.
A second HarfBuzz entry is insufficient evidence for the curve interpreter itself.
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.basePen import BasePen
from fontTools.pens.transformPen import TransformPen
import hashlib,json,math
root=Path('.codex-work/font-outlines');sha=lambda b:hashlib.sha256(b).hexdigest()
def rounded(v):return math.floor(v+0.5) if v>=0 else math.ceil(v-0.5)
class Pen(BasePen):
    def __init__(self,glyphs):super().__init__(glyphs);self.path=[];self.start=None;self.current=None
    def point(self,p):return {'x':rounded(p[0]*64),'y':rounded(p[1]*64)}
    def _moveTo(self,p):self.start=self.current=p;self.path.append({'kind':'move','to':self.point(p)})
    def _lineTo(self,p):self.current=p;self.path.append({'kind':'line','to':self.point(p)})
    def _qCurveToOne(self,p1,p2):self.current=p2;self.path.append({'kind':'quadratic','control':self.point(p1),'to':self.point(p2)})
    def _curveToOne(self,p1,p2,p3):self.current=p3;self.path.append({'kind':'cubic','control1':self.point(p1),'control2':self.point(p2),'to':self.point(p3)})
    def _closePath(self):
        if self.current!=self.start:self._lineTo(self.start)
        self.path.append({'kind':'close'})
    def _endPath(self):raise AssertionError('open contour')
cases=[];glyphs_checked=commands=coordinates=0
for name in ['owned.ttf-0','owned.otf-0','owned.ttc-0','owned.ttc-1','owned-quadratic-variable','owned-cubic']:
    q=json.loads((root/(name+'.request.json')).read_text());data=(root/(name+'.response.json')).read_bytes();result=json.loads(data)['outlines']
    path='fixtures/fonts/'+({'owned-quadratic-variable':'owned-outlines.ttf','owned-cubic':'owned-outlines.otf'}.get(name,name.rsplit('-',1)[0]))
    font=TTFont(path,fontNumber=q['faceIndex']);assert sha(Path(path).read_bytes())==q['expectedSha256']
    checked=0
    for instance,actual in zip(q['instances'],result['instances']):
        # The native axes are f32 user coordinates. These fixture values are exact.
        location={v['tag']:v['value1616']/65536 for v in instance['variations']}
        glyphset=font.getGlyphSet(location=location)
        for gid,glyph in zip(instance['glyphIds'],actual['glyphs']):
            pen=Pen(glyphset);glyphname=font.getGlyphOrder()[gid];font_glyph=glyphset[glyphname]
            draw_pen=pen
            # FontTools Glyph.draw applies lsb-xMin to simple glyphs but ignores
            # its offset argument for composites. Apply that documented origin
            # conversion once at the composite root, never to component children.
            if 'glyf' in font:
                instance_glyph,offset=font_glyph._getGlyphAndOffset()
                if instance_glyph.isComposite():draw_pen=TransformPen(pen,(1,0,0,1,offset,0))
            font_glyph.draw(draw_pen)
            assert glyph['path']==pen.path,(name,location,gid,glyph['path'],pen.path)
            checked+=1;commands+=len(pen.path)
            coordinates+=sum(2*sum(isinstance(v,dict) for v in c.values()) for c in pen.path)
    glyphs_checked+=checked;cases.append({'name':name,'fontSha256':q['expectedSha256'],'responseSha256':sha(data),'glyphs':checked})
report={'format':'musteroffice.font-outlines-fonttools-reference/1','implementation':'FontTools 4.61.1 BasePen, separate glyf/gvar/CFF interpreters','cases':cases,'glyphs':glyphs_checked,'commands':commands,'coordinates':coordinates,'exactQuantizedCoordinates':True,'compositeOrigin':'FontTools 4.61.1 composite draw ignores offset; root lsb-xMin applied from its own font instance', 'scope':'Owned fixtures only; no independent proof for arbitrary malformed, VARC or CFF2 fonts'}
(root/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ['glyphs','commands','coordinates']}))
