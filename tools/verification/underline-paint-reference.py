"""Independent XML/FontTools/Fraction decoration and opaque interior pixel checks.
HarfBuzz integer glyph placement is a declared input, not independently certified.
The oracle reads no production decoration coordinates to construct its image.
"""
from decimal import Decimal as D
from fractions import Fraction as F
import hashlib
import json
import math
from pathlib import Path
import zipfile
import numpy as np
from PIL import Image
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen
from fontTools.varLib.models import normalizeLocation, piecewiseLinearMap
from fontTools.varLib.varStore import VarStoreInstancer
from line_geometry_math import reference
from page_reference_math import cs
from mce_reference import project, A, P

ROOT = Path('.codex-work/underline-paint')
NS = dict(a=A, p=P)
FONT = TTFont('fixtures/fonts/owned-decorations.ttf')
MANIFEST = json.loads(Path('fixtures/fonts/decoration-manifest.json').read_text())
axes = {a.axisTag: (a.minValue, a.defaultValue, a.maxValue) for a in FONT['fvar'].axes}
records = {r.ValueTag: r.VarIdx for r in FONT['MVAR'].table.ValueRecord}
def entry(p):
    p=Path(p); b=p.read_bytes()
    return dict(path=str(p), byteLength=len(b), sha256=hashlib.sha256(b).hexdigest())
def nearest(x): return math.floor(x+0.5) if x>=0 else math.ceil(x-0.5)
def metric(tag, table, name, variations):
    loc=normalizeLocation({v['tag']:v['value1616']/65536 for v in variations}, axes)
    # HarfBuzz normalizes to F2Dot14 before avar; avar outputs F2Dot14 too.
    loc={k:nearest(v*16384)/16384 for k,v in loc.items()}
    loc={k:nearest(piecewiseLinearMap(v,FONT['avar'].segments[k])*16384)/16384 for k,v in loc.items()}
    inst=VarStoreInstancer(FONT['MVAR'].table.VarStore,FONT['fvar'].axes,loc)
    return nearest((getattr(FONT[table],name)+inst[records[tag]])*64)
def fixed(x): return F(int(x),2**32)
def transform(shape):
    x=shape.find('p:spPr/a:xfrm',NS)
    off=x.find('a:off',NS);ext=x.find('a:ext',NS)
    w,h=D(ext.get('cx')),D(ext.get('cy'))
    c,s=cs(int(x.get('rot','0')))
    def apply(pt):
        a,b=[D(v.numerator)/D(v.denominator) if isinstance(v,F) else D(v) for v in pt]
        a=(a-w/2)*(-1 if x.get('flipH')=='1' else 1)
        b=(b-h/2)*(-1 if x.get('flipV')=='1' else 1)
        return [(D(off.get('x'))+w/2+c*a-s*b)/4000,(D(off.get('y'))+h/2+s*a+c*b)/4000]
    return apply,(w,h)
