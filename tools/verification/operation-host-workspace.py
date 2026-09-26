"""Verify the real durable host and preserve this stage's distinct outputs."""
import json
import os
from pathlib import Path
import subprocess

root = Path('.codex-work/operation-host')
root.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_BUILD_JOBS='2', MO_SKIA_LIB_DIR=str(Path('.codex-work/gradient-coordinates/component').resolve()))
commands = [
    ('fmt', ['cargo', 'fmt', '--all', '--', '--check']),
    ('tests', ['cargo', 'test', '--workspace', '--locked', '--offline']),
    ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']),
    ('native-build', ['cargo', 'build', '--locked', '--offline', '-p', 'mo-cli', '-p', 'mo-host']),
    ('host-release', ['cargo', 'build', '--locked', '--offline', '--release', '-p', 'mo-host']),
    ('schema-check', ['cargo', 'run', '--locked', '--offline', '-p', 'mo-contract-codegen', '--', 'check', 'contracts/generated']),
    ('types-check', ['pnpm', 'check:types']),
    ('pure-service-wasm', ['cargo', 'check', '--locked', '--offline', '-p', 'mo-operation-service', '--target', 'wasm32-unknown-unknown']),
    ('rust-wasm', ['cargo', 'build', '--locked', '--offline', '--release', '-p', 'mo-wasm', '--target', 'wasm32-unknown-unknown']),
    ('bindgen', ['.codex-work/toolchain/bin/wasm-bindgen', 'target/wasm32-unknown-unknown/release/mo_wasm.wasm', '--target', 'nodejs', '--out-dir', str(root/'wasm-node')]),
    ('sqlite-build', ['cargo', 'test', '--locked', '--offline', '-p', 'mo-standard-host', '--lib', 'actual_sqlite_build_and_connection_durability_configuration', '--', '--exact', 'tests::actual_sqlite_build_and_connection_durability_configuration', '--nocapture']),
]
records = []
for name, argv in commands:
    path = root / (name + '.log')
    with path.open('w') as log:
        result = subprocess.run(argv, env=env, stdout=log, stderr=subprocess.STDOUT)
    records.append(dict(name=name, argv=argv, exitCode=result.returncode, log=str(path)))
    (root/'workspace.json').write_text(json.dumps(records, indent=2)+'\n')
    print(json.dumps(records[-1]), flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
(root/'wasm-node/package.json').write_text('{"private":true,"type":"commonjs"}\n')
