"""Rebuild native consumers and verify unchanged Rust/TS contracts."""
import json, os, subprocess
from pathlib import Path
root=Path('.codex-work/elliptic-fast')
env=dict(os.environ,CARGO_BUILD_JOBS='2',
    MO_SKIA_LIB_DIR=str((root/'component').resolve()),
    MO_ELLIPTIC_SOURCE_DIR=str((root/'native-pages').resolve()))
commands=[
    ('fmt',['cargo','fmt','--all','--','--check']),
    ('tests',['cargo','test','--workspace','--locked']),
    ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--','-D','warnings']),
    ('native-build',['cargo','build','--locked','-p','mo-cli','-p','mo-raster-worker','-p','mo-text-worker']),
    ('schema-check',['cargo','run','--locked','-p','mo-contract-codegen','--','check','contracts/generated']),
    ('types-check',['pnpm','check:types']),
]
records=[]
for name,argv in commands:
    path=root/(name+'.log')
    with path.open('w') as log:r=subprocess.run(argv,env=env,stdout=log,stderr=subprocess.STDOUT)
    record=dict(name=name,argv=argv,exitCode=r.returncode,log=str(path));records.append(record)
    (root/'workspace.json').write_text(json.dumps(records,indent=2)+'\n')
    print(json.dumps(record),flush=True)
    if r.returncode:raise SystemExit(r.returncode)
