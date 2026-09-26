"""Independent source XML / Decimal / FontTools polygon and interior pixel oracle.

Native HarfBuzz + Skia test host is re-executed. Placement comes from XML frames,
outlines from FontTools, glyph positions from Fraction expressions over measured
font/shaping data. Two-pixel edge bands are excluded from the opaque pixel oracle;
this does not certify antialiasing, general paint, font metrics or Office fidelity.
"""
import argparse
from decimal import Decimal as D
from fractions import Fraction as F
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile
import platform
import fontTools
import numpy as np
from PIL import Image
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen
from line_geometry_math import reference
from page_reference_math import cs, U
from mce_reference import project, A, P

ROOT=Path('.codex-work/source-text-page')
NS=dict(a=A,p=P)
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--executable',required=True,type=Path)
args=parser.parse_args()
def entry(p):
    p=Path(p);b=p.read_bytes()
    return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def decimal(x):return D(x.numerator)/D(x.denominator) if isinstance(x,F) else D(x)
def transform(shape):
    ancestors=list(reversed(shape.xpath('ancestor::p:grpSp',namespaces=NS)))+[shape]
    center=[D(0),D(0)];scale=[D(1),D(1)];anchor=[D(0),D(0)];angle=0;flips=[False,False]
    for n,node in enumerate(ancestors):
        group=node.tag=='{'+P+'}grpSp'
        f=node.find('p:grpSpPr/a:xfrm' if group else 'p:spPr/a:xfrm',NS)
        def xy(name,keys):
            v=f.find('a:'+name,NS)
            return [D(v.get(k)) for k in keys]
        dst=xy('ext',['cx','cy']);src=xy('chExt',['cx','cy']) if group else dst
        off=xy('off',['x','y']);child=xy('chOff',['x','y']) if group else [D(0),D(0)]
        own=[off[i]+dst[i]/2 for i in range(2)];rotation=int(f.get('rot','0'))
        if n:
            delta=[(own[i]-anchor[i])*scale[i]*(-1 if flips[i] else 1) for i in range(2)]
            c,s=cs(angle);center=[center[0]+c*delta[0]-s*delta[1],center[1]+s*delta[0]+c*delta[1]]
            if ((rotation%21600000)+2700000)//5400000%2:scale.reverse()
            angle+=(-1 if flips[0]!=flips[1] else 1)*rotation
        else:center=own;angle=rotation
        scale=[scale[i]*dst[i]/src[i] for i in range(2)]
        flips=[flips[i]!=(f.get(k,'0') in ['true','1']) for i,k in enumerate(['flipH','flipV'])]
        anchor=[child[i]+src[i]/2 for i in range(2)]
    c,s=cs(angle);signed=[scale[i]*(-1 if flips[i] else 1) for i in range(2)]
    linear=[c*signed[0],-s*signed[1],s*signed[0],c*signed[1]]
    def apply(p):
        p=[decimal(p[i])-anchor[i] for i in range(2)]
        return [(sum(linear[2*r+i]*p[i] for i in range(2))+center[r])/4000 for r in range(2)]
    return apply
def rgb(node,fontref=None):
    if node.tag=='{'+A+'}schemeClr':
        assert node.get('val')=='phClr' and fontref is not None
        values=np.array(rgb(fontref)[:3],dtype=float)/255
    else:
        assert node.tag=='{'+A+'}srgbClr'
        values=np.array([int(node.get('val')[i:i+2],16) for i in [0,2,4]],dtype=float)/255
    for operation in node:
        assert operation.tag=='{'+A+'}shade'
        linear=np.where(values<=0.04045,values/12.92,((values+0.055)/1.055)**2.4)
        linear*=int(operation.get('val'))/100000
        values=np.where(linear<=0.0031308,linear*12.92,1.055*linear**(1/2.4)-0.055)
    return np.floor(values*255+0.5).astype(int).tolist()+[255]

