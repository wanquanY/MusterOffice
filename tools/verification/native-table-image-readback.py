"""Owned table images: independent XML/XSD, receiver and nearest-pixel checks.

This is not Office/WPS display or editable-roundtrip acceptance.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import zipfile
from lxml import etree

NS = {'a': 'http://schemas.openxmlformats.org/drawingml/2006/main',
      'p': 'http://schemas.openxmlformats.org/presentationml/2006/main',
      'r': 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'}
U = 1 << 32
COLORS = ((255,0,0,255),(0,255,255,255),(0,0,255,255),(255,255,0,255))

def receipt(p):
    b=p.read_bytes()
    return dict(path=str(p),sha256=hashlib.sha256(b).hexdigest(),byteLength=len(b))

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--corpus-dir',type=Path,required=True)
    p.add_argument('--schema-dir',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args();assert not a.output.exists()
    schema=etree.XMLSchema(etree.parse(str(a.schema_dir/'dml-main.xsd')))
    presentation=etree.XMLSchema(etree.parse(str(a.schema_dir/'pml.xsd')))
    cases=[];pixel_checks=0
    for path in sorted(a.corpus_dir.glob('*.pptx')):
        with zipfile.ZipFile(path) as z:
            root=etree.fromstring(z.read('ppt/slides/slide1.xml'))
            assert z.read('ppt/media/owned-image.png')==z.read('ppt/media/owned-copy.png')
            assert z.read('ppt/media/owned-image.png')!=z.read('ppt/media/owned-cyan.png')
        presentation.assertValid(root)
        obj=root.findall('p:cSld/p:spTree/p:graphicFrame',NS)
        assert len(obj)==1 and not root.findall('p:cSld/p:spTree/p:sp',NS)
        table=obj[0].find('a:graphic/a:graphicData/a:tbl',NS);schema.assertValid(table)
        fills=table.findall('.//a:blipFill',NS);assert len(fills)==4
        assert all(v.get('rotWithShape')=='1' and v.get('dpi')=='72' for v in fills)
        if path.name.startswith('playback-'):
            assert root.find('p:timing',NS) is not None
            if path.stem=='playback-reveal':
                assert obj[0].find('p:nvGraphicFramePr/p:cNvPr',NS).get('hidden')=='1'
                assert root.find('.//p:set/p:to/p:strVal[@val="visible"]',NS) is not None
            cases.append(dict(source=receipt(path),timing=True));continue
        widths=[int(v.get('w')) for v in table.findall('a:tblGrid/a:gridCol',NS)]
        rows=table.findall('a:tr',NS);heights=[int(v.get('h')) for v in rows]
        assert widths==[1828800]*3 and heights==[914400]*3
        rtl=table.find('a:tblPr',NS).get('rtl')=='1'
        plan=json.loads(path.with_suffix('.plan.json').read_bytes())
        query=json.loads(path.with_suffix('.request.json').read_bytes())
        pixels=path.with_suffix('.rgba').read_bytes();stride=query['viewport']['width']
        assert len(pixels)==stride*query['viewport']['height']*4
        assert len(plan['images']['decoded'])==2 and len(plan['images']['bindings'])==4
        assert plan['images']['gatherCopyBytes']==32
        checked=0
        for binding in plan['images']['bindings']:
            target=binding['layout']['target'];native=plan['page']['bindings'][binding['binding']]
            assert native['location']['object']==2 and native['fill']['target']==target
            if target['kind']=='tableBackground':
                x=y=0;w=sum(widths);h=sum(heights)
                fill=table.find('a:tblPr/a:blipFill',NS)
            else:
                assert target['kind']=='tableCell'
                row=target['cell']['row'];column=target['cell']['column'];cell=rows[row].findall('a:tc',NS)[column]
                cs=int(cell.get('gridSpan',1));rs=int(cell.get('rowSpan',1))
                x=sum(widths[column+cs:]) if rtl else sum(widths[:column]);y=sum(heights[:row])
                w=sum(widths[column:column+cs]);h=sum(heights[row:row+rs]);fill=cell.find('a:tcPr/a:blipFill',NS)
            assert fill is not None
            assert native['region']=={'bounds':{'min':{'x':str(x*U),'y':str(y*U)},'max':{'x':str((x+w)*U),'y':str((y+h)*U)}},'coordinateErrorBound':'0'}
            if target['kind']=='tableBackground':continue
            left,top,width,height=[v//12700 for v in (x,y,w,h)]
            # Keep away from native borders and centered test text. Expected
            # colors come from the four owned source pixels, not a render plan.
            for py in [top+2,top+10,top+height-7]:
                for px in range(left+2,left+width-2,3):
                    tile=fill.find('a:tile',NS)
                    if tile is not None:
                        assert tile.get('sx')==tile.get('sy')=='400000' and tile.get('flip')=='xy'
                        ix=math.floor((px+.5-left)/4)%4;iy=math.floor((py+.5-top)/4)%4
                        ix=ix if ix<2 else 3-ix;iy=iy if iy<2 else 3-iy
                        color=COLORS[iy*2+ix]
                    else:
                        crop=fill.find('a:srcRect',NS);rect=fill.find('a:stretch/a:fillRect',NS)
                        l=float(rect.get('l',0))/100000;r=float(rect.get('r',0))/100000
                        lx=left+width*l;rx=left+width*(1-r)
                        if not lx<=px+.5<rx:color=(0,255,255,255)
                        else:
                            source_w=2*(1-(float(crop.get('r',0))/100000 if crop is not None else 0))
                            ix=math.floor((px+.5-lx)/(rx-lx)*source_w)
                            iy=math.floor((py+.5-top)/height*2);color=COLORS[iy*2+ix]
                    at=(py*stride+px)*4
                    assert tuple(pixels[at:at+4])==color,(path.name,px,py,tuple(pixels[at:at+4]),color)
                    checked+=1
        pixel_checks+=checked
        cases.append(dict(source=receipt(path),plan=receipt(path.with_suffix('.plan.json')),pixels=receipt(path.with_suffix('.rgba')),independentImagePixels=checked))
    assert {Path(c['source']['path']).stem for c in cases}=={'images-rtl-false','images-rtl-true','playback-motion','playback-fade','playback-hidden','playback-reveal'}
    report=dict(status='passed',profile='musteroffice.native-table-image-readback/1',xsdSlides=len(cases),xsdTables=len(cases),independentImagePixels=pixel_checks,cases=cases,officeAccepted=False)
    a.output.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='cases'}))

if __name__=='__main__':main()
