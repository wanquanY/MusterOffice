"""Build and check the actual anchor-based radial geometry workspace and both runtimes."""
import json, os, subprocess
from pathlib import Path
root = Path('.codex-work/radial-observation')
env = dict(os.environ, CARGO_BUILD_JOBS='2',
           MO_SKIA_LIB_DIR=str(Path('.codex-work/rect-gradient/component').resolve()),
           MO_RADIAL_ANCHOR_DIR=str((root/'native').resolve()))
commands = [
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('tests', ['cargo', 'test', '--workspace', '--locked']),
    ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings']),
    ('native-build', ['cargo', 'build', '--locked', '-p', 'mo-cli', '-p', 'mo-raster-worker', '-p', 'mo-text-worker']),
    ('schema-write', ['cargo', 'run', '--locked', '-p', 'mo-contract-codegen', '--', 'write', 'contracts/generated']),
    ('schema-check', ['cargo', 'run', '--locked', '-p', 'mo-contract-codegen', '--', 'check', 'contracts/generated']),
    ('types-write', ['pnpm', 'generate:types']),
    ('types-check', ['pnpm', 'check:types']),
    ('raster-adapter', ['pnpm', 'exec', 'tsc', '--project', 'packages/raster-component/tsconfig.json', '--outDir', str(root/'ts-raster')]),
    ('rust-wasm', ['cargo', 'build', '--locked', '--release', '-p', 'mo-wasm', '--target', 'wasm32-unknown-unknown']),
    ('bindgen', ['.codex-work/toolchain/bin/wasm-bindgen', 'target/wasm32-unknown-unknown/release/mo_wasm.wasm', '--target', 'nodejs', '--out-dir', str(root/'wasm-node')]),
]
records = []
for name, command in commands:
    with (root/(name+'.log')).open('w') as log:
        r = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT)
    records.append(dict(name=name, command=command, exitCode=r.returncode, log=str(root/(name+'.log'))))
    (root/'checks.json').write_text(json.dumps(records, indent=2)+'\n')
    print(json.dumps(records[-1]), flush=True)
    if r.returncode:
        raise SystemExit(r.returncode)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
