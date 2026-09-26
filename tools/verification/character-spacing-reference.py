"""Owned page oracle: raw XML character spacing and sizes, FontTools metrics and Fraction layout.
Selected glyph integers and line topology remain declared inputs; no target-app claim.
"""
from fractions import Fraction as F
import hashlib
import json
from pathlib import Path
import zipfile
import numpy as np
from PIL import Image
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen
from line_geometry_math import reference, baseline
from mce_reference import project, A, P

ROOT=Path('.codex-work/character-spacing'); UNIT=2**32
NS=dict(a=A,p=P)
font=TTFont('fixtures/fonts/owned-tracking.ttf')
manifest=json.loads(Path('fixtures/fonts/tracking-manifest.json').read_text())
def entry(p):
    p=Path(p);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def pct(s):return F(s[:-1])/100 if s.endswith('%') else F(s)/100000
def fixed(s):return F(int(s),UNIT)
def tracking(s):
    for unit,factor in [('mm',36000),('cm',360000),('in',914400),('pt',12700),('pc',152400),('pi',152400)]:
        if s.endswith(unit):return F(s[:-len(unit)])*factor
    return F(s)*127
def local(n):return n.tag.split('}')[-1]
counts=dict(packages=0,paragraphs=0,spacingDeclarations=0,trackingDeclarations=0,styleValues=0,metricValues=0,lines=0,glyphCoordinates=0,decorationRectangles=0,interiorPixels=0,excludedEdgePixels=0)
records=[]
ys,xs=np.mgrid[:300,:400];xx=xs+0.5;yy=ys+0.5
for path in sorted((ROOT/'cases').glob('*.plan.json')):
    name=path.name.removesuffix('.plan.json');source=path.with_name(name+'.pptx')
    plan=json.loads(path.read_text());assert len(plan['texts'])==1
    t=plan['texts'][0];frame=t['frame'];bound=fixed(t['localCoordinateErrorBound'])
    with zipfile.ZipFile(source) as z:xml,_,ordinals=project(z.read('ppt/slides/slide1.xml'),True)
    shape=xml.find('.//p:sp',NS);body=shape.find('p:txBody',NS)
    defaults=body.find('a:lstStyle/a:lvl1pPr',NS);default_char=defaults.find('a:defRPr',NS)
    bodypr=body.find('a:bodyPr',NS)
    outer=bodypr.get('spcFirstLastPara','0')=='1';anchor=bodypr.get('anchor','t')
    xfrm=shape.find('p:spPr/a:xfrm',NS);assert not xfrm.attrib
    off=xfrm.find('a:off',NS);ext=xfrm.find('a:ext',NS)
    ox,oy=F(off.get('x')),F(off.get('y'));width,height=F(ext.get('cx')),F(ext.get('cy'))
    image=np.full((300,400,4),255,dtype=np.uint8);mask=np.ones((300,400),dtype=bool)
    def polygon(points,color):
        points=[(float((x+ox)/4000),float((y+oy)/4000)) for x,y in points]
        if points[0]==points[-1]:points=points[:-1]
        inside=np.zeros((300,400),dtype=bool)
        for (x0,y0),(x1,y1) in zip(points,points[1:]+points[:1]):
            dx,dy=x1-x0,y1-y0;length=dx*dx+dy*dy
            if not length:continue
            ratio=np.clip(((xx-x0)*dx+(yy-y0)*dy)/length,0,1)
            mask[:]&=(xx-x0-ratio*dx)**2+(yy-y0-ratio*dy)**2>4
            if dy:inside[:]^=((y0>yy)!=(y1>yy))&(xx<dx*(yy-y0)/dy+x0)
        image[inside]=color
    polygon([(0,0),(width,0),(width,height),(0,height)],[244,234,220,255])
    accumulated=F(0);glyphs=[];decoration_cells=[]
    paragraphs=body.findall('a:p',NS)
    assert len(paragraphs)==len(frame['paragraphs'])
    for pi,paragraph in enumerate(paragraphs):
        native=frame['text']['paragraphs'][pi];actual=frame['paragraphs'][pi];inp=frame['inputs'][pi]
        computed=actual['computed'];geo=computed['geometry']['paths']['layout']['geometry'];shaped=geo['shaping']
        ppr=paragraph.find('a:pPr',NS)
        runs=[n for n in paragraph if local(n) in ['r','br']]
        end=paragraph.find('a:endParaRPr',NS)
        def char(n):
            attrs=dict(default_char.attrib)
            if n is not None:attrs.update(n.attrib)
            return attrs
        styles=[None]*len(inp['geometry'])
        for binding in inp['fonts']:
            ri,si=binding['run'],binding['style']
            attrs=char(end if ri is None else runs[ri].find('a:rPr',NS))
            size=F(int(attrs['sz'])*127);shift=pct(attrs.get('baseline','0'))*size
            assert F(inp['geometry'][si]['fontSize'])==size
            assert abs(baseline(inp['geometry'][si]['baselineShift'])-shift)<=F(1,2*UNIT)
            space=tracking(attrs.get('spc','0'))
            assert abs(fixed(inp['geometry'][si].get('clusterSpacing','0'))-space)<=F(1,2*UNIT)
            features=inp['styles'][si]['features']
            assert [f['tag'] for f in features]==(['kern','liga','clig'] if space else ['kern'])
            styles[si]=dict(fontSize=str(size),baselineShift=shift,clusterSpacing=space*UNIT)
            source_node=end if ri is None else runs[ri].find('a:rPr',NS)
            if source_node is None or source_node.get('spc') is None:source_node=default_char
            if source_node.get('spc') is not None:
                source_style=native['endStyle'] if ri is None else native['runs'][ri]['style']
                assert source_style['origins']['spacing']==dict(kind='object',object=dict(part='/ppt/slides/slide1.xml',nativeId=42),sourceOrdinal=ordinals[source_node])
                counts['trackingDeclarations']+=1
            counts['styleValues']+=1
        def selected(slot):
            for owner in [ppr,defaults]:
                if owner is not None:
                    n=owner.find('a:'+slot,NS)
                    if n is not None:return n
            return None
        declarations={slot:selected(slot) for slot in ['lnSpc','spcBef','spcAft']}
        slots=dict(lnSpc='lineSpacing',spcBef='spaceBefore',spcAft='spaceAfter')
        for slot,n in declarations.items():
            if n is None:continue
            d=native['declarations'][slots[slot]]
            assert d['element']==slot and d['origin']==dict(kind='object',object=dict(part='/ppt/slides/slide1.xml',nativeId=42),sourceOrdinal=ordinals[n])
            counts['spacingDeclarations']+=1
        def heights(n):
            if local(n[0])=='spcPts':return [F(int(n[0].get('val'))*127)]*len(styles)
            assert local(n[0])=='spcPct'
            return [pct(n[0].get('val'))*F(s['fontSize']) for s in styles]
        def max_line(line,values):
            values_on_line=[values[item['style']] for item in shaped['items'][line['itemStart']:line['itemEnd']] if item['kind']=='text']
            return max(values_on_line) if values_on_line else values[inp['endStyle']]
        n=declarations['lnSpc']
        spacing=dict(kind='natural') if n is None else dict(kind='styleMaximum',heights=[v*UNIT for v in heights(n)])
        q=dict(shaping=dict(paragraph=dict(fonts=manifest['fonts'],styles=[dict(candidates=[b['candidate']]) for b in computed['bindings']])),styles=styles,strutStyle=inp['endStyle'],spacing=spacing)
        expected=reference(q,geo,False)
        for m in geo['metricInstances']:
            assert not m['variations'] or all(v['value1616']==400*65536 for v in m['variations'])
            assert [v['position'] for v in m['measured']['values']]==[getattr(font['hhea'],k)*64 for k in ['ascent','descent','lineGap']]
            counts['metricValues']+=3
        before=max_line(shaped['lines'][0],heights(declarations['spcBef'])) if declarations['spcBef'] is not None and (pi>0 or outer) else F(0)
        after=max_line(shaped['lines'][-1],heights(declarations['spcAft'])) if declarations['spcAft'] is not None and (pi+1<len(paragraphs) or outer) else F(0)
        assert abs(fixed(actual['appliedBefore'])-before)<=F(1,2*UNIT)
        assert abs(fixed(actual['appliedAfter'])-after)<=F(1,2*UNIT)
        accumulated+=before
        ranges=[];scalar=0
        for run in runs:
            attrs=char(run.find('a:rPr',NS));text='\u2028' if local(run)=='br' else run.find('a:t',NS).text
            ranges.append((scalar,scalar+len(text or ''),attrs));scalar+=len(text or '')
        gi=0
        for li,line in enumerate(expected['lines']):
            align=ppr.get('algn','l') if ppr is not None else 'l'
            horizontal=F(0) if align=='l' else width-line['penMax']
            if align=='ctr':horizontal=(width-line['penMax']-line['penMin'])/2
            actual_line=computed['geometry']['precise']['lines'][li]
            for key in ['top','baseline','bottom','height']:
                assert abs(fixed(actual_line[key])-line[key])<=bound,(name,pi,key)
            for g in line['glyphs']:
                i,j=g['source']['fallbackItem'],g['source']['fragment']
                fragment=shaped['fallback']['items'][i]['fragments'][j];run=fragment['shaped']['runs'][0];raw=run['glyphs'][g['glyph']]
                si=shaped['items'][shaped['shapedItemIndices'][i]]['style'];size=F(styles[si]['fontSize'])
                assert not run['effectiveVariations'] or all(v['requested1616']==400*65536 for v in run['effectiveVariations'])
                pen=RecordingPen();font.getGlyphSet()[font.getGlyphName(raw['glyphId'])].draw(pen)
                pts=[]
                for op,values in pen.value:
                    assert op in ['moveTo','lineTo','closePath']
                    pts.extend((F(x)*size/1000+g['x']+horizontal,-F(y)*size/1000+g['y']+accumulated) for x,y in values)
                glyphs.append(dict(paragraph=pi,glyph=gi,x=g['x']+horizontal,y=g['y']+accumulated,points=pts));gi+=1
                attrs=next(attrs for start,end,attrs in ranges if start<=raw['cluster']<end)
                scale=size/fragment['shaped']['positionUnitsPerEm'];x0=g['x']+horizontal-raw['xOffset']*scale;x1=x0+raw['xAdvance']*scale
                glyph_index=g['glyph'];all_glyphs=run['glyphs']
                if glyph_index+1==len(all_glyphs) or all_glyphs[glyph_index+1]['cluster']!=raw['cluster']:
                    x1+=F(styles[si]['clusterSpacing'])/UNIT
                if x0==x1:continue
                base=g['y']+accumulated+raw['yOffset']*scale
                for enabled,kind,table,pos,thickness in [
                    (attrs.get('u')=='sng','underline','post','underlinePosition','underlineThickness'),
                    (attrs.get('strike')=='sngStrike','strike','OS/2','yStrikeoutPosition','yStrikeoutSize')]:
                    if not enabled:continue
                    top=base-getattr(font[table],pos)*size/1000;bottom=top+getattr(font[table],thickness)*size/1000
                    decoration_cells.append(dict(paragraph=pi,line=li,kind=kind,rect=[min(x0,x1),top,max(x0,x1),bottom]))
        accumulated+=expected['height']+after
        counts['paragraphs']+=1;counts['lines']+=len(expected['lines'])
    assert abs(fixed(frame['contentHeight'])-accumulated)<=bound
    vertical=F(0) if anchor=='t' else (height-accumulated)/(2 if anchor=='ctr' else 1)
    assert len(glyphs)==len(frame['glyphs'])
    for g,actual in zip(glyphs,frame['glyphs']):
        assert (g['paragraph'],g['glyph'])==(actual['paragraph'],actual['glyph'])
        for axis,value in [('x',g['x']),('y',g['y']+vertical)]:
            assert abs(fixed(actual['origin'][axis])-value)<=bound,(name,axis)
            counts['glyphCoordinates']+=1
        if g['points']:polygon([(x,y+vertical) for x,y in g['points']],[32,112,192,255])
    # Independently union collinear coverage intervals. Several advance cells
    # (including a combining mark's trailing tracking) form one decoration.
    groups={}
    for d in decoration_cells:
        x0,y0,x1,y1=d['rect']
        groups.setdefault((d['paragraph'],d['line'],d['kind'],y0,y1),[]).append((x0,x1))
    decoration_cells=[]
    for (pi,li,kind,y0,y1),segments in groups.items():
        merged=[]
        for x0,x1 in sorted(segments):
            if merged and x0<=merged[-1][1]:merged[-1]=(merged[-1][0],max(x1,merged[-1][1]))
            else:merged.append((x0,x1))
        decoration_cells.extend(dict(paragraph=pi,line=li,kind=kind,rect=[x0,y0,x1,y1]) for x0,x1 in merged)
    assert len(t['decorations'])==len(decoration_cells),(name,len(t['decorations']),len(decoration_cells))
    for d,actual in zip(decoration_cells,t['decorations']):
        x0,y0,x1,y1=d['rect'];y0+=vertical;y1+=vertical
        assert actual['paragraph']==d['paragraph'] and actual['line']==d['line'] and actual['kind']==d['kind']
        values=[fixed(actual['rect'][corner][axis]) for corner,axis in [('min','x'),('min','y'),('max','x'),('max','y')]]
        assert all(abs(a-b)<=bound for a,b in zip(values,[x0,y0,x1,y1]))
        polygon([(x0,y0),(x1,y0),(x1,y1),(x0,y1)],[32,112,192,255]);counts['decorationRectangles']+=1
    pixels=np.frombuffer(path.with_name(name+'.rgba').read_bytes(),dtype=np.uint8).reshape(300,400,4)
    assert np.array_equal(pixels[mask],image[mask]),(name,int(np.any(pixels!=image,axis=2)[mask].sum()))
    Image.fromarray(pixels).save(ROOT/(name+'.png'))
    counts['packages']+=1;counts['interiorPixels']+=int(mask.sum());counts['excludedEdgePixels']+=int((~mask).sum())
    records.append(dict(name=name,source=entry(source),plan=entry(path),pixels=entry(path.with_name(name+'.rgba'))))
assert counts['packages']==15
(ROOT/'reference.json').write_text(json.dumps(dict(format='musteroffice.character-spacing-reference/1',counts=counts,font=entry('fixtures/fonts/owned-tracking.ttf'),cases=records),indent=2)+'\n')
print(json.dumps(counts))
