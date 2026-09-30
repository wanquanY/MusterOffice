"""Create owned PPTX circular-plot calculation fixtures, not Office screenshots.
Usage: python3 source-circular-fixtures.py NEW_DIRECTORY
"""
import hashlib
import json
from pathlib import Path
import sys
from xml.sax.saxutils import escape
import zipfile

P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
C = 'http://schemas.openxmlformats.org/drawingml/2006/chart'
R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
OPC = 'http://schemas.openxmlformats.org/package/2006/relationships'
U = 2**32


def series(index, order, values, point_order=None):
    points = ''.join(f'<c:pt idx="{i}"><c:v>{escape(values[i])}</c:v></c:pt>' for i in (point_order or range(len(values))))
    return f'<c:ser><c:idx val="{index}"/><c:order val="{order}"/><c:spPr><a:solidFill><a:srgbClr val="2468AC"/></a:solidFill></c:spPr><c:val><c:numLit><c:ptCount val="{len(values)}"/>{points}</c:numLit></c:val></c:ser>'


def chart(series_xml, angle='17', hole='50', pie=False):
    kind = 'pieChart' if pie else 'doughnutChart'
    return f'<c:chartSpace xmlns:c="{C}" xmlns:a="{A}" xmlns:r="{R}"><c:chart><c:plotArea><c:{kind}>{series_xml}<c:firstSliceAng val="{angle}"/>' + ('' if pie else f'<c:holeSize val="{hole}"/>') + f'</c:{kind}></c:plotArea></c:chart></c:chartSpace>'


