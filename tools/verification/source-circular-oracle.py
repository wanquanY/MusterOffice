"""Independent source XML -> exact-rational plot geometry -> Decimal curve oracle.
Usage: python3 source-circular-oracle.py CONFIG_JSON PARITY_REPORT_JSON
The source radius and source angles are kept exact, never taken from output IR.
"""
from decimal import Decimal as D, localcontext
from fractions import Fraction as F
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import posixpath
import sys
import xml.etree.ElementTree as ET
import zipfile

spec=importlib.util.spec_from_file_location('circle_math',Path(__file__).with_name('chart-geometry-oracle.py'))
math_oracle=importlib.util.module_from_spec(spec)
spec.loader.exec_module(math_oracle)
C='{http://schemas.openxmlformats.org/drawingml/2006/chart}'
P='{http://schemas.openxmlformats.org/presentationml/2006/main}'
A='{http://schemas.openxmlformats.org/drawingml/2006/main}'
R='{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'
U=2**32
PALETTE=[bytes(c) for c in [[41,91,160,255],[19,166,179,255],[240,163,49,255],[177,80,151,255]]]


def rounded(value):
    assert value>=0
    return (2*value.numerator+value.denominator)//(2*value.denominator)


def compile_source(source,request):
    with zipfile.ZipFile(source) as z:
        assert z.testzip() is None
        part=request['object']['part'].lstrip('/')
        slide=ET.fromstring(z.read(part))
        frames=[n for n in slide.iter(P+'graphicFrame') if int(n.find(P+'nvGraphicFramePr/'+P+'cNvPr').get('id'))==request['object']['nativeId']]
        assert len(frames)==1
        rel_id=frames[0].find(A+'graphic/'+A+'graphicData/'+C+'chart').get(R+'id')
        folder=posixpath.dirname(part)
        rels=ET.fromstring(z.read(folder+'/_rels/'+posixpath.basename(part)+'.rels'))
        rel=[r for r in rels if r.get('Id')==rel_id][0]
        assert rel.get('TargetMode')!='External'
        chart=posixpath.normpath(posixpath.join(folder,rel.get('Target'))).lstrip('/')
        raw=z.read(chart);root=ET.fromstring(raw);elements=list(root.iter());plot=elements[request['plotSourceOrdinal']]
        assert plot.tag in [C+'pieChart',C+'doughnutChart']
        angle=F(plot.find(C+'firstSliceAng').get('val'))/360
        hole=F(0) if plot.tag==C+'pieChart' else F(plot.find(C+'holeSize').get('val').rstrip('%'))/100
        series=sorted(plot.findall(C+'ser'),key=lambda s:int(s.find(C+'order').get('val')))
        return chart,hashlib.sha256(raw).hexdigest(),elements,series,angle,hole


