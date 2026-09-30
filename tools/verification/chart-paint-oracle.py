"""Read original ZIP/XML and independently resolve the exercised chart colors.

Usage: python3 chart-paint-oracle.py config.json parity.json
Only the explicitly implemented color operations below are accepted by this
oracle; an unknown operation fails the verifier, never becomes a default color.
"""
from pathlib import Path
from fractions import Fraction
import colorsys
import hashlib
import json
import math
import posixpath
import sys
import xml.etree.ElementTree as ET
import zipfile

A='{http://schemas.openxmlformats.org/drawingml/2006/main}'
C='{http://schemas.openxmlformats.org/drawingml/2006/chart}'
P='{http://schemas.openxmlformats.org/presentationml/2006/main}'
R='{http://schemas.openxmlformats.org/officeDocument/2006/relationships}'


def relationships(z,part):
    name=posixpath.dirname(part)+'/_rels/'+posixpath.basename(part)+'.rels'
    if name not in z.namelist():return []
    return [(r.get('Id'),r.get('Type').split('/')[-1],None if r.get('TargetMode')=='External' else posixpath.normpath(posixpath.join(posixpath.dirname(part),r.get('Target'))).lstrip('/')) for r in ET.fromstring(z.read(name))]


def source(z,query):
    part=query['object']['part'].lstrip('/');slide=ET.fromstring(z.read(part))
    frame=next(f for f in slide.iter(P+'graphicFrame') if int(f.find(P+'nvGraphicFramePr/'+P+'cNvPr').get('id'))==query['object']['nativeId'])
    rid=frame.find(A+'graphic/'+A+'graphicData/'+C+'chart').get(R+'id')
    chart_part=next(t for i,k,t in relationships(z,part) if i==rid and k=='chart');assert chart_part
    root=ET.fromstring(z.read(chart_part));parents=[part]
    for kind in ['slideLayout','slideMaster']:
        target=next((t for _,k,t in relationships(z,parents[-1]) if k==kind),None)
        if target:parents.append(target)
        else:break
    mapping=None;master_mapping=None;scheme=None;scheme_part=None
    for parent in reversed(parents):
        view=ET.fromstring(z.read(parent))
        direct=view.find(P+'clrMap')
        if direct is not None:mapping=dict(direct.attrib);master_mapping=mapping
        override=view.find(P+'clrMapOvr')
        if override is not None:
            explicit=override.find(A+'overrideClrMapping')
            if explicit is not None:mapping=dict(explicit.attrib)
            elif override.find(A+'masterClrMapping') is not None:mapping=master_mapping
        for kind in ['theme','themeOverride']:
            target=next((t for _,k,t in relationships(z,parent) if k==kind),None)
            if target:
                theme=ET.fromstring(z.read(target));colors=theme.find('.//'+A+'clrScheme')
                if colors is not None:scheme=colors;scheme_part=target
    explicit=root.find(C+'clrMapOvr')
    if explicit is not None:mapping=dict(explicit.attrib)
    target=next((t for _,k,t in relationships(z,chart_part) if k=='themeOverride'),None)
    if target:
        colors=ET.fromstring(z.read(target)).find(A+'clrScheme')
        if colors is not None:scheme=colors;scheme_part=target
    return chart_part,root,mapping,scheme,scheme_part


def percent(text):return float(Fraction(text[:-1])/100 if text.endswith('%') else Fraction(text)/100000)


def evaluate(node,mapping,scheme,context,depth=0):
    assert depth<16
    kind=node.tag.removeprefix(A)
    if kind=='srgbClr':
        value=node.get('val');rgb=[int(value[i:i+2],16)/255 for i in [0,2,4]];alpha=1
    elif kind=='sysClr':
        value=context['systemColors'].get(node.get('val'))
        if value is None:
            fallback=node.get('lastClr')
            if fallback is None:return None
            value=[int(fallback[i:i+2],16) for i in [0,2,4]]
        rgb=[v/255 for v in value];alpha=1
    elif kind=='schemeClr':
        slot=node.get('val')
        if slot=='phClr':
            value=context.get('placeholder')
            if value is None:return None
            rgb=[v/255 for v in value[:3]];alpha=value[3]/255
        else:
            if scheme is None:return None
            if slot not in ['dk1','dk2','lt1','lt2']:
                if mapping is None:return None
                slot=mapping[slot]
            child=scheme.find(A+slot)
            if child is None:return None
            result=evaluate(child[0],mapping,scheme,context,depth+1)
            if result is None:return None
            rgb=result[:3];alpha=result[3]
    else:raise AssertionError('unimplemented oracle color '+kind)
    for t in node:
        kind=t.tag.removeprefix(A);value=percent(t.get('val'))
        if kind=='alpha':alpha=value
        elif kind=='redMod':rgb[0]*=value
        elif kind in ['lumMod','lumOff']:
            h,l,s=colorsys.rgb_to_hls(*rgb);l=l*value if kind=='lumMod' else l+value
            rgb=list(colorsys.hls_to_rgb(h,max(0,min(1,l)),s))
        else:raise AssertionError('unimplemented oracle transform '+kind)
    return rgb+[alpha]


