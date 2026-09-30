"""Independently follow original ZIP/XML point/series styles and probe actual RGBA.

This does not import Rust outputs as reference colors, widths or source ordinals.
The narrow declared-solid profile is checked; no automatic chart styles assumed.
"""
from pathlib import Path
from fractions import Fraction
import hashlib
import importlib.util
import json
import math
import sys
import zipfile

spec=importlib.util.spec_from_file_location('chart_colors',Path(__file__).with_name('chart-paint-oracle.py'))
color=importlib.util.module_from_spec(spec);spec.loader.exec_module(color)
A,C=color.A,color.C


def paint(layers,nodes,mapping,scheme,context):
    selected=None;expression=None
    for layer in layers:
        if layer is None:continue
        declarations=[n for n in layer if n.tag in [A+k for k in ['noFill','solidFill','gradFill','pattFill','blipFill','grpFill']]]
        assert len(declarations)<=1
        if not declarations:continue
        node=declarations[0]
        if selected is None:selected=node
        if selected.tag==A+'noFill':return dict(kind='none',declaration=nodes.index(selected))
        assert selected.tag==A+'solidFill','only solid profile is admitted'
        if node.tag==selected.tag and len(node):expression=node[0];break
    assert expression is not None
    value=color.evaluate(expression,mapping,scheme,context);assert value is not None
    return dict(kind='solid',declaration=nodes.index(selected),color=nodes.index(expression),rgba8=[int(max(0,min(1,v))*255+0.5) for v in value])


def expected_stroke(lines,nodes,part):
    def origin(n):
        return {'kind':'chart','part':'/'+part,'sourceOrdinal':nodes.index(n)} if n is not None else {'kind':'profileDefault'}
    def attribute(name,default):
        node=next((n for n in lines if n is not None and n.get(name) is not None),None)
        return {'value':node.get(name) if node is not None else default,'declaredBy':origin(node)}
    attrs={key:attribute(name,default) for key,name,default in [
        ('width','w','9525'),('cap','cap','flat'),('compound','cmpd','sng'),('alignment','algn','ctr')]}
    attrs['width']['value']=str(int(attrs['width']['value']))
    join=next((child for n in lines if n is not None for child in n if child.tag in [A+'round',A+'bevel',A+'miter']),None)
    attrs['join']={'kind':join.tag.removeprefix(A) if join is not None else 'round','declaredBy':origin(join)}
    dash=None;preset=None
    for n in lines:
        if n is None:continue
        d=next((c for c in n if c.tag in [A+'prstDash',A+'custDash']),None)
        if d is None:continue
        if dash is None:dash=d
        if dash.tag==A+'prstDash' and d.tag==dash.tag and d.get('val') is not None:preset=d;break
    assert dash is None or dash.tag==A+'prstDash'
    attrs['dash']={'kind':'preset','declaredBy':origin(dash),'value':{'value':preset.get('val') if preset is not None else 'solid','declaredBy':origin(preset)}}
    for side in ['head','tail']:
        entries=[n.find(A+side+'End') for n in lines if n is not None]
        entry=next((n for n in entries if n is not None),None)
        attrs[side]={'declaredBy':origin(entry)}
        for key,name,default in [('kind','type','none'),('width','w','med'),('length','len','med')]:
            selected=next((n for n in entries if n is not None and n.get(name) is not None),None)
            attrs[side][key]={'value':selected.get(name) if selected is not None else default,'declaredBy':origin(selected)}
    return attrs


