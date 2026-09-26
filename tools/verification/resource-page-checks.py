"""Reproducible build/test commands for the resource page integration stage."""
import json, os, subprocess
from pathlib import Path
root=Path('.codex-work/resource-page'); root.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CARGO_BUILD_JOBS="2",MO_SKIA_LIB_DIR=str(Path('.codex-work/clips/component').resolve()),MO_RESOURCE_PAGE_EVIDENCE_DIR=str((root/'native').resolve()))
commands=[
 ('fmt',['cargo','fmt','--all','--','--check']),
 ('tests',['cargo','test','--workspace','--locked']),
 ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--','-D','warnings']),
 ('native-build',['cargo','build','--locked','-p','mo-cli','-p','mo-raster-worker']),
 ('schema-write',['cargo','run','--locked','-p','mo-contract-codegen','--','write','contracts/generated']),
 ('types-write',['pnpm','generate:types']),
 ('schema-check',['cargo','run','--locked','-p','mo-contract-codegen','--','check','contracts/generated']),
 ('types-check',['pnpm','check:types']),
 ('rust-wasm',['cargo','build','--locked','--release','-p','mo-wasm','--target','wasm32-unknown-unknown']),
 ('bindgen',['.codex-work/toolchain/bin/wasm-bindgen','target/wasm32-unknown-unknown/release/mo_wasm.wasm','--target','nodejs','--out-dir',str(root/'wasm-node')]),
]
records=[]
for name,command in commands:
 with (root/(name+'.log')).open('w') as log: r=subprocess.run(command,env=env,stdout=log,stderr=subprocess.STDOUT)
 records.append(dict(name=name,command=command,exitCode=r.returncode,log=str(root/(name+'.log'))))
 (root/'checks.json').write_text(json.dumps(records,indent=2)+'\n')
 print(json.dumps(records[-1]),flush=True)
 if r.returncode: raise SystemExit(r.returncode)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
