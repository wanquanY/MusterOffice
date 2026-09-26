"""Build the shared animation rendering boundary and run workspace/contract verification."""
import json, os, subprocess
from pathlib import Path
root=Path('.codex-work/playback-render');root.mkdir(exist_ok=True,parents=True)
env=dict(os.environ,CARGO_BUILD_JOBS='2',MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
commands=[
 ('fmt',['cargo','fmt','--all','--','--check']),
 ('tests',['cargo','test','--workspace','--locked','--offline']),
 ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--offline','--','-D','warnings']),
 ('native-build',['cargo','build','--locked','--offline','-p','mo-cli','-p','mo-raster-worker','-p','mo-text-worker']),
 ('schema-write',['cargo','run','--locked','--offline','-p','mo-contract-codegen','--','write','contracts/generated']),
 ('schema-check',['cargo','run','--locked','--offline','-p','mo-contract-codegen','--','check','contracts/generated']),
 ('types-write',['pnpm','generate:types']),
 ('types-check',['pnpm','check:types']),
 ('rust-wasm',['cargo','build','--locked','--offline','--release','-p','mo-wasm','--target','wasm32-unknown-unknown']),
 ('bindgen',['.codex-work/toolchain/bin/wasm-bindgen','target/wasm32-unknown-unknown/release/mo_wasm.wasm','--target','nodejs','--out-dir',str(root/'wasm-node')]),
]
records=[]
for name,argv in commands:
 path=root/(name+'.log')
 with path.open('w') as log:r=subprocess.run(argv,env=env,stdout=log,stderr=subprocess.STDOUT)
 records.append(dict(name=name,argv=argv,exitCode=r.returncode,log=str(path)))
 (root/'workspace.json').write_text(json.dumps(records,indent=2)+'\n')
 print(json.dumps(records[-1]),flush=True)
 if r.returncode:raise SystemExit(r.returncode)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
