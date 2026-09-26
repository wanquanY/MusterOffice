"""Owned noncanonical static angles for source-preserving editing checks."""
import hashlib,json,zipfile
from pathlib import Path
root=Path('.codex-work/angle-export');report=json.loads((root/'export-regression.json').read_text())
source=Path(report['artifactDirectory'])/'native-objects.pptx';cases=[]
for angle in [-8100000,24300000,-2147483648,2147483647]:
    path=root/('source-'+str(angle)+'.pptx')
    with zipfile.ZipFile(source) as original,zipfile.ZipFile(path,'w',zipfile.ZIP_STORED) as target:
        for info in original.infolist():
            data=original.read(info.filename)
            if info.filename=='ppt/slides/slide2.xml':
                assert data.count(b'rot="21300000"')==1
                data=data.replace(b'rot="21300000"',f'rot="{angle}"'.encode())
            target.writestr(info,data)
    cases.append({'name':'source-'+str(angle),'angle':angle,'sourcePath':str(path),'sourceSha256':hashlib.sha256(path.read_bytes()).hexdigest()})
(root/'source-inputs.json').write_text(json.dumps(cases,indent=2)+'\n')
