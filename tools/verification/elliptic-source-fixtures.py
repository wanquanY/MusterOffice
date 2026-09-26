"""Owned invalid source mutations; no target-application files are rewritten."""
import hashlib,io,json,zipfile
from pathlib import Path
root=Path('.codex-work/elliptic-source');out=root/'invalid';out.mkdir(exist_ok=True)
base=(root/'native-final/center-point.pptx').read_bytes();page=json.loads((root/'native-final/center-point.request.json').read_text())
source_cases=[]
mutations={
 'stationary':lambda s:s.replace('<a:gradFill>','<a:gradFill rotWithShape="0">'),
 'inverted-focus':lambda s:s.replace('l="50000" t="50000" r="50000" b="50000"','l="80000" t="50000" r="80000" b="50000"'),
 'inverted-tile':lambda s:s.replace('</a:gradFill>','<a:tileRect l="80000" r="80000"/></a:gradFill>'),
 'shape-path':lambda s:s.replace('path="circle"','path="shape"'),
}
for name,mutate in mutations.items():
    buf=io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(base)) as inp,zipfile.ZipFile(buf,'w') as output:
        for info in inp.infolist():
            data=inp.read(info.filename)
            if info.filename=='ppt/slides/slide1.xml':
                original=data;data=mutate(data.decode()).encode();assert original!=data
            output.writestr(info,data)
    source=buf.getvalue();p=out/(name+'.pptx');p.write_bytes(source)
    q=json.loads(json.dumps(page));q['expectedSourceSha256']=hashlib.sha256(source).hexdigest()
    request=dict(profile='drawingml-resource-page-q32-v1-draft',page=q,imageSource='embeddedSnapshot',sampling='nearest',fonts=None)
    path=out/(name+'.json');path.write_text(json.dumps(request))
    source_cases.append(dict(name=name,source=str(p),request=str(path)))
(root/'invalid.json').write_text(json.dumps(source_cases,indent=2)+'\n')