ys,xs=np.mgrid[:300,:400]; xx=xs+0.5; yy=ys+0.5
counts=dict(packages=0, metricValues=0, paintDeclarations=0, decorationRectangles=0, interiorPixels=0, excludedEdgePixels=0)
results=[]
for path in sorted((ROOT/'cases').glob('*.pptx')):
    if not path.with_suffix('.plan.json').exists():continue
    name=path.stem;plan=json.loads(path.with_suffix('.plan.json').read_text())
    with zipfile.ZipFile(path) as z: xml,_,ordinals=project(z.read('ppt/slides/slide1.xml'),True)
    shapes=xml.findall('.//p:sp',NS);assert len(shapes)==1 and len(plan['texts'])==1
    shape=shapes[0];t=plan['texts'][0];frame=t['frame'];assert len(frame['paragraphs'])==1
    apply,(w,h)=transform(shape)
    expected=np.full((300,400,4),255,dtype=np.uint8); mask=np.ones((300,400),dtype=bool)
    def polygon(points,color):
        pts=[tuple(map(float,p)) for p in points]
        if pts[0]==pts[-1]:pts=pts[:-1]
        inside=np.zeros((300,400),dtype=bool)
        for (x0,y0),(x1,y1) in zip(pts,pts[1:]+pts[:1]):
            dx,dy=x1-x0,y1-y0;length=dx*dx+dy*dy
            if not length:continue
            a=np.clip(((xx-x0)*dx+(yy-y0)*dy)/length,0,1)
            mask[:]&=(xx-x0-a*dx)**2+(yy-y0-a*dy)**2>4
            if dy:inside[:]^=((y0>yy)!=(y1>yy))&(xx<dx*(yy-y0)/dy+x0)
        expected[inside]=color
    polygon([apply(p) for p in [(0,0),(w,0),(w,h),(0,h)]],[244,234,220,255])
    ranges=[];offset=0
    page=json.loads(path.with_suffix('.page.json').read_text())
    defaults=shape.find('p:txBody/a:lstStyle/a:lvl1pPr/a:defRPr',NS)
    fontref=shape.find('p:style/a:fontRef/*',NS)
    def color(fill, placeholder=False):
        assert fill is not None
        if fill.tag=='{'+A+'}noFill':return None
        assert fill.tag=='{'+A+'}solidFill'
        n=fill[0]
        if n.tag=='{'+A+'}srgbClr':
            v=n.get('val');values=[int(v[i:i+2],16)/255 for i in [0,2,4]]
        elif n.tag=='{'+A+'}sysClr':
            values=[v/255 for v in page['colorContext']['systemColors'][n.get('val')]]
        else:
            assert not placeholder and n.tag=='{'+A+'}schemeClr' and n.get('val')=='phClr'
            assert fontref is not None and fontref.tag=='{'+A+'}srgbClr'
            v=fontref.get('val');values=[int(v[i:i+2],16)/255 for i in [0,2,4]]
        for op in n:
            assert op.tag=='{'+A+'}shade'
            linear=[v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4 for v in values]
            linear=[v*int(op.get('val'))/100000 for v in linear]
            values=[v*12.92 if v<=0.0031308 else 1.055*v**(1/2.4)-0.055 for v in linear]
        return [math.floor(v*255+0.5) for v in values]+[255]
    def selected(style,names):
        for owner in [style,defaults]:
            if owner is not None:
                found=[n for n in owner if n.tag in ['{'+A+'}'+name for name in names]]
                if found:return found[0]
        return None
    for ri,run in enumerate(shape.findall('p:txBody/a:p/a:r',NS)):
        text=run.find('a:t',NS).text or '';style=run.find('a:rPr',NS)
        fill=selected(style,['solidFill','noFill']);glyph_color=color(fill)
        def attr(name,default):
            v=style.get(name) if style is not None else None
            return v if v is not None else defaults.get(name,default)
        u=attr('u','none')=='sng';strike=attr('strike','noStrike')=='sngStrike'
        ufill=selected(style,['uFill','uFillTx']) if u else None
        independent=u and ufill is not None and ufill.tag=='{'+A+'}uFill'
        line_color=(color(ufill[0]) if independent else glyph_color) if u else None
        rpaint=t['paints'][0][ri]
        assert rpaint['fill']['declaration']['origin']['sourceOrdinal']==ordinals[fill]
        counts['paintDeclarations']+=1
        assert (rpaint['underline'] is not None)==u
        if u:
            item=rpaint['underline'];assert item['kind']==('independent' if independent else 'followText')
            if ufill is not None:
                assert item['declaration']['origin']['sourceOrdinal']==ordinals[ufill]
                counts['paintDeclarations']+=1
            if independent:
                assert item['fill']['declaration']['origin']['sourceOrdinal']==ordinals[ufill[0]]
                counts['paintDeclarations']+=1
        ranges.append((offset,offset+len(text),glyph_color,line_color,strike));offset+=len(text)
    computed=frame['paragraphs'][0]['computed'];source=frame['inputs'][0]
    geometry=computed['geometry']['paths']['layout']['geometry'];shaped=geometry['shaping']
    for m in geometry['metricInstances']:
        actual=[v['position'] for v in m['measured']['values']]
        values=[metric(tag,'hhea',field,m['variations']) for tag,field in [('hasc','ascent'),('hdsc','descent'),('hlgp','lineGap')]]
        assert actual==values,(name,actual,values);counts['metricValues']+=3
    q=dict(shaping=dict(paragraph=dict(fonts=MANIFEST['fonts'],styles=[dict(candidates=[b['candidate']]) for b in computed['bindings']])),
           styles=source['geometry'],strutStyle=source['endStyle'],spacing=dict(kind='natural'))
    positioned=reference(q,geometry,False)
    cells=[]
    for line_index,line in enumerate(positioned['lines']):
        for pos in line['glyphs']:
            i,j=pos['source']['fallbackItem'],pos['source']['fragment']
            fragment=shaped['fallback']['items'][i]['fragments'][j];run=fragment['shaped']['runs'][0];g=run['glyphs'][pos['glyph']]
            size=int(source['geometry'][shaped['items'][shaped['shapedItemIndices'][i]]['style']]['fontSize'])
            scale=F(size,fragment['shaped']['positionUnitsPerEm'])
            start=g['cluster'];end=min([v['cluster'] for v in run['glyphs'] if v['cluster']>start]+[fragment['end']])
            owned=[v for v in ranges if v[0]<end and v[1]>start];assert owned and all(v[2:]==owned[0][2:] for v in owned)
            _,_,glyph_color,underline_color,strike=owned[0]
            variations=[dict(tag=v['tag'],value1616=v['requested1616']) for v in run['effectiveVariations']]
            glyphset=FONT.getGlyphSet(location={v['tag']:v['value1616']/65536 for v in variations})
            pen=RecordingPen();glyphset[FONT.getGlyphName(g['glyphId'])].draw(pen)
            points=[]
            for op,pts in pen.value:
                assert op in ['moveTo','lineTo','closePath'],op
                points.extend((F(x)*size/1000+pos['x'],-F(y)*size/1000+pos['y']) for x,y in pts)
            if glyph_color is not None and points:polygon([apply(p) for p in points],glyph_color)
            pen_x=pos['x']-g['xOffset']*scale;end_x=pen_x+g['xAdvance']*scale
            if pen_x==end_x:continue
            baseline=pos['y']+g['yOffset']*scale
            for line_color,kind,table,offset_name,size_name,otag,stag in [
                (underline_color,'underline','post','underlinePosition','underlineThickness','undo','unds'),
                (glyph_color if strike else None,'strike','OS/2','yStrikeoutPosition','yStrikeoutSize','stro','strs')]:
                if line_color is None:continue
                top=baseline-metric(otag,table,offset_name,variations)*scale
                bottom=top+metric(stag,table,size_name,variations)*scale
                rect=[min(pen_x,end_x),top,max(pen_x,end_x),bottom]
                cells.append(dict(line=line_index,kind=kind,start=start,end=end,rect=rect,color=line_color))
    for cell in cells:
        x0,y0,x1,y1=cell['rect']
        polygon([apply(p) for p in [(x0,y0),(x1,y0),(x1,y1),(x0,y1)]],cell['color'])
    # Check compiled merged rectangles independently against the covered spans.
    matched=set()
    for deco in t['decorations']:
        clusters=[t['clusters'][c] for c in deco['clusters']]
        members=[(i,c) for i,c in enumerate(cells) if c['kind']==deco['kind'] and c['line']==deco['line'] and any(c['start']==a['start'] and c['end']==a['end'] for a in clusters)]
        assert members and not any(i in matched for i,c in members)
        assert all(c['color']==deco['rgba'] for i,c in members)
        matched.update(i for i,c in members)
        expected_rect=[min(c['rect'][0] for i,c in members),min(c['rect'][1] for i,c in members),max(c['rect'][2] for i,c in members),max(c['rect'][3] for i,c in members)]
        actual=[fixed(deco['rect'][corner][axis]) for corner,axis in [('min','x'),('min','y'),('max','x'),('max','y')]]
        bound=fixed(t['localCoordinateErrorBound'])
        assert all(abs(a-b)<=bound for a,b in zip(actual,expected_rect)),(name,actual,expected_rect,bound)
        counts['decorationRectangles']+=1
    assert len(matched)==len(cells)
    pixels=np.frombuffer(path.with_suffix('.rgba').read_bytes(),dtype=np.uint8).reshape(300,400,4)
    assert np.array_equal(pixels[mask],expected[mask]),(name,int(np.any(pixels!=expected,axis=2)[mask].sum()))
    Image.fromarray(pixels).save(ROOT/(name+'.png'))
    counts['packages']+=1;counts['interiorPixels']+=int(mask.sum());counts['excludedEdgePixels']+=int((~mask).sum())
    results.append(dict(name=name,source=entry(path),plan=entry(path.with_suffix('.plan.json')),pixels=entry(path.with_suffix('.rgba')),interiorPixels=int(mask.sum())))
assert counts['packages']==12
(ROOT/'reference.json').write_text(json.dumps(dict(format='musteroffice.underline-paint-reference/1',counts=counts,font=entry('fixtures/fonts/owned-decorations.ttf'),cases=results),indent=2)+'\n')
print(json.dumps(counts))