def verify(config,report,report_path):
    tau=2*(16*math_oracle.atan_inverse(5)-4*math_oracle.atan_inverse(239))
    records=series_count=points_count=curves_count=samples=probes=0
    for case,record in zip(config['cases'],report['cases'],strict=True):
        assert case['name']==record['name']
        source=Path(case['source']);raw=Path(case['request']).read_bytes();request=json.loads(raw)
        assert hashlib.sha256(source.read_bytes()).hexdigest()==record['sourceSha256']
        assert hashlib.sha256(raw).hexdigest()==record['requestSha256']
        if record['response']['status']!='compiled':continue
        chart,sha,elements,series,angle,hole=compile_source(source,request)
        result=record['response']['geometry'];assert result['chartPart']=='/'+chart;assert result['chartSha256']==sha
        assert result['dataAuthority']=='sourceCacheSnapshot';assert result['object']==request['object']
        assert result['firstSliceDegrees']==angle*360;assert result['holePercent']==hole*100
        radius=F(int(request['outerRadius']),U);center=math_oracle.point(request['center']);records+=1
        pixels=(report_path.parent/(case['name']+'.rgba')).read_bytes() if record.get('raster') else None
        for order,(s,out) in enumerate(zip(series,result['series'],strict=True)):
            series_count+=1
            assert out['index']==int(s.find(C+'idx').get('val'));assert out['order']==int(s.find(C+'order').get('val'))
            assert out['sourceOrdinal']==elements.index(s)
            channel=s.find(C+'val');assert out['valuesSourceOrdinal']==elements.index(channel)
            ref=channel.find(C+'numRef');cache=ref.find(C+'numCache') if ref is not None else channel.find(C+'numLit')
            formula=ref.find(C+'f').text if ref is not None else None;assert out['formula']==formula
            values=sorted(cache.findall(C+'pt'),key=lambda p:int(p.get('idx')))
            numbers=[abs(F(p.find(C+'v').text)) for p in values];total=sum(numbers)
            inner=radius*(hole+(1-hole)*F(order,len(series)))
            outer=radius*(hole+(1-hole)*F(order+1,len(series)))
            assert int(out['innerRadius'])==rounded(inner*U);assert int(out['outerRadius'])==rounded(outer*U)
            source_error=F(int(out['sourceGeometryErrorBound']),U)
            sectors=out['geometry']['layout']['sectors']
            actual_start=F(int(sectors[0]['startTurn']),U) if sectors else angle%1
            radial_error=max(abs(inner-F(int(out['innerRadius']),U)),abs(outer-F(int(out['outerRadius']),U)))
            minimum_error=math_oracle.decimal(abs(actual_start-angle%1)*radius)*tau+math_oracle.decimal(radial_error)
            assert math_oracle.decimal(source_error)+D('1e-75')>=minimum_error
            assert int(out['coordinateErrorBound'])==int(out['sourceGeometryErrorBound'])+max((int(p['coordinateErrorBound']) for p in out['geometry']['paths']),default=0)
            assert int(out['coordinateErrorBound'])<=int(request['coordinateTolerance'])
            before=F(0)
            for node,number,binding,path in zip(values,numbers,out['points'],out['geometry']['paths'],strict=True):
                points_count+=1;index=int(node.get('idx'));assert index==path['pointIndex']==binding['index'];assert elements.index(node)==binding['sourceOrdinal']
                if number==0:
                    assert path['commands']==[];continue
                first=angle+before/total;before+=number;last=angle+before/total
                arcs=[];current=[];pen=None
                for c in path['commands']:
                    if c['kind']=='move':assert not current;pen=math_oracle.point(c['to'])
                    elif c['kind']=='cubic':
                        current.append([pen,math_oracle.point(c['control1']),math_oracle.point(c['control2']),math_oracle.point(c['to'])]);pen=current[-1][-1]
                    elif c['kind'] in ['line','close']:
                        if current:arcs.append(current);current=[]
                        if c['kind']=='line':pen=math_oracle.point(c['to'])
                    else:raise AssertionError(c['kind'])
                assert not current;assert len(arcs)==(2 if inner else 1)
                bound=math_oracle.decimal(F(int(path['coordinateErrorBound']),U)+source_error)
                assert sum(map(len,arcs))==path['arcSegments']
                assert int(path['coordinateErrorBound'])==sum(int(path[k]) for k in ['numericErrorBound','curveErrorBound','angularErrorBound'])
                for i,arc in enumerate(arcs):
                    a,b=(first,last) if i==0 else (last,first);r=math_oracle.decimal(outer if i==0 else inner)
                    for j,controls in enumerate(arc):
                        curves_count+=1
                        for sample in range(9):
                            t=D(sample)/8;fraction=(D(j)+t)/len(arc);theta=(math_oracle.decimal(a)+math_oracle.decimal(b-a)*fraction)*tau
                            sine,cosine=math_oracle.sin_cos(theta,tau);exact=[center[0]+r*sine,center[1]-r*cosine];actual=math_oracle.curve(controls,t)
                            assert all(abs(x-y)<=bound+D('1e-75') for x,y in zip(exact,actual)),(case['name'],order,index,i,j,sample)
                            samples+=1
                if pixels:
                    theta=float((first+last)/2)*math.tau;r=float((inner+outer)/2)/5
                    x=int(256+r*math.sin(theta));y=int(256-r*math.cos(theta));actual=pixels[(512*y+x)*4:(512*y+x)*4+4]
                    assert actual==PALETTE[index%4],(case['name'],order,index,actual);probes+=1
        if pixels:
            for x,y in [(0,0),(511,511)]+([(256,256)] if hole else []):
                assert pixels[(512*y+x)*4+3]==0;probes+=1
    return dict(profile='source-circular-independent-xml-decimal-oracle/1',sourcePlots=records,series=series_count,points=points_count,cubics=curves_count,sampledPoints=samples,interiorColorAndHoleProbes=probes,decimalPrecision=90,originalXmlAnglesAndRadii=True,allBoundsEncloseSamples=True)


if __name__=='__main__':
    with localcontext() as context:
        context.prec=90
        report_path=Path(sys.argv[2])
        print(json.dumps(verify(json.loads(Path(sys.argv[1]).read_text()),json.loads(report_path.read_text()),report_path)))
