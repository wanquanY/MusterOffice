/** Public visual WASM: compare complete GPU commands with native Rust. */
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const [nativePath, deliveryPath, reportPath, vectorsPath] = process.argv.slice(2);
assert(nativePath && deliveryPath && reportPath, 'usage: native delivery report');
import { document, queries, ratio } from './video-visual-fixture.mjs';
function native(input) {
  const run = spawnSync(resolve(nativePath), [], { input: JSON.stringify(input), encoding: 'utf8', maxBuffer: 8 << 20 });
  assert.equal(run.status, 0, run.stderr); return JSON.parse(run.stdout);
}
const expected = native({ document, queries });
for (let i = 0; i < queries.length; i++) {
  queries[i].replies = expected.results[i].requests.map(request => {
    const desc = request.binding.descriptor;
    const image = { ...desc.dimensions, hasHdr: false, display: { sampleAspectRatio: [2, 1], matrix: [0, 1, 0, -1, 0, 0, 7, -9, 1] } };
    const interval = { start: ratio(0), end: ratio(4) };
    return { request: request.key, binding: request.binding, visual: desc.kind === 'image' ? { kind: 'image', image } : i === 3 ? { kind: 'videoGap', interval } : { kind: 'videoFrame', interval, image } };
  });
}
const reference = native({ document, queries });
if (vectorsPath) await writeFile(vectorsPath, JSON.stringify(reference));
const root = resolve(deliveryPath), bytes = await readFile(root + '/runtime/mo_video_visual_wasm_bg.wasm');
const module = await WebAssembly.compile(bytes);
const { createVisualRuntime } = await import(pathToFileURL(root + '/index.mjs'));
const runtime = await createVisualRuntime(module);
await assert.rejects(createVisualRuntime(module));
assert.equal(runtime.composition_shader(), await readFile(new URL('../../crates/mo-video-gpu-plan/src/compose.wgsl', import.meta.url), 'utf8'));
function rejects(fn, code) {
  try { fn(); assert.fail('unexpected success'); } catch (e) { assert.equal(typeof e, 'string', String(e)); assert.equal(JSON.parse(e).code, code); }
}
const raw = JSON.stringify(document);
rejects(() => runtime.ProjectVisual.open(raw, '00'.repeat(32)), 'invalid_request');
const project = runtime.ProjectVisual.open(raw, reference.documentDigest);
assert.equal(JSON.parse(project.info()).frameCount, '120');
const programs = [];
let faults = 0;
for (let i = 0; i < queries.length; i++) {
  const q = queries[i], candidate = project.frame(BigInt(q.frame), q.width, q.height);
  assert.deepEqual(JSON.parse(candidate.requests()), reference.results[i].requests);
  assert.equal(JSON.parse(candidate.identity()).frameIndex, q.frame);
  assert.deepEqual(JSON.parse(candidate.compile(JSON.stringify(q.replies))), reference.results[i].program);
  for (const change of [
    r => r.pop(), r => { r[1] = r[0]; }, r => { r[0].binding.digest = 'aa'.repeat(32); },
    r => { r[0].request = 'bb'.repeat(32); }, r => { r[1].visual.image.width++; },
  ]) {
    const bad = structuredClone(q.replies); change(bad);
    rejects(() => candidate.compile(JSON.stringify(bad)), 'invalid_request'); faults++;
  }
  programs.push(JSON.parse(candidate.compile(JSON.stringify(q.replies))));
  candidate.free();
}
for (const index of [-1n, 120n]) rejects(() => project.frame(index, 12, 8), 'invalid_request');
rejects(() => project.frame(0n, 0, 8), 'invalid_request');
const owned = project.frame(0n, 12, 16);
project.free(); // candidate remains an owned immutable value
assert.deepEqual(JSON.parse(owned.compile(JSON.stringify(queries[0].replies))), reference.results[0].program);
owned.free();
const sha = v => createHash('sha256').update(v).digest('hex');
const report = { format: 'musteroffice.video-visual-sdk-check/1', status: 'passed', frames: queries.length, replyFaults: faults,
  profile: runtime.profile, wasmBytes: bytes.length, wasmSha256: sha(bytes), manifestSha256: sha(await readFile(root + '/bundle-manifest.json')),
  nativeSha256: sha(await readFile(resolve(nativePath))), fixtureSha256: sha(JSON.stringify({ document, queries })), programSha256: sha(JSON.stringify(programs)),
  scope: 'Actual public WASM vs native Rust byte-exact commands: nested reverse, noninteger frame rate, translation/rotation/mirroring/SAR/group opacity, still image and genuine gap, changed reply rejection and owned candidate lifetime. No browser codec/GPU/device playback or product acceptance.' };
await writeFile(reportPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report));
