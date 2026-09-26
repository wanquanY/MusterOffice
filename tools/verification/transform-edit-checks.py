"""Build and check the public Native/WASM transform editing implementation."""
import json,os,subprocess
from pathlib import Path
root=Path('.codex-work/transform-edit');root.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CARGO_BUILD_JOBS='2',MO_SKIA_LIB_DIR=str(Path('.codex-work/elliptic-fast/component').resolve()))
commands=[
 ('fmt',['cargo','fmt','--all','--','--check']),
 ('tests',['cargo','test','--workspace','--locked']),
 ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--','-D','warnings']),
 ('schema-write',['cargo','run','--locked','-p','mo-contract-codegen','--','write','contracts/generated']),
 ('types-write',['pnpm','generate:types']),
 ('schema-check',['cargo','run','--locked','-p','mo-contract-codegen','--','check','contracts/generated']),
 ('types-check',['pnpm','check:types']),
 ('native-build',['cargo','build','--locked','-p','mo-cli','-p','mo-raster-worker','-p','mo-text-worker']),
 ('rust-wasm',['cargo','build','--locked','--release','-p','mo-wasm','--target','wasm32-unknown-unknown']),
 ('bindgen',['.codex-work/toolchain/bin/wasm-bindgen','target/wasm32-unknown-unknown/release/mo_wasm.wasm','--target','nodejs','--out-dir',str(root/'wasm-node')])]
records=[]
for name,args in commands:
    p=root/(name+'.log')
    with p.open('w') as log:r=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT)
    record=dict(name=name,argv=args,exitCode=r.returncode,log=str(p));records.append(record)
    (root/'checks.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(record),flush=True)
    if r.returncode:raise SystemExit(r.returncode)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
