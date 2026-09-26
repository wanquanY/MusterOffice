"""Reproducible checks with append-only attempt logs for delivery admission."""
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path('.codex-work/delivery-receive')
root.mkdir(exist_ok=True)
env = dict(os.environ, CARGO_BUILD_JOBS='2', MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
commands = [
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('workspace-tests', ['cargo', 'test', '--workspace', '--locked', '--offline']),
    ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']),
    ('native', ['cargo', 'build', '--locked', '--offline', '-p', 'mo-cli']),
    ('schemas', ['cargo', 'run', '--locked', '--offline', '-p', 'mo-contract-codegen', '--', 'check', 'contracts/generated']),
    ('types', ['pnpm', 'check:types']),
    ('wasm', ['cargo', 'build', '--locked', '--offline', '--release', '-p', 'mo-wasm', '--target', 'wasm32-unknown-unknown']),
    ('bindgen', ['.codex-work/toolchain/bin/wasm-bindgen', 'target/wasm32-unknown-unknown/release/mo_wasm.wasm', '--target', 'nodejs', '--out-dir', str(root/'wasm-node')]),
]
names = [name for name, _ in commands]
start = sys.argv[1] if len(sys.argv) > 1 else names[0]
assert start in names
record = root/'workspace.json'
records = json.loads(record.read_text()) if record.exists() else []
for name, argv in commands[names.index(start):]:
    attempt = 1
    while (root/f'{name}-{attempt}.log').exists():
        attempt += 1
    log = root/f'{name}-{attempt}.log'
    with log.open('x') as out:
        run = subprocess.run(argv, env=env, stdout=out, stderr=subprocess.STDOUT)
    result = dict(name=name, argv=argv, exitCode=run.returncode, log=str(log))
    records.append(result)
    record.write_text(json.dumps(records, indent=2)+'\n')
    print(json.dumps(result), flush=True)
    if run.returncode:
        raise SystemExit(run.returncode)
    if name == 'bindgen':
        (root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
