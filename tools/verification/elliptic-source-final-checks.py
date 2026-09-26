"""Final API/CLI/sanitizer checks after all production artifacts are built."""
import json,subprocess
from pathlib import Path
root=Path('.codex-work/elliptic-source')
commands=[
 ('contract-compat',['node','tools/verification/elliptic-source-contract-compat.mjs']),
 ('runtime',['node','tools/verification/elliptic-source-runtime.mjs']),
 ('source-parity',['node','tools/verification/elliptic-source-parity.mjs']),
 ('parameter-reference',['python3','tools/verification/elliptic-parameter-reference.py']),
 ('reference',['python3','tools/verification/elliptic-source-reference.py']),
 ('components',['node','tools/verification/elliptic-source-components.mjs']),
 ('previews',['python3','tools/verification/elliptic-source-previews.py']),
]
records=[]
for name,args in commands:
    path=root/(name+'-verified.log')
    with path.open('w') as log:r=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
    record=dict(name=name,argv=args,exitCode=r.returncode,log=str(path));records.append(record)
    (root/'final-checks.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(record),flush=True)
    if r.returncode:raise SystemExit(r.returncode)
