"""Build and verify resource ownership without rewriting prior stage outputs."""
import json
import os
from pathlib import Path
import subprocess

root=Path('.codex-work/resource-host')
root.mkdir(exist_ok=True,parents=True)
env=dict(os.environ,CARGO_BUILD_JOBS='2',MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
commands=[
    ('fmt',['cargo','fmt','--all','--','--check']),
    ('tests',['cargo','test','--workspace','--locked','--offline']),
    ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),
    ('native-build',['cargo','build','--locked','--offline','-p','mo-host']),
    ('host-release',['cargo','build','--locked','--offline','--release','-p','mo-host']),
    ('schema-check',['cargo','run','--locked','--offline','-p','mo-contract-codegen','--','check','contracts/generated']),
    ('types-check',['pnpm','check:types']),
    ('pure-service-wasm',['cargo','check','--locked','--offline','-p','mo-operation-service','--target','wasm32-unknown-unknown']),
    ('host-tree',['cargo','tree','-p','mo-host','--edges','normal,build','--locked','--offline','--format','{p} | {l} | {f}']),
    ('wasm-tree',['cargo','tree','-p','mo-wasm','--target','wasm32-unknown-unknown','--edges','normal,build','--locked','--offline','--format','{p} | {l} | {f}']),
]
records=[]
for name,argv in commands:
    path=root/(name+'.log')
    with path.open('w') as log:
        result=subprocess.run(argv,env=env,stdout=log,stderr=subprocess.STDOUT)
    records.append(dict(name=name,argv=argv,exitCode=result.returncode,log=str(path)))
    (root/'workspace.json').write_text(json.dumps(records,indent=2)+'\n')
    print(json.dumps(records[-1]),flush=True)
    if result.returncode:raise SystemExit(result.returncode)
