import concurrent.futures,json,subprocess
from pathlib import Path
root=Path('.codex-work/source-page')
cli='target/release/mo-cli';wasm='.codex-work/wasm-node/mo_wasm.js'
base=[
 ('document','native-wasm-parity',[]),
 ('opc','opc-parity',['.codex-work/opc-fixtures/manifest.json']),
 ('export','pptx-parity',[]),
 ('source','pptx-source-parity',['.codex-work/pptx-color-map-fixtures/manifest.json','.codex-work/pptx-fixture/lo-roundtrip/native-initial.pptx']),
 ('color','pptx-color-parity',['.codex-work/pptx-colors-fixtures/manifest.json','.codex-work/pptx-color-map-fixtures/manifest.json']),
 ('font','font-parity',['.codex-work/font-fixtures/manifest.json']),
]
commands=[(n,['node','tools/verification/'+s+'.mjs',cli,wasm,*a],True) for n,s,a in base]
commands += [(s,['node','tools/verification/'+s+'.mjs'],False) for s in ['text-parity','unicode-parity','cascade-parity','bidi-parity','bidi-conformance','itemization-parity','font-fallback-parity','line-break-parity','font-metrics-parity','line-shape-parity','line-geometry-parity','paragraph-layout-parity','font-outlines-parity']]
def run(job):
 name,cmd,full=job
 r=subprocess.run(cmd,capture_output=True,text=True)
 (root/(name+'-regression.log')).write_text(r.stderr)
 assert r.returncode==0,(name,r.stdout[-1500:],r.stderr[-1500:])
 (root/(name+'-regression.'+('json' if full else 'txt'))).write_text(r.stdout)
 print(name+' passed',flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:list(pool.map(run,commands))

run(('paragraph-paths-parity',['node','tools/verification/paragraph-paths-parity.mjs'],False))

run(('path-raster-parity',['node','tools/verification/path-raster-parity.mjs'],False))

run(('scene-raster-parity',['node','tools/verification/scene-raster-parity.mjs'],False))

for script,args in [("page-render-parity",[]),("page-placement-parity",[]),("group-placement-parity",[".codex-work/angle-export"]),("angle-export-parity",[]),("angle-source-parity",[]),("source-line-parity",[]),("source-line-author",[]),("line-style-parity",[])]:
 run((script,["node","tools/verification/"+script+".mjs",*args],False))

run(("line-color-parity",["node","tools/verification/line-color-parity.mjs"],False))

run(("source-geometry-parity",["node","tools/verification/source-geometry-parity.mjs"],False))

run(("geometry-eval-parity",["node","tools/verification/geometry-eval-parity.mjs"],False))
run(("geometry-application-parity",["node","tools/verification/geometry-application-parity.mjs"],False))

run(("native-path-parity",["node","tools/verification/native-path-parity.mjs"],False))

run(("source-fill-parity",["node","tools/verification/source-fill-parity.mjs"],False))

run(("fill-style-parity",["node","tools/verification/fill-style-parity.mjs"],False))

run(("source-effect-parity",["node","tools/verification/source-effect-parity.mjs"],False))

run(("fill-color-parity",["node","tools/verification/fill-color-parity.mjs"],False))

run(("gradient-raster-parity",["node","tools/verification/gradient-raster-parity.mjs"],False))

run(("preset-expansion-parity",["node","tools/verification/preset-expansion-parity.mjs"],False))

run(("source-placement-parity",["node","tools/verification/source-placement-parity.mjs"],False))
