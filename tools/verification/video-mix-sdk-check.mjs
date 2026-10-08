/** Actual public mixer WASM against the native shared DSP, not a JS mixer. */
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { fixture } from './video-audio-fixture.mjs';
const [nativePath, deliveryPath, reportPath] = process.argv.slice(2);
assert(nativePath && deliveryPath && reportPath, 'usage: native delivery report');
const raw = JSON.stringify(fixture);
const native = spawnSync(resolve(nativePath), ['--audio'], { input: raw, encoding: 'utf8', maxBuffer: 8 << 20 });
assert.equal(native.status, 0, native.stderr);
const expected = JSON.parse(native.stdout);
assert.equal(expected.status, 'complete', expected.error);
const root = resolve(deliveryPath);
const bytes = await readFile(root + '/runtime/mo_video_mix_wasm_bg.wasm');
const module = await WebAssembly.compile(bytes);
const { createMixRuntime } = await import(pathToFileURL(root + '/index.mjs'));
const runtime = await createMixRuntime(module);
await assert.rejects(createMixRuntime(module));
const description = expected.descriptions[0];
let mode = 'normal', describes = 0, reads = 0;
const describe = text => {
  describes++;
  assert.deepEqual(JSON.parse(text), description.binding);
  if (mode === 'pendingDescribe') return null;
  if (mode === 'describePromise') return Promise.resolve('unused');
  const value = structuredClone(description);
  if (mode === 'stale') value.binding.digest = '33'.repeat(32);
  if (mode === 'wrongLattice') value.sampleZero.numerator = '1';
  return JSON.stringify(value);
};
const read = text => {
  reads++;
  const request = JSON.parse(text);
  assert.deepEqual(request.binding, description.binding);
  assert.equal(BigInt(request.endSample) - BigInt(request.firstSample), BigInt(request.frames));
  assert.equal(request.maxBytes, request.frames * 4);
  if (mode === 'pendingRead') return undefined;
  if (mode === 'readPromise') return Promise.resolve(new Uint8Array());
  if (mode === 'length') return new Uint8Array(request.maxBytes + 4);
  const data = new Uint8Array(request.maxBytes), view = new DataView(data.buffer);
  for (let i = 0; i < request.frames; i++) view.setFloat32(i * 4, fixture.pcm[0].samples[Number(request.firstSample) + i], true);
  if (mode === 'nan') view.setFloat32(0, NaN, true);
  return data;
};
const document = JSON.stringify(fixture.document);
function rejects(run, code) {
  try { run(); assert.fail('unexpected success'); } catch (error) {
    assert.equal(typeof error, 'string', String(error));
    const failure = JSON.parse(error); assert.equal(failure.code, code); return failure;
  }
}
rejects(() => runtime.ProjectAudio.open(document, '00'.repeat(32), describe, read), 'invalid_request');
const project = runtime.ProjectAudio.open(document, expected.documentDigest, describe, read);
const info = JSON.parse(project.info());
assert.equal(info.documentDigest, expected.documentDigest);
assert.equal(info.audioSampleRate, 8000);
assert.equal(info.audioSampleCount, '4096');
assert.equal(info.audioChannels, 2);
let mixed = 0;
for (let i = 0; i < fixture.queries.length; i++) {
  const q = fixture.queries[i], want = expected.results[i];
  if (want.status !== 'mixed') {
    rejects(() => project.read(BigInt(q.first), q.frames), i === 5 ? 'unsupported_capability' : 'invalid_request');
    assert.equal(project.stats(), 'null'); continue;
  }
  const pcm = project.read(BigInt(q.first), q.frames), view = new DataView(pcm.buffer, pcm.byteOffset, pcm.byteLength);
  const bits = Array.from({ length: pcm.length / 4 }, (_, n) => view.getUint32(n * 4, true));
  assert.deepEqual(bits, want.bits.flatMap(b => [b, b])); mixed++;
  assert.equal(JSON.parse(project.stats()).voices, want.stats.voices);
}
for (const fault of ['pendingDescribe', 'pendingRead', 'describePromise', 'readPromise', 'stale', 'wrongLattice', 'length', 'nan']) {
  mode = fault;
  const result = rejects(() => project.read(0n, 16), fault.startsWith('pending') ? 'pending_source' : 'invalid_request');
  if (fault.startsWith('pending')) assert.deepEqual(result.source.binding, description.binding);
  assert.equal(project.stats(), 'null');
  mode = 'normal';
  assert.equal(project.read(0n, 16).length, 128);
}
const reference = project.read(123n, 137);
project.clear();
assert.equal(project.stats(), 'null');
assert.deepEqual(project.read(123n, 137), reference);
for (const n of [0, 4097]) rejects(() => project.read(0n, n), 'invalid_request');
project.free();
for (const channels of [0, 3, 64]) rejects(() => runtime.ProjectAudio.open_with_channels(document, expected.documentDigest, channels, describe, read), 'invalid_request');
let monoMixed = 0;
const mono = runtime.ProjectAudio.open_with_channels(document, expected.documentDigest, 1, describe, read);
try {
  assert.equal(JSON.parse(mono.info()).audioChannels, 1);
  for (let i = 0; i < fixture.queries.length; i++) {
    if (expected.results[i].status !== 'mixed') continue;
    const q = fixture.queries[i], pcm = mono.read(BigInt(q.first), q.frames);
    const v = new DataView(pcm.buffer, pcm.byteOffset, pcm.byteLength);
    assert.deepEqual(Array.from({ length: pcm.length / 4 }, (_, n) => v.getUint32(n * 4, true)), expected.results[i].bits);
    monoMixed++;
  }
} finally { mono.free(); }
const sha = v => createHash('sha256').update(v).digest('hex');
const report = { format: 'musteroffice.video-mix-sdk-check/1', status: 'passed', mixed, monoMixed,
  queryRejections: 3, resourceFaults: 8, describes, reads, profile: runtime.profile,
  wasmSha256: sha(bytes), manifestSha256: sha(await readFile(root + '/bundle-manifest.json')),
  nativeSha256: sha(await readFile(resolve(nativePath))), fixtureSha256: sha(raw),
  scope: 'Actual public mono WASM bit-exact vs native mono DSP; compatible stereo WASM duplicated equally. Overlap, gain, fades, rate, reverse, nested sequence, random access and explicit unsupported preserve-pitch. Faults retry cleanly. No browser clock, codec, storage or product acceptance.' };
await writeFile(reportPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report));
