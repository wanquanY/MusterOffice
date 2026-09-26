"""Scoped reproducible workspace validation; prior frozen reports are immutable."""
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path('.codex-work/delivery-pipeline')
start = sys.argv[1] if len(sys.argv) > 1 else 'fmt'
env = dict(os.environ, CARGO_BUILD_JOBS='2', MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
commands = [
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('tests', ['cargo', 'test', '--workspace', '--locked', '--offline']),
    ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']),
    ('native-build', ['cargo', 'build', '--locked', '--offline', '-p', 'mo-cli', '-p', 'mo-host', '-p', 'mo-raster-worker']),
    ('example-build', ['cargo', 'build', '--locked', '--offline', '-p', 'mo-native-render', '--example', 'delivery']),
    ('schema-check', ['cargo', 'run', '--locked', '--offline', '-p', 'mo-contract-codegen', '--', 'check', 'contracts/generated']),
    ('types-check', ['pnpm', 'check:types']),
    ('pure-delivery-wasm', ['cargo', 'check', '--locked', '--offline', '-p', 'mo-presentation-delivery', '--target', 'wasm32-unknown-unknown']),
    ('rust-wasm', ['cargo', 'build', '--locked', '--offline', '--release', '-p', 'mo-wasm', '--target', 'wasm32-unknown-unknown']),
    ('bindgen', ['.codex-work/toolchain/bin/wasm-bindgen', 'target/wasm32-unknown-unknown/release/mo_wasm.wasm', '--target', 'nodejs', '--out-dir', str(root/'wasm-node')]),
    ('native-tree', ['cargo', 'tree', '-p', 'mo-native-render', '--edges', 'normal,build', '--locked', '--offline', '--format', '{p} | {l} | {f}']),
    ('wasm-tree', ['cargo', 'tree', '-p', 'mo-presentation-delivery', '--target', 'wasm32-unknown-unknown', '--edges', 'normal,build', '--locked', '--offline', '--format', '{p} | {l} | {f}']),
]
names = [x[0] for x in commands]
assert start in names
report_path = root/'workspace.json'
records = json.loads(report_path.read_text()) if report_path.exists() else []
for name, argv in commands[names.index(start):]:
    attempt = 1
    while (root/f'{name}-{attempt}.log').exists(): attempt += 1
    path = root/f'{name}-{attempt}.log'
    with path.open('x') as log:
        run = subprocess.run(argv, env=env, stdout=log, stderr=subprocess.STDOUT)
    records.append(dict(name=name, argv=argv, exitCode=run.returncode, log=str(path)))
    report_path.write_text(json.dumps(records, indent=2)+'\n')
    print(json.dumps(records[-1]), flush=True)
    if run.returncode: raise SystemExit(run.returncode)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
