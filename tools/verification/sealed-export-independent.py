"""Run pinned independent readers against the newly generated native files."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import lxml
import pptx

root=Path('.codex-work/sealed-export');out=root/'independent';out.mkdir()
def entry(p):
    p=Path(p);b=p.read_bytes();return dict(path=str(p),byteLength=len(b),sha256=hashlib.sha256(b).hexdigest())
parent=json.loads(Path('docs/reviews/evidence/2026-09-26-resource-host-verification.json').read_text())
prior={r['path']:r for r in parent['sourceFiles']}
programs=['tools/verification/pptx-independent.py','tools/verification/pptx-source-independent.py','tools/verification/mce_reference.py']
for p in programs:assert entry(p)==prior[p]
schema_receipt=Path('docs/reviews/evidence/2026-09-24-ecma-schema-inputs.json')
assert entry(schema_receipt)['sha256']=='4abdb8b4eadada4b2c223d9543922946b70b4c0604d2974532528a9ef04c8766'
schema_input=json.loads(schema_receipt.read_text())
for item in schema_input['files']:
    assert entry(Path('.codex-work/ecma376/xsd')/item['name'])['sha256']==item['sha256']
authored=json.loads((root/'authored-parity.json').read_text());directory=Path(authored['artifactDirectory'])
reports=[];schemas=0
for c in authored['cases']:
    if c['status']!='exported':continue
    run=subprocess.run([sys.executable,programs[0],str(directory/(c['name']+'.pptx')),'--request',str(directory/(c['name']+'.json')),'--xsd-directory','.codex-work/ecma376/xsd'],capture_output=True,text=True)
    (out/(c['name']+'.stderr')).write_text(run.stderr)
    assert run.returncode==0 and run.stderr=='',run.stderr
    report=json.loads(run.stdout);assert report['result']=='passed' and not report['differences']
    assert report['fileSha256']==c['sha256'] and report['nativeObjectsCompared']==15
    schemas+=len(report['schemasChecked'])
    p=out/(c['name']+'.json');p.write_text(run.stdout);reports.append(entry(p))
run=subprocess.run([sys.executable,programs[1],str(root/'source-parity.json')],capture_output=True,text=True)
(out/'source.stderr').write_text(run.stderr)
assert run.returncode==0 and run.stderr=='',run.stderr
source=json.loads(run.stdout)
p=out/'source.json';p.write_text(run.stdout);reports.append(entry(p))
report=dict(format='musteroffice.sealed-export-independent/1',authoredFiles=len(reports)-1,schemasChecked=schemas,sourceIndependentExitCode=run.returncode,sourceOutputs=source['passed'],preservedCompressedEntries=sum(c['unchangedCompressedEntriesVerified'] for c in source['cases']),reports=reports,programs=[entry(p) for p in programs],schemaFiles=[entry(p) for p in sorted(Path('.codex-work/ecma376/xsd').glob('*.xsd'))],environment=dict(python=sys.version,lxml=lxml.__version__,pythonPptx=pptx.__version__),limitations=['Independent generated-subset XSD/object/media and source-leaf preservation verification; not Office/WPS editing or visual/advanced-feature acceptance.'])
(root/'independent.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:report[k] for k in ['authoredFiles','schemasChecked','sourceOutputs','preservedCompressedEntries']}))
