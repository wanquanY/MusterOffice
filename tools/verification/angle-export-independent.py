"""Independent XML/package audit of canonical static-angle serialization."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import zipfile
from lxml import etree

ROOT=Path('.codex-work/angle-export')

def read(p):return json.loads(Path(p).read_text())
def entry(p):
    p=Path(p);b=p.read_bytes()
    return {'path':str(p),'byteLength':len(b),'sha256':hashlib.sha256(b).hexdigest()}

def compare(old_path,new_path):
    changed=[];tokens=0
    with zipfile.ZipFile(old_path) as old,zipfile.ZipFile(new_path) as new:
        assert old.namelist()==new.namelist()
        for name in old.namelist():
            a=old.read(name);b=new.read(name)
            if a==b:continue
            count=0
            def canonical(match):
                nonlocal count
                value=int(match[1]);normalized=value%21600000
                if value!=normalized:count+=1
                return b'rot="'+str(normalized).encode()+b'"'
            expected=re.sub(rb'\brot="(-?\d+)"',canonical,a)
            assert expected==b,(name,'unexpected change beyond rotation token')
            assert count>0
            tokens+=count;changed.append(name)
    return {'before':entry(old_path),'after':entry(new_path),'changedParts':changed,'canonicalizedRotationTokens':tokens,'allOtherUncompressedBytesIdentical':True}

comparisons=[];audits=[]
for fixture in read(ROOT/'probes.json'):
    comparisons.append(compare('.codex-work/group-compat/'+fixture['name']+'.pptx',fixture['pptx']))
    audits.append((fixture['pptx'],str(ROOT/(fixture['name']+'.export.json'))))
old_report=read('.codex-work/group-compat/export-regression.json');new_report=read(ROOT/'export-regression.json')
for old,new in zip(old_report['cases'],new_report['cases'],strict=True):
    assert old['name']==new['name'] and old['status']==new['status']
    if old['status']=='exported':
        assert old['parts']==new['parts']
        old_path=Path(old_report['artifactDirectory'])/(old['name']+'.pptx')
        new_path=Path(new_report['artifactDirectory'])/(new['name']+'.pptx')
        comparisons.append(compare(old_path,new_path))
        audits.append((str(new_path),str(Path(new_report['artifactDirectory'])/(new['name']+'.json'))))
    else:assert old==new
for case in read(ROOT/'angle-parity.json')['cases']:audits.append((case['pptxPath'],case['requestPath']))
results=[]
for pptx,request in audits:
    r=subprocess.run([sys.executable,'tools/verification/pptx-independent.py',pptx,'--request',request,'--xsd-directory','.codex-work/ecma376/xsd'],capture_output=True,text=True,timeout=30)
    assert r.returncode==0,(pptx,r.stderr)
    result=json.loads(r.stdout);assert result['result']=='passed' and not result['differences']
    with zipfile.ZipFile(pptx) as package:
        rotations=[]
        for name in package.namelist():
            if not name.endswith('.xml'):continue
            node=etree.fromstring(package.read(name),etree.XMLParser(resolve_entities=False,no_network=True))
            for t in node.iter('{http://schemas.openxmlformats.org/drawingml/2006/main}xfrm'):
                value=int(t.get('rot','0'));assert 0<=value<21600000;rotations.append(value)
    result['canonicalRotationCount']=len(rotations);results.append(result)
assert len(comparisons)==10 and len(results)==30
report={'format':'musteroffice.angle-export-independent/1','scope':'Independent ZIP/XML byte diff, lxml/official ECMA XSD and python-pptx on actual native/WASM exports. Only static a:xfrm rotations normalize; no source editing or animation normalization.',
        'packageComparisons':comparisons,'fileAudits':results,
        'totals':{'comparedPackages':len(comparisons),'auditedPackages':len(results),'schemaParts':sum(len(r['schemasChecked']) for r in results),'nativeObjects':sum(r['nativeObjectsCompared'] for r in results),'canonicalRotationValues':sum(r['canonicalRotationCount'] for r in results),'changedRotationTokens':sum(r['canonicalizedRotationTokens'] for r in comparisons)}}
(ROOT/'independent.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report['totals']))
