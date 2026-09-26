"""Independent DOM/XSD/source-provenance checks of resolved fill expressions.

This does not use a second copy of the Rust merge algorithm as an oracle.
Explicit authored expectations test branch/priority/default behavior; this
checker binds every returned origin to the real physical XML and verifies
actual edited packages retain all non-text content.
"""
import hashlib
import json
import zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import project, A, P

ROOT=Path('.codex-work/fill-resolution'); XSD=Path('.codex-work/ecma376/xsd'); NS={'a':A,'p':P}
read=lambda p:json.loads(Path(p).read_text()); sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=read(ROOT/'manifest.json'); report=read(ROOT/'parity.json'); records={c['name']:c for c in report['cases']}
schemas={k:E.XMLSchema(E.parse(str(XSD/n),E.XMLParser(resolve_entities=False,no_network=True))) for k,n in [('surface','pml.xsd'),('theme','dml-main.xsd')]}
checks=[]; total_origins=total_expectations=total_xsd=total_contexts=0
for case in manifest['cases']:
    record=records['owned-'+case['name']]; raw=Path(record['responsePath']).read_bytes(); assert sha(raw)==record['responseSha256']; response=json.loads(raw)
    assert sha(Path(case['path']).read_bytes())==case['sha256']
    with zipfile.ZipFile(case['path']) as z: before={n:z.read(n) for n in z.namelist()}
    surfaces={}; physical={}; ords={}; parts=0
    for part,b in before.items():
        if not part.endswith('.xml') or not part.startswith(('ppt/slides/','ppt/slideLayouts/','ppt/slideMasters/','ppt/theme/')): continue
        dom,_,ordinal=project(b,with_ordinals=True); part='/'+part
        surfaces[part]=dom; physical[part]={v:k for k,v in ordinal.items()}; ords[part]=ordinal
        if case['xsdConforming'] or part!='/ppt/slides/slide1.xml': schemas['theme' if part.startswith('/ppt/theme/') else 'surface'].assertValid(dom); parts+=1
    def owner(binding):
        part=binding['part'];root=surfaces[part];target=binding['target'];kind=target['kind']
        if kind=='background': return root.find('p:cSld/p:bg',NS)
        if kind=='rootGroup': return root.find('p:cSld/p:spTree/p:grpSpPr',NS)
        return next(n.getparent().getparent() for n in root.findall('.//p:cNvPr',NS) if int(n.get('id'))==target['nativeId'])
    def contains(parent,node): return parent is node or parent in node.iterancestors()
    def walk(value):
        if isinstance(value,list):
            for v in value: walk(v)
        if not isinstance(value,dict): return
        if 'declaredBy' in value: verify(value['declaredBy'])
        if 'origin' in value: verify(value['origin'])
        if value.get('contextOwner'):
            assert owner(value['contextOwner']) is not None
            counters['contexts']+=1
        for k,v in value.items():
            if k not in ['declaredBy','origin']: walk(v)
    def verify(origin):
        counters['origins']+=1; kind=origin['kind']
        if kind=='profileDefault': return
        part=origin.get('part') or origin['owner']['part']; node=physical[part][origin['sourceOrdinal']]
        if kind=='declaration':
            assert contains(owner(origin['owner']),node),(case['name'],origin)
        elif kind=='schemaDefault':
            assert E.QName(node).localname in ['tileRect','fillRect'],origin
        elif kind=='theme':
            ref=physical[origin['via']['part']][origin['referenceOrdinal']]
            assert contains(owner(origin['via']),ref)
            idx=origin['styleIndex']; assert int(ref.get('idx'))==idx
            local=E.QName(ref).localname
            family='lnStyleLst' if local=='lnRef' else 'bgFillStyleLst' if idx>1000 else 'fillStyleLst'
            offset=idx-1001 if idx>1000 and local!='lnRef' else idx-1
            entry=surfaces[part].find('a:themeElements/a:fmtScheme/a:'+family,NS)[offset]
            assert contains(entry,node),(case['name'],origin)
        else: raise AssertionError(kind)
    counters={'origins':0,'contexts':0};walk(response)
    for pointer,expected in case['expectations'].items():
        value=response
        for k in pointer[1:].split('/'): value=value[int(k)] if isinstance(value,list) else value[k]
        assert value==expected,(case['name'],pointer,value,expected)
    edited=records['edited-'+case['name']]; candidate=Path(edited['pptxPath']).read_bytes();assert sha(candidate)==edited['pptxSha256']
    with zipfile.ZipFile(edited['pptxPath']) as z: after={n:z.read(n) for n in z.namelist()}
    assert before.keys()==after.keys()
    edit=read(edited['editRequestPath'])['edits'][0]; target=edit['target'];part=target['part'].lstrip('/')
    for p,b in before.items():
        if p!=part: assert b==after[p],p
    a=E.fromstring(before[part]);b=E.fromstring(after[part])
    def text(dom):
        shape=next(n.getparent().getparent() for n in dom.findall('.//p:cNvPr',NS) if int(n.get('id'))==target['objectId'])
        return shape.find('p:txBody/a:p/a:r/a:t',NS)
    at,bt=text(a),text(b);assert at.text==edit['expectedText'];assert bt.text==edit['replacement'];bt.text=at.text
    assert E.tostring(a)==E.tostring(b),case['name']
    checks.append({'name':case['name'],'sourceSha256':case['sha256'],'origins':counters['origins'],'colorContexts':counters['contexts'],'authoredExpectations':len(case['expectations']),'xsdParts':parts,'nonTextXmlAndOtherPartsPreserved':True})
    total_origins+=counters['origins'];total_contexts+=counters['contexts'];total_expectations+=len(case['expectations']);total_xsd+=parts
result={'format':'musteroffice.fill-style-independent/1','cases':checks,'origins':total_origins,'colorContexts':total_contexts,'authoredExpectations':total_expectations,'validXsdParts':total_xsd,'xsdInputs':[{'path':str(p),'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],'scope':'Explicit inheritance/default expectations, physical provenance, native structure and real text preservation. Not color/image/effect evaluation, imported-page painting or Office/WPS acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['cases','xsdInputs','scope']}))