env=dict(os.environ,MO_TEXT_PAGE_EVIDENCE_DIR=str((ROOT/'fixtures').resolve()))
env.pop('MO_TEXT_PAGE_CHILD',None)
run=subprocess.run([str(args.executable),'--test-threads=1'],env=env,capture_output=True,timeout=60)
(ROOT/'reference-native.log').write_bytes(run.stdout+run.stderr)
assert run.returncode==0 and b'7 passed;' in run.stdout,run.stdout+run.stderr
manifest_path=Path('fixtures/fonts/manifest-paragraph.json')
manifest=json.loads(manifest_path.read_text())['manifest']
font=TTFont('fixtures/fonts/owned.ttf');glyphset=font.getGlyphSet()
ys,xs=np.mgrid[:300,:400];xx=xs+0.5;yy=ys+0.5
records=[];counts=dict(packages=0,objects=0,glyphs=0,worldControlCoordinates=0,interiorPixels=0,excludedEdgePixels=0)
for pptx in sorted((ROOT/'fixtures').glob('*.pptx')):
    result=pptx.with_suffix('.json');plan=json.loads(result.read_text());page=plan['page']
    assert entry(pptx)['sha256']==pptx.stem==page['info']['sourceSha256']
    with zipfile.ZipFile(pptx) as z:root,_,ordinals=project(z.read('ppt/slides/slide1.xml'),True)
    expected=np.full((300,400,4),255,dtype=np.uint8);mask=np.ones((300,400),dtype=bool)
    def polygon(points,color):
        if points[0]==points[-1]:points=points[:-1]
        pts=[(float(x),float(y)) for x,y in points]
        inside=np.zeros((300,400),dtype=bool)
        for (x0,y0),(x1,y1) in zip(pts,pts[1:]+pts[:1]):
            dx,dy=x1-x0,y1-y0;length=dx*dx+dy*dy
            if length==0:continue
            t=np.clip(((xx-x0)*dx+(yy-y0)*dy)/length,0,1)
            mask[:]&=(xx-x0-t*dx)**2+(yy-y0-t*dy)**2>4
            if dy:inside[:]^=((y0>yy)!=(y1>yy))&(xx<dx*(yy-y0)/dy+x0)
        expected[inside]=color
    by_object={page['bindings'][t['binding']]['location']['object']:t for t in plan['texts']}
    draw_sources={(s['textBinding'],s['glyph']):s for s in plan['textSources']}
    for shape in root.findall('.//p:sp',NS):
        native_id=int(shape.find('p:nvSpPr/p:cNvPr',NS).get('id'))
        t=by_object[native_id];frame=t['frame'];binding=plan['texts'].index(t)
        apply=transform(shape);extent=shape.find('p:spPr/a:xfrm/a:ext',NS)
        w,h=[F(extent.get(k)) for k in ['cx','cy']]
        polygon([apply(p) for p in [(0,0),(w,0),(w,h),(0,h)]],rgb(shape.find('p:spPr/a:solidFill/*',NS)))
        p=shape.find('p:txBody/a:p',NS);assert len(frame['paragraphs'])==1
        assert frame['inputs'][0]['sourceOrdinal']==ordinals[p]
        runcolors=[];ranges=[];start=0
        fontref=shape.find('p:style/a:fontRef/*',NS)
        for node in p.findall('a:r',NS):
            text=node.find('a:t',NS).text or '';end=start+len(text);ranges.append((start,end))
            fill=node.find('a:rPr/a:solidFill/*',NS)
            if node.find('a:rPr/a:noFill',NS) is not None:color=None
            else:color=rgb(fill if fill is not None else fontref,fontref)
            runcolors.append(color);start=end
        r=frame['paragraphs'][0]['computed'];source=frame['inputs'][0]
        # All owned corpus frames use one natural line, zero native insets,
        # no margins, explicit 30pt paragraph font and top/left alignment.
        assert all(shape.find('p:txBody/a:bodyPr',NS).get(k)=='0' for k in ['lIns','rIns','tIns','bIns'])
        assert int(shape.find('p:txBody/a:lstStyle/a:lvl1pPr/a:defRPr',NS).get('sz'))*127==int(source['geometry'][0]['fontSize'])
        q=dict(shaping=dict(paragraph=dict(fonts=manifest['fonts'],styles=[dict(candidates=[b['candidate']]) for b in r['bindings']])),
               styles=source['geometry'],strutStyle=source['endStyle'],spacing=dict(kind='natural'))
        geometry=r['geometry']['paths']['layout']['geometry'];positions=reference(q,geometry,False)
        assert len(positions['lines'])==1
        scene=r['geometry']['paths']['scene']
        for i,(g,pos) in enumerate(zip(scene['glyphs'],positions['lines'][0]['glyphs'],strict=True)):
            fragment=geometry['shaping']['fallback']['items'][pos['source']['fallbackItem']]['fragments'][pos['source']['fragment']]
            glyph=fragment['shaped']['runs'][0]['glyphs'][pos['glyph']]
            start=glyph['cluster'];end=min([g['cluster'] for g in fragment['shaped']['runs'][0]['glyphs'] if g['cluster']>start]+[fragment['end']])
            owners=[j for j,(a,b) in enumerate(ranges) if a<end and b>start]
            assert owners and all(runcolors[j]==runcolors[owners[0]] for j in owners)
            color=runcolors[owners[0]]
            path=scene['paths'][g['path']]
            pen=RecordingPen();glyphset[font.getGlyphName(glyph['glyphId'])].draw(pen)
            points=[]
            for op,values in pen.value:
                assert op in ['moveTo','lineTo','closePath']
                points.extend((F(x)*int(path['fontSize'])/1000,-F(y)*int(path['fontSize'])/1000) for x,y in values)
            got=[(F(int(c['to']['x']),2**32),F(int(c['to']['y']),2**32)) for c in path['commands'] if 'to' in c]
            if got and got[0]==got[-1]:got=got[:-1]
            assert len(got)==len(points)
            assert all(abs(a-b)<=F(1,2**32) for p0,p1 in zip(got,points) for a,b in zip(p0,p1))
            world=[apply((x+pos['x'],y+pos['y'])) for x,y in points]
            if color is not None and points:
                actual_source=draw_sources[(binding,i)];instance=page['raster']['scene']['instances'][actual_source['instance']]
                assert instance['brush']==dict(kind='solid',rgba=color)
                affine=page['raster']['scene']['transforms'][instance['transform']]['affine']
                linear=[D(v)/U for v in affine['linear']];offset=[D(affine['translation'][k])/U for k in ['x','y']]
                bound=D(page['downstreamCoordinateErrorBound'])/U
                for (x,y),want in zip(got,world):
                    xy=[decimal(x),decimal(y)]
                    for axis in range(2):
                        computed=(sum(linear[2*axis+k]*xy[k] for k in range(2))+offset[axis])/4000
                        assert abs(computed-want[axis])<=bound+D('1e-80'),(pptx.name,computed,want[axis],bound)
                        counts['worldControlCoordinates']+=1
                polygon(world,color)
            else:assert (binding,i) not in draw_sources
            counts['glyphs']+=1
        counts['objects']+=1
    raw=pptx.with_suffix('.rgba');image=np.frombuffer(raw.read_bytes(),dtype=np.uint8).reshape(300,400,4)
    difference=np.any(image!=expected,axis=2)&mask
    assert not difference.any(),(pptx.name,int(difference.sum()))
    png=pptx.with_suffix('.png');Image.fromarray(image).save(png)
    counts['interiorPixels']+=int(mask.sum());counts['excludedEdgePixels']+=int((~mask).sum());counts['packages']+=1
    records.append(dict(source=entry(pptx),result=entry(result),pixels=entry(raw),preview=entry(png),interiorPixels=int(mask.sum())))
assert counts['packages']==10 and counts['worldControlCoordinates']>0
report=dict(format='musteroffice.source-text-page-native-reference/1',scope=__doc__,counts=counts,cases=records,
            referenceEnvironment=dict(python=platform.python_version(),numpy=np.__version__,fontTools=fontTools.__version__),
            executable=entry(args.executable),executionLog=entry(ROOT/'reference-native.log'),
            fontInputs=[entry(manifest_path),entry('fixtures/fonts/owned.ttf')])
(ROOT/'reference.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(counts))
