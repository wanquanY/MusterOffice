"""Check real package declarations, physical bindings, XSD and edit preservation."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path
import zipfile
from lxml import etree as E
from mce_reference import project,A,P
from line_reference import line,reference

ROOT=Path('.codex-work/source-lines');XSD=Path('.codex-work/ecma376/xsd')
report=json.loads((ROOT/'parity.json').read_text());manifest=json.loads((ROOT/'manifest.json').read_text())
lookup={c['name']:c for c in report['cases']};NS={'p':P,'a':A}
schema={kind:E.XMLSchema(E.parse(str(XSD/file),E.XMLParser(resolve_entities=False,no_network=True))) for kind,file in [('surface','pml.xsd'),('theme','dml-main.xsd')]}
checks=[];line_count=reference_count=xsd_count=negative_xsd=0
def sha(b):return hashlib.sha256(b).hexdigest()
for c in manifest['cases']:
    record=lookup['inspect-'+c['name']];response=Path(record['responsePath']).read_bytes()
    assert sha(response)==record['responseSha256'];r=json.loads(response)
    with zipfile.ZipFile(c['path']) as z:raw={n:z.read(n) for n in z.namelist()}
    changed=project(raw[c['part'].lstrip('/')])[0]
    sch=schema['theme' if c['part'].startswith('/ppt/theme/') else 'surface']
    if c['error']:
        assert not sch.validate(changed),c['name'];negative_xsd+=1;continue
    index=r['index'];lines=refs=parts=0
    for part,container in {**index['surfaces'],**index['themes']}.items():
        projected,_,ordinals=project(raw[part.lstrip('/')],with_ordinals=True)
        if part in index['surfaces']:
            for node in projected.findall('.//p:sp',NS)+projected.findall('.//p:pic',NS)+projected.findall('.//p:cxnSp',NS):
                # Exclude retained extension payloads, which are not paint-tree objects.
                if any(E.QName(p).localname=='extLst' for p in node.iterancestors()):continue
                identity=node.find('./*/p:cNvPr',NS);assert identity is not None
                obj=next(o for o in container['objects'] if o['nativeId']==int(identity.get('id')))
                for path,field,fn in [('p:spPr/a:ln','line',line),('p:style/a:lnRef','lineReference',reference)]:
                    found=node.find(path,NS);expected=None if found is None else fn(found,ordinals)
                    assert obj.get(field)==expected,(c['name'],part,field,obj.get(field),expected)
                    if found is not None:
                        if field=='line':lines+=1
                        else:refs+=1
        else:
            for node in projected.findall('.//a:lnStyleLst/a:ln',NS):
                style=next(s for s in container['formatScheme']['lines'] if s['sourceOrdinal']==ordinals[node])
                assert style['line']==line(node,ordinals),(c['name'],part);lines+=1
        if c['xsdConforming'] or part!=c['part']:
            schema['surface' if part in index['surfaces'] else 'theme'].assertValid(projected);parts+=1
    edit=lookup['edit-'+c['name']];assert sha(Path(edit['pptxPath']).read_bytes())==edit['pptxSha256']
    with zipfile.ZipFile(edit['pptxPath']) as z:after={n:z.read(n) for n in z.namelist()}
    assert raw.keys()==after.keys()
    for part,b in raw.items():
        if part!='ppt/slides/slide1.xml':assert b==after[part],(c['name'],part)
    # Compare all original non-text XML exactly, after neutralizing just the
    # requested run text. This catches silently rewriting line limits/defaults.
    before_tree=E.fromstring(raw['ppt/slides/slide1.xml']);after_tree=E.fromstring(after['ppt/slides/slide1.xml'])
    path='p:cSld/p:spTree/p:sp/p:txBody/a:p/a:r/a:t'
    a=before_tree.find(path,NS);b=after_tree.find(path,NS);assert b.text=='line source preserved 中文 & < >';b.text=a.text
    assert E.tostring(before_tree)==E.tostring(after_tree),c['name']
    line_count+=lines;reference_count+=refs;xsd_count+=parts
    checks.append({'name':c['name'],'sourceSha256':c['sha256'],'lines':lines,'references':refs,'xsdParts':parts,'lineXmlAndOtherPartsPreserved':True})
authors=[]
for c in json.loads((ROOT/'author/parity.json').read_text())['cases']:
    for kind in ['request','pptx','response']:assert sha(Path(c[kind+'Path']).read_bytes())==c[kind+'Sha256']
    run=subprocess.run([sys.executable,'tools/verification/pptx-independent.py',c['pptxPath'],'--request',c['requestPath'],'--xsd-directory',str(XSD)],capture_output=True,text=True,timeout=30)
    assert run.returncode==0,run.stderr
    verified=json.loads(run.stdout);assert verified['result']=='passed' and not verified['differences']
    authors.append({'name':c['name'],'pptxSha256':c['pptxSha256'],'verification':verified})
result={'format':'musteroffice.source-line-independent/1','cases':checks,'authorFiles':authors,'lineDeclarations':line_count,'lineReferences':reference_count,
        'validXsdParts':xsd_count,'negativeXsdCases':negative_xsd,'xsdInputs':[{ 'path':str(p),'sha256':sha(p.read_bytes())} for p in sorted(XSD.glob('*.xsd'))],
        'scope':'Parsed declarations and physical bindings, projected ECMA XSD, actual native text-edit preservation. No inheritance/rendering or application interoperability acceptance.'}
(ROOT/'independent.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['cases','authorFiles','xsdInputs','scope']}))