def verify(config,report,base):
    charts=points=draws=probes=stroke_probes=0
    for case,record in zip(config['cases'],report['cases'],strict=True):
        assert case['name']==record['name']
        query=json.loads(Path(case['request']).read_text())
        assert hashlib.sha256(Path(case['source']).read_bytes()).hexdigest()==record['sourceSha256']
        if record['response']['status']!='compiled':continue
        result=record['response']['plot'];actual_draws=result['scene']['instances']
        expected_draws=[];expected_paths=0
        pixels=(base/(case['name']+'.rgba')).read_bytes()
        assert hashlib.sha256(pixels).hexdigest()==record['raster']['pixelSha256']
        def pixel(x,y):return list(pixels[(512*y+x)*4:(512*y+x)*4+4])
        with zipfile.ZipFile(case['source']) as z:
            part,root,mapping,scheme,_=color.source(z,query['geometry']);nodes=list(root.iter())
            assert result['chartPart']=='/'+part
            assert result['chartSha256']==hashlib.sha256(z.read(part)).hexdigest()
            assert result['sourceSha256']==record['sourceSha256']
            plot=nodes[query['geometry']['plotSourceOrdinal']]
            assert plot.tag in [C+'pieChart',C+'doughnutChart'];charts+=1
            theta0=Fraction(plot.find(C+'firstSliceAng').get('val'))/360
            source_series=sorted(plot.findall(C+'ser'),key=lambda s:int(s.find(C+'order').get('val')))
            assert len(source_series)==1,'this oracle tests single rings, not uncalibrated multi-ring layout'
            ser=source_series[0];idx=int(ser.find(C+'idx').get('val'));sp=ser.find(C+'spPr')
            cache=ser.find(C+'val').find('.//'+C+'numCache')
            if cache is None:cache=ser.find(C+'val/'+C+'numLit')
            values=sorted(cache.findall(C+'pt'),key=lambda p:int(p.get('idx')))
            numbers=[Fraction(p.find(C+'v').text) for p in values];total=sum(numbers);before=Fraction(0)
            assert len(result['points'])==len(values)
            for p,node,value in zip(result['points'],values,numbers,strict=True):
                index=int(node.get('idx'));points+=1
                local=next((n.find(C+'spPr') for n in ser.findall(C+'dPt') if int(n.find(C+'idx').get('val'))==index),None)
                layers=[local,sp];lines=[n.find(A+'ln') if n is not None else None for n in layers]
                fill=paint(layers,nodes,mapping,scheme,query['colorContext'])
                line=paint(lines,nodes,mapping,scheme,query['colorContext'])
                geom=expected_stroke(lines,nodes,part)
                assert p['seriesIndex']==idx and p['pointIndex']==index and p['cacheSourceOrdinal']==nodes.index(node)
                assert p['fill']==fill and p['line']==line,(case['name'],index)
                assert p['lineGeometry']==geom,(case['name'],index,p['lineGeometry'],geom)
                effect=next((c for n in layers if n is not None for c in n if c.tag in [A+'effectLst',A+'effectDag']),None)
                assert effect is not None and effect.tag==A+'effectLst' and len(effect)==0
                assert p['effectsSourceOrdinal']==nodes.index(effect)
                if not value:
                    assert p['path'] is None
                    continue
                assert p['path']==expected_paths;expected_paths+=1
                for kind,paint_info in [('fill',fill),('line',line)]:
                    if paint_info['kind']=='none':continue
                    entry={'path':p['path'],'transform':None,'brush':{'kind':'solid','rgba':paint_info['rgba8']}}
                    if kind=='line':entry['stroke']={'width':str(int(geom['width']['value'])<<32),'cap':{'flat':'butt','rnd':'round','sq':'square'}[geom['cap']['value']],'join':{'kind':geom['join']['kind']}}
                    expected_draws.append(entry)
                theta=float(theta0+(before+value/2)/total)*math.tau;before+=value
                x=int(256+150*math.sin(theta));y=int(256-150*math.cos(theta))
                rgba=fill.get('rgba8',[0,0,0,0]);alpha=rgba[3]
                # Premultiplication is independently checked within one byte;
                # solid opaque colors/hole transparency require exact equality.
                expected=[int(c*alpha/255+0.5) for c in rgba[:3]]+[alpha]
                observed=pixel(x,y);assert max(abs(a-b) for a,b in zip(expected,observed))<=(1 if 0<alpha<255 else 0),(case['name'],index,observed,expected)
                probes+=1
                if line['kind']=='solid' and line['rgba8'][3]==255:
                    width=int(geom['width']['value'])/5000
                    # A pixel wholly inside the outer-arc stroke and away from
                    # the radial boundaries is an independent line-color probe.
                    cx=256+200*math.sin(theta);cy=256-200*math.cos(theta);found=False
                    for yy in range(int(cy)-2,int(cy)+3):
                        for xx in range(int(cx)-2,int(cx)+3):
                            radial=math.hypot(xx+.5-256,yy+.5-256)
                            if abs(radial-200)+math.sqrt(.5)<width/2:
                                assert pixel(xx,yy)==line['rgba8'],(case['name'],index,'stroke',xx,yy,pixel(xx,yy),line['rgba8'])
                                found=True;stroke_probes+=1
                    assert found,'no interior stroke probe'
            assert len(result['scene']['paths'])==expected_paths
            assert actual_draws==expected_draws,(case['name'],'draw order/paint/stroke mismatch')
            draws+=len(expected_draws)
            for x,y in [(0,0),(256,256),(511,511)]:
                assert pixel(x,y)[3]==0;probes+=1
    return dict(profile='source-chart-plot-independent-xml-oracle/1',charts=charts,points=points,draws=draws,fillAndHoleProbes=probes,strokePixelProbes=stroke_probes,allMatched=True)


if __name__=='__main__':
    config=json.loads(Path(sys.argv[1]).read_text());file=Path(sys.argv[2])
    print(json.dumps(verify(config,json.loads(file.read_text()),file.parent)))