def package(path, chart_xml):
    types = {'/ppt/presentation.xml': 'application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml', '/ppt/slides/slide1.xml': 'application/vnd.openxmlformats-officedocument.presentationml.slide+xml', '/ppt/charts/chart1.xml': 'application/vnd.openxmlformats-officedocument.drawingml.chart+xml'}
    parts = {
        '[Content_Types].xml': '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>' + ''.join(f'<Override PartName="{n}" ContentType="{t}"/>' for n,t in types.items()) + '</Types>',
        '_rels/.rels': f'<Relationships xmlns="{OPC}"><Relationship Id="main" Type="{R}/officeDocument" Target="ppt/presentation.xml"/></Relationships>',
        'ppt/presentation.xml': f'<p:presentation xmlns:p="{P}" xmlns:r="{R}"><p:sldIdLst><p:sldId id="256" r:id="slide"/></p:sldIdLst><p:sldSz cx="9144000" cy="5143500"/></p:presentation>',
        'ppt/_rels/presentation.xml.rels': f'<Relationships xmlns="{OPC}"><Relationship Id="slide" Type="{R}/slide" Target="slides/slide1.xml"/></Relationships>',
        'ppt/slides/slide1.xml': f'<p:sld xmlns:p="{P}" xmlns:a="{A}" xmlns:c="{C}" xmlns:r="{R}"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="Owned circular chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="2000000"/></p:xfrm><a:graphic><a:graphicData uri="{C}"><c:chart r:id="chart"/></a:graphicData></a:graphic></p:graphicFrame></p:spTree></p:cSld></p:sld>',
        'ppt/slides/_rels/slide1.xml.rels': f'<Relationships xmlns="{OPC}"><Relationship Id="chart" Type="{R}/chart" Target="../charts/chart1.xml"/></Relationships>',
        'ppt/charts/chart1.xml': chart_xml,
    }
    with zipfile.ZipFile(path, 'x', zipfile.ZIP_DEFLATED) as z:
        for name, text in sorted(parts.items()):
            info = zipfile.ZipInfo(name, (2026,1,1,0,0,0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, text.encode())


def main():
    root = Path(sys.argv[1]).resolve()
    root.mkdir(parents=True, exist_ok=False)
    cases = []
    def add(name, xml, code=None, changes=None, render=False):
        source = root / (name + '.pptx')
        package(source, xml)
        q = dict(expectedSourceSha256=hashlib.sha256(source.read_bytes()).hexdigest(), object=dict(part='/ppt/slides/slide1.xml', nativeId=2), plotSourceOrdinal=3, profile='source-cache-declared-circular-plot-v1-draft', center=dict(x='0', y='0'), outerRadius=str(1000*U), coordinateTolerance=str(1<<24), negativeWeights='reject')
        q.update(changes or {})
        request=root/(name+'.json');request.write_text(json.dumps(q,indent=2)+'\n')
        cases.append(dict(name=name,source=str(source),request=str(request),expectedStatus='error' if code else 'compiled',**({'expectedCode':code} if code else {}),render=render))
    single = series(7,0,['1','2','3'],[2,0,1])
    for angle in ['0','17','90','359','360']:
        add('angle-'+angle, chart(single,angle), render=angle in ['17','90'])
    for hole in ['10','33','50','90','33%']:
        add('hole-'+hole.replace('%','percent'),chart(single,hole=hole))
    for n in [1,2,3,5]:
        s=''.join(series(7+i*3,n-i-1,[str(i+1),'2','3'],[1,2,0]) for i in range(n))
        add('rings-'+str(n),chart(s),render=n in [2,3,5])
    add('fractional-radius',chart(single),changes={'outerRadius':str(1001*U+1)})
    add('pie',chart(single,pie=True),render=True)
    add('full-ring',chart(series(7,0,['1'])),render=True)
    add('zeros',chart(series(7,0,['0','-0.00'])))
    add('decimals',chart(series(7,0,['1.20','2e-1','3.5678'])))
    negative=chart(series(7,0,['-1','0','2']))
    add('negative-magnitude',negative,changes={'negativeWeights':'absoluteMagnitude'})
    add('negative-rejected',negative,'INPUT_INVALID')
    base=chart(single)
    for name,old,new,code in [
        ('sparse','<c:pt idx="1"><c:v>2</c:v></c:pt>','','MAPPING_NOT_IMPLEMENTED'),
        ('blank','<c:v>2</c:v>','<c:v/>','MAPPING_NOT_IMPLEMENTED'),
        ('missing-value','<c:v>2</c:v>','','MAPPING_NOT_IMPLEMENTED'),
        ('error-value','<c:v>2</c:v>','<c:v>#N/A</c:v>','MAPPING_NOT_IMPLEMENTED'),
        ('no-count','<c:ptCount val="3"/>','','MAPPING_NOT_IMPLEMENTED'),
        ('huge-count','<c:ptCount val="3"/>','<c:ptCount val="1000000000"/>','LIMIT_EXCEEDED'),
        ('no-angle','<c:firstSliceAng val="17"/>','','MAPPING_NOT_IMPLEMENTED'),
        ('empty-angle','<c:firstSliceAng val="17"/>','<c:firstSliceAng/>','MAPPING_NOT_IMPLEMENTED'),
        ('bad-angle','<c:firstSliceAng val="17"/>','<c:firstSliceAng val="361"/>','MAPPING_NOT_IMPLEMENTED'),
        ('small-hole','<c:holeSize val="50"/>','<c:holeSize val="9"/>','MAPPING_NOT_IMPLEMENTED'),
        ('no-hole','<c:holeSize val="50"/>','','MAPPING_NOT_IMPLEMENTED'),
        ('duplicate-angle','<c:firstSliceAng val="17"/>','<c:firstSliceAng val="17"/><c:firstSliceAng val="0"/>','SOURCE_CONFLICT'),
        ('exploded-series','</c:ser>','<c:explosion val="10"/></c:ser>','MAPPING_NOT_IMPLEMENTED'),
        ('exploded-point','</c:ser>','<c:dPt><c:idx val="0"/><c:explosion val="10"/></c:dPt></c:ser>','MAPPING_NOT_IMPLEMENTED'),
        ('unknown-geometry','</c:ser>','<c:unknownGeometry/></c:ser>','MAPPING_NOT_IMPLEMENTED'),
        ('extension','</c:ser>','<c:extLst><c:ext uri="owned"/></c:extLst></c:ser>','MAPPING_NOT_IMPLEMENTED'),
        ('root-extension','</c:chartSpace>','<c:extLst><c:ext uri="owned"/></c:extLst></c:chartSpace>','MAPPING_NOT_IMPLEMENTED'),
        ('duplicate-point','</c:ser>','<c:dPt><c:idx val="1"/></c:dPt><c:dPt><c:idx val="1"/></c:dPt></c:ser>','SOURCE_CONFLICT'),
    ]:
        add(name,base.replace(old,new),code)
    add('stale-pin',base,'SOURCE_CONFLICT',{'expectedSourceSha256':'0'*64})
    add('wrong-object',base,'SOURCE_CONFLICT',{'object':{'part':'/ppt/slides/slide1.xml','nativeId':99}})
    add('wrong-plot',base,'SOURCE_CONFLICT',{'plotSourceOrdinal':4})
    add('tiny-tolerance',base,'MAPPING_NOT_IMPLEMENTED',{'coordinateTolerance':'1'})
    (root/'cases.json').write_text(json.dumps(cases,indent=2)+'\n')
    print(json.dumps({'cases':len(cases),'expectedCompiled':sum(c['expectedStatus']=='compiled' for c in cases)}))


if __name__=='__main__':
    main()