def verify(config,report,base):
    charts=declarations=colors=probes=0
    for case,record in zip(config['cases'],report['cases'],strict=True):
        assert case['name']==record['name'];raw=Path(case['request']).read_bytes();query=json.loads(raw);data=Path(case['source']).read_bytes()
        assert hashlib.sha256(raw).hexdigest()==record['requestSha256'];assert hashlib.sha256(data).hexdigest()==record['sourceSha256']
        if record['response']['status']!='computed':continue
        with zipfile.ZipFile(case['source']) as z:
            assert z.testzip() is None
            part,root,mapping,scheme,scheme_part=source(z,query);nodes=list(root.iter());paints=record['response']['paints']
            assert paints['chart']['part']=='/'+part;assert paints['chart']['sha256']==hashlib.sha256(z.read(part)).hexdigest();charts+=1
            assert (paints['colorScheme']['part'] if paints['colorScheme'] else None)==('/'+scheme_part if scheme_part else None)
            if scheme_part:
                theme_root=ET.fromstring(z.read(scheme_part));theme_colors=theme_root.find('.//'+A+'clrScheme')
                assert paints['colorScheme']['sourceOrdinal']==list(theme_root.iter()).index(theme_colors)
            expected_nodes=[n for n in nodes if n.tag==C+'spPr'];assert len(expected_nodes)==len(paints['declarations'])
            for decl,node in zip(paints['declarations'],expected_nodes,strict=True):
                assert nodes.index(node)==decl['sourceOrdinal'];declarations+=1
                color_roots=[]
                for parent in [node,node.find(A+'ln')]:
                    if parent is None:continue
                    for child in parent:
                        if child.tag in [A+'solidFill',A+'gradFill',A+'pattFill']:color_roots.append(child)
                expected_colors=[n for root in color_roots for n in root.iter() if n.tag in [A+k for k in ['srgbClr','scrgbClr','hslClr','sysClr','schemeClr','prstClr']]]
                assert [nodes.index(n) for n in expected_colors]==[c['sourceOrdinal'] for c in decl['colors']]
                for color in decl['colors']:
                    value=evaluate(nodes[color['sourceOrdinal']],mapping,scheme,query['context']);out=color['outcome'];colors+=1
                    if value is None:assert out['status']=='unresolved';continue
                    assert out['status']=='resolved'
                    assert all(abs(x-y)<2e-14 for x,y in zip(value,out['srgb'],strict=True)),(case['name'],value,out['srgb'])
                    assert out['rgba8']==[int(max(0,min(1,v))*255+0.5) for v in value]
                    assert out['rgba16']==[int(max(0,min(1,v))*65535+0.5) for v in value]
            if diagnostic:=record.get('fillDiagnostic'):
                pixels=(base/(case['name']+'.rgba')).read_bytes();assert hashlib.sha256(pixels).hexdigest()==diagnostic['pixelSha256']
                plot=root.find('.//'+C+'doughnutChart');series=plot.find(C+'ser');values=series.find(C+'val').find('.//'+C+'numCache');points=sorted(values.findall(C+'pt'),key=lambda p:int(p.get('idx')))
                numbers=[Fraction(p.find(C+'v').text) for p in points];total=sum(numbers);angle=Fraction(plot.find(C+'firstSliceAng').get('val'))/360;before=Fraction(0)
                for node,number in zip(points,numbers,strict=True):
                    idx=int(node.get('idx'));point=next(p for p in series.findall(C+'dPt') if int(p.find(C+'idx').get('val'))==idx);clr=point.find(C+'spPr/'+A+'solidFill')[0]
                    value=evaluate(clr,mapping,scheme,query['context']);expected=bytes(int(v*255+0.5) for v in value)
                    theta=float(angle+(before+number/2)/total)*math.tau;before+=number;x=int(256+150*math.sin(theta));y=int(256-150*math.cos(theta))
                    assert pixels[(512*y+x)*4:(512*y+x)*4+4]==expected;probes+=1
                for x,y in [(0,0),(256,256),(511,511)]:assert pixels[(512*y+x)*4+3]==0;probes+=1
    return dict(profile='source-chart-independent-xml-color-oracle/1',charts=charts,shapeDeclarations=declarations,declaredColors=colors,sourceColorPixelProbes=probes,allMatched=True)


if __name__=='__main__':
    config=json.loads(Path(sys.argv[1]).read_text());file=Path(sys.argv[2]);print(json.dumps(verify(config,json.loads(file.read_text()),file.parent)))
