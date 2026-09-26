"""Build current XML source editors and both product/probe runtimes."""
import json,os,subprocess,sys
from pathlib import Path
root=Path('.codex-work/attribute-edit')
env=dict(os.environ,CARGO_BUILD_JOBS='2',MO_SKIA_LIB_DIR=str(Path('.codex-work/elliptic-fast/component').resolve()))
commands=[('fmt',['cargo','fmt','--all','--','--check']),
 ('tests',['cargo','test','--workspace','--locked']),
 ('clippy',['cargo','clippy','--workspace','--all-targets','--locked','--','-D','warnings']),
 ('native-build',['cargo','build','--locked','-p','mo-cli','-p','mo-raster-worker','-p','mo-text-worker']),
 ('schema-check',['cargo','run','--locked','-p','mo-contract-codegen','--','check','contracts/generated']),
 ('types-check',['pnpm','check:types']),
 ('rust-wasm',['cargo','build','--locked','--release','-p','mo-wasm','--target','wasm32-unknown-unknown']),
 ('bindgen',['.codex-work/toolchain/bin/wasm-bindgen','target/wasm32-unknown-unknown/release/mo_wasm.wasm','--target','nodejs','--out-dir',str(root/'wasm-node')]),
 ('native-xml',['cargo','build','--release','--locked','-p','mo-xml','--message-format=json']),
 ('wasm-xml',['cargo','build','--release','--locked','-p','mo-xml','--target','wasm32-unknown-unknown','--message-format=json'])]
records=[]
if '--probes-only' in sys.argv:
    records=json.loads((root/'checks.json').read_text())[:len(commands)]
    assert len(records)==len(commands)
    assert all(r['exitCode']==0 and r['name']==name and r['argv']==argv for r,(name,argv) in zip(records,commands))
def run(name,args):
    p=root/(name+'.log')
    with p.open('w') as log:r=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT)
    record=dict(name=name,argv=args,exitCode=r.returncode,log=str(p));records.append(record)
    (root/'checks.json').write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(record),flush=True)
    if r.returncode:raise SystemExit(r.returncode)
if '--probes-only' not in sys.argv:
    for name,args in commands:run(name,args)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
for target in ['native','wasm']:
    artifacts=[]
    for line in (root/(target+'-xml.log')).read_text().splitlines():
        if not line.startswith('{'):continue
        r=json.loads(line)
        if r.get('reason')=='compiler-artifact' and r['target']['name']=='mo_xml':artifacts+=r['filenames']
    library=next(p for p in artifacts if p.endswith('.rlib'))
    argv=['rustc','--edition=2024','-Copt-level=3','-Coverflow-checks=on','-Cpanic=abort','-Clto=thin','-Ccodegen-units=1','-L','dependency='+str(Path(library).parent/'deps')]
    # Cargo's top-level rlib reexports dependency metadata from the deps folder.
    if Path(library).parent.name=='deps':argv[-1]='dependency='+str(Path(library).parent)
    if target=='wasm':argv+=['--target','wasm32-unknown-unknown','--crate-type=cdylib','-L','dependency=target/release/deps']
    argv+=['--extern','mo_xml='+library,'tools/verification/xml-attribute-probe.rs','-o',str(root/('probe.wasm' if target=='wasm' else 'native-probe'))]
    run(target+'-probe',argv)
