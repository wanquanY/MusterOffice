"""Verify the existing Rust callers against the new pinned raster component."""
import json,os,subprocess
from pathlib import Path
root=Path('.codex-work/elliptic-render')
env=dict(os.environ,CARGO_BUILD_JOBS='2',MO_SKIA_LIB_DIR=str((root/'component').resolve()))
commands=[('fmt',['cargo','fmt','--all','--','--check']),
 ('tests',['cargo','test','--workspace','--locked']),
 ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--','-D','warnings']),
 ('native-build',['cargo','build','--locked','-p','mo-cli','-p','mo-raster-worker','-p','mo-text-worker']),
 ('schema-check',['cargo','run','--locked','-p','mo-contract-codegen','--','check','contracts/generated']),
 ('types-check',['pnpm','check:types'])]
records=[]
for name,args in commands:
    p=root/(name+'.log')
    with p.open('w') as log:r=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT)
    records.append(dict(name=name,argv=args,exitCode=r.returncode,log=str(p)))
    (root/'workspace-checks.json').write_text(json.dumps(records,indent=2)+'\n')
    print(json.dumps(records[-1]),flush=True)
    if r.returncode:raise SystemExit(r.returncode)
