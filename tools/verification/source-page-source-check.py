"""Independent raw XML visibility/coverage binding and changed-part XSD checks."""
import hashlib,json,zipfile
from pathlib import Path
from lxml import etree as E
from mce_reference import A,P
ROOT=Path('.codex-work/source-page');N={'a':A,'p':P}
read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
schema=E.XMLSchema(E.parse('.codex-work/ecma376/xsd/pml.xsd',E.XMLParser(resolve_entities=False,no_network=True)))
counts={'surfaces':0,'objects':0,'explicitVisibility':0,'textBodies':0,'visualLocations':0,'validChangedParts':0};invalid=[];cases=[]
modified=['ppt/slides/slide1.xml','ppt/slideLayouts/slideLayout2.xml','ppt/slideMasters/slideMaster2.xml']
def flag(node,name):
    return None if name not in node.attrib else node.get(name) in ['1','true']
def issues(owner,raw,nodes):
    for issue in owner.get('visualIssues',[]):
        node=nodes[issue['sourceOrdinal']]
        assert node is raw or raw in node.iterancestors()
        if issue['kind']=='element':
            q=E.QName(node)
            assert (q.namespace or '',q.localname)==(issue['namespace'],issue['localName'])
        else:
            key='{'+issue['namespace']+'}'+issue['localName'] if issue['namespace'] else issue['localName']
            assert key in node.attrib
        counts['visualLocations']+=1
for c in read(ROOT/'manifest.json')['cases']:
    assert sha(Path(c['path']).read_bytes())==c['sha256']
    response_path=ROOT/(c['name']+'.source.response.json');r=read(response_path)
    assert r['status']=='inspected'
    with zipfile.ZipFile(c['path']) as z:
        for part in modified:
            xml=E.fromstring(z.read(part))
            if schema.validate(xml):counts['validChangedParts']+=1
            else:
                assert c['name'] in ['hidden-group','visible-unknown'] and part==modified[0],(c['name'],part,str(schema.error_log))
                assert 'scene3d' in str(schema.error_log)
                invalid.append({'name':c['name'],'part':part,'reason':str(schema.error_log.last_error)})
        for part,surface in r['index']['surfaces'].items():
            doc=E.fromstring(z.read(part[1:]));nodes=list(doc.iter());ordinals={id(n):i for i,n in enumerate(nodes)}
            assert surface.get('showMasterShapes')==flag(doc,'showMasterSp')
            counts['surfaces']+=1;counts['explicitVisibility']+='showMasterSp' in doc.attrib
            issues(surface,doc,nodes)
            objects={}
            for n in doc.find('p:cSld/p:spTree',N).iter():
                if E.QName(n).namespace==P and E.QName(n).localname in ['sp','pic','cxnSp','grpSp','graphicFrame']:
                    ident=n.find('./*/p:cNvPr',N);objects[int(ident.get('id'))]=n
            for o in surface['objects']:
                n=objects[o['nativeId']];ident=n.find('./*/p:cNvPr',N);text=n.find('p:txBody',N)
                assert o.get('hidden')==flag(ident,'hidden')
                assert o.get('textBodyOrdinal')==(None if text is None else ordinals[id(text)])
                counts['objects']+=1;counts['explicitVisibility']+='hidden' in ident.attrib;counts['textBodies']+=text is not None
                issues(o,n,nodes)
    cases.append({'name':c['name'],'sourcePath':c['path'],'sourceSha256':c['sha256'],'responsePath':str(response_path),'responseSha256':sha(response_path.read_bytes())})
assert len(invalid)==2
result={'format':'musteroffice.source-page-source-check/1','counts':counts,'invalidChangedParts':invalid,'scope':'XSD covers the three rewritten page parts in each owned package; raw metadata bindings cover all indexed surfaces. Two intentionally incomplete scene3d declarations are negative coverage probes.','cases':cases}
(ROOT/'source-check.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'counts':counts,'invalidChangedParts':len(invalid)}))
