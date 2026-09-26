"""Check runtime diagnostics against independent XML ordinals and wire schemas."""
import hashlib
import importlib.metadata
import json
from pathlib import Path
import zipfile
import jsonschema
from mce_reference import project, A, P

ROOT=Path('.codex-work/text-resource-diagnostics')
def entry(path):
    p=Path(path);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
def load(record):
    assert entry(record['path'])==record
    return Path(record['path']).read_bytes()
report=json.loads((ROOT/'parity.json').read_text())
ns={'a':A,'p':P};paint_runs=font_uses=0
def nodes(source,part):
    with zipfile.ZipFile(source) as z:
        return project(z.read(part.lstrip('/')),True)
for case in report['cases']:
    result=json.loads(load(case['response']))
    if result['status']!='error' or result['error']['stage']!='page':continue
    error=result['error'];location=error['error'].get('location')
    if not location:continue
    root,_,ordinals=nodes(case['source']['path'],location['part'])
    obj=root.xpath('.//p:sp[p:nvSpPr/p:cNvPr/@id=$id]',namespaces=ns,id=str(location['object']))[0]
    paragraphs=obj.findall('p:txBody/a:p',ns)
    if 'paintLocation' in error:
        at=error['paintLocation'];paragraph=paragraphs[at['paragraph']]
        runs=paragraph.xpath('a:r|a:br|a:fld',namespaces=ns);node=runs[at['run']]
        assert ordinals[node]==at['sourceOrdinal'],case['name'];paint_runs+=1
        if error['detail']['kind']=='paintProperty':assert node.find('a:rPr',ns).get('u')=='sng'
        if error['detail']['kind']=='missingPaint':assert node.find('a:rPr/a:solidFill',ns) is None
        if error['detail']['kind']=='color':assert node.find('a:rPr/a:solidFill/a:sysClr',ns).get('val')=='window'
    if error.get('detail',{}).get('kind')=='fontSelection':
        f=error['detail']['failure'];assert f['sourceSha256']==case['source']['sha256']
        assert f['object']==dict(part=location['part'],nativeId=location['object'])
        paragraph=paragraphs[f['paragraph']];assert ordinals[paragraph]==f['sourceOrdinal']
        runs=paragraph.xpath('a:r|a:br|a:fld',namespaces=ns)
        # Fixture defaults are explicit. Compute the requested slot from the
        # actual content/end properties, independently of Rust style indices.
        defaults=obj.find('p:txBody/a:lstStyle/a:lvl1pPr/a:defRPr',ns)
        for use in f['uses']:
            rpr=paragraph.find('a:endParaRPr',ns) if use['run'] is None else runs[use['run']].find('a:rPr',ns)
            def flag(name):
                value=rpr.get(name) if rpr is not None else None
                if value is None:value=defaults.get(name,'0')
                return value in ['1','true']
            style={ (False,False):'regular',(True,False):'bold',(False,True):'italic',(True,True):'boldItalic'}[(flag('b'),flag('i'))]
            assert style==f['selection']['fontStyle'],case['name']
            declaration=use['font']['declaredBy'];origin=declaration['origin'];assert origin['kind']=='object'
            owner,_,owner_ordinals=nodes(case['source']['path'],origin['object']['part'])
            node=next(n for n,o in owner_ordinals.items() if o==origin['sourceOrdinal'])
            assert node.tag=='{'+A+'}latin' and declaration['element']=='latin'
            assert node.get('typeface')==use['font']['typeface']==f['selection']['typeface']
            assert use['style']==f['selection']['style'];font_uses+=1
request_schema=json.loads(Path('contracts/generated/pptx-text-page-request.schema.json').read_text())
response_schema=json.loads(Path('contracts/generated/pptx-text-page-raster-response.schema.json').read_text())
rv=jsonschema.Draft202012Validator(request_schema);sv=jsonschema.Draft202012Validator(response_schema)
for schema in [request_schema,response_schema]:jsonschema.Draft202012Validator.check_schema(schema)
valid=invalid=0
def unique(pairs):
    result={}
    for key,value in pairs:
        if key in result:raise ValueError('duplicate member')
        result[key]=value
    return result
for case in report['cases']:
    bad=case['name'] in ['prior-unknown-profile','prior-unknown-member','prior-numeric-resource-offset','prior-duplicate-field']
    try:q=json.loads(load(case['request']),object_pairs_hook=unique)
    except ValueError:assert bad;invalid+=1
    else:
        errors=list(rv.iter_errors(q));assert bool(errors)==bad,(case['name'],errors)
        invalid+=bool(errors);valid+=not errors
    sv.validate(json.loads(load(case['response'])))
result=dict(format='musteroffice.text-resource-reference/1',parity=entry(ROOT/'parity.json'),
            referenceInputs=[entry('tools/verification/mce_reference.py'),entry('contracts/generated/pptx-text-page-request.schema.json'),entry('contracts/generated/pptx-text-page-raster-response.schema.json')],
            counts=dict(physicalPaintRuns=paint_runs,fontUses=font_uses,validRequests=valid,invalidRequests=invalid,responses=len(report['cases'])),
            tools={name:importlib.metadata.version(name) for name in ['lxml','jsonschema']},
            scope='Owned direct-font fixtures: actual XML paragraph/run ordinals, font declarations and explicit bold/italic/insertion flags. Not full theme-font or Office/WPS conformance.')
(ROOT/'reference.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result['counts']))
