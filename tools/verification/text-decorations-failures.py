"""Missing optional tables, derived only from the owned synthetic font."""
import hashlib
import json
from pathlib import Path
from fontTools.ttLib import TTFont

root=Path('.codex-work/text-decorations')
cases=[]
for name,table,source,metric in [('missing-post','post','underline','underlineOffset'),('missing-os2','OS/2','strike','strikeoutOffset')]:
    font=TTFont('fixtures/fonts/owned-decorations.ttf',recalcTimestamp=False)
    del font[table]
    path=root/(name+'.ttf');font.save(path);data=path.read_bytes()
    manifest=json.loads(Path('fixtures/fonts/decoration-manifest.json').read_text())
    manifest['fonts'][0].update(expectedSha256=hashlib.sha256(data).hexdigest(),byteLength=str(len(data)))
    q=dict(profile='drawingml-solid-text-page-q32-draft-v1',page=json.loads((root/'cases'/(source+'.page.json')).read_text()),fonts=manifest)
    query=root/(name+'.input.json');query.write_text(json.dumps(q))
    cases.append(dict(name=name,table=table,source=str(root/'cases'/(source+'.pptx')),font=str(path),query=str(query),metric=metric))
(root/'failures.json').write_text(json.dumps(cases,indent=2)+'\n')
