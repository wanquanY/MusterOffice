import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = new URL('../../', import.meta.url);
const output = new URL('.codex-work/wasm-node/', root);
await mkdir(output, { recursive: true });
const bindgen = process.env.MO_WASM_BINDGEN_BIN || fileURLToPath(new URL('.codex-work/toolchain/bin/wasm-bindgen', root));
const result = spawnSync(bindgen, [
  fileURLToPath(new URL('target/wasm32-unknown-unknown/release/mo_wasm.wasm', root)),
  '--target', 'nodejs', '--out-dir', fileURLToPath(output),
], { stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) throw new Error(`wasm-bindgen failed: ${result.status}`);
// wasm-bindgen's nodejs target emits CommonJS. Give that artifact its own package
// boundary so the repository's ESM development tools cannot change its interpretation.
await writeFile(new URL('package.json', output), '{"private":true,"type":"commonjs"}\n');
