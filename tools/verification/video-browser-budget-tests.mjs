import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFile, mkdtemp, writeFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { build } from 'esbuild';
import { BrowserVideoBudget, videoSurfaceBytes } from '../../.codex-work/video-browser-media/build/video-browser-media/src/video-budget.js';
const rejects = (fn, code) => assert.throws(fn, error => error.code === code);

test('shared byte admission is atomic and stale release cannot free another owner', () => {
  const pool = new BrowserVideoBudget(1024);
  const a = pool.reserve(768), b = pool.reserve(256);
  rejects(() => pool.reserve(1), 'resource_limit');
  assert.equal(pool.usedBytes, 1024);
  a(); const c = pool.reserve(512); a();
  assert.equal(pool.usedBytes, 768);
  b(); c(); c(); assert.equal(pool.usedBytes, 0); assert.equal(pool.peakBytes, 1024);
});
test('invalid budgets and overflow never admit a resource', () => {
  for (const value of [0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
    rejects(() => new BrowserVideoBudget(value), 'invalid_request');
    const pool = new BrowserVideoBudget(1024);
    rejects(() => pool.reserve(value), 'resource_limit'); assert.equal(pool.usedBytes, 0);
  }
  rejects(() => videoSurfaceBytes(Number.MAX_SAFE_INTEGER, 4096), 'resource_limit');
  assert.equal(videoSurfaceBytes(1920, 1080, 4), 1920 * 1088 * 4);
  assert.equal(videoSurfaceBytes(1920, 1080, 8), 1920 * 1088 * 8);
});
test('AVC admission checks in-band growth, malformed NAL boundaries and extension reference models', async () => {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const spec = JSON.parse(await readFile(path.join(root, 'components/video-browser-media/component.json')));
  const vendor = path.join(root, '.codex-work/components/mediabunny', spec.sourceCommit);
  const hash = createHash('sha256');
  for (const name of [...spec.sourceFiles].sort()) hash.update(name + '\0').update(await readFile(path.join(vendor, name))).update('\0');
  assert.equal(hash.digest('hex'), spec.sourceTreeSha256);
  const directory = await mkdtemp(path.join(root, '.codex-work/video-budget-unit-'));
  try {
    const bundled = await build({ absWorkingDir: root, entryPoints: ['packages/video-browser-media/src/mediabunny-platform.ts'],
      bundle: true, format: 'esm', platform: 'node', target: 'node22', write: false,
      plugins: [{ name: 'reviewed-codec', setup(build) {
        build.onResolve({ filter: /^mediabunny$/ }, () => ({ path: path.join(vendor, 'src/index.ts') }));
        build.onResolve({ filter: /^@musteroffice\/mediabunny-codec-data$/ }, () => ({ path: path.join(vendor, 'src/codec-data.ts') }));
      } }],
    });
    const module = path.join(directory, 'capacity.mjs'); await writeFile(module, bundled.outputFiles[0].contents);
    const { avcCapacity } = await import(pathToFileURL(module).href);
    // Actual x264 configuration from the independent B-frame fixture, 128x96.
    const description = Buffer.from('0164000affe1001a6764000aacd9420db016a020202800001f480007530078912cb001000668ebe3cb22c0fdf8f800', 'hex');
    const capacity = avcCapacity({ codec: 'avc1.64000a', description });
    const nal = bytes => { const out = Buffer.alloc(4 + bytes.length); out.writeUInt32BE(bytes.length); out.set(bytes, 4); return out; };
    const sps = description.subarray(8, 34);
    capacity.validatePacket(nal(sps));
    // Independently encoded 512x512 SPS; do not allow a decoder to grow silently.
    rejects(() => capacity.validatePacket(nal(Buffer.from('67640016acd94080106c044000000300400000030083c58b6580', 'hex'))), 'resource_limit');
    rejects(() => capacity.validatePacket(Buffer.from([0, 0, 0, 10, 0x65])), 'unsupported_media');
    rejects(() => capacity.validatePacket(Buffer.concat([nal(sps), Buffer.from([1])])), 'unsupported_media');
    rejects(() => capacity.validatePacket(Buffer.from([0, 0, 0, 0])), 'unsupported_media');
    rejects(() => capacity.validatePacket(nal(Buffer.from([0xe5]))), 'unsupported_media');
    rejects(() => capacity.validatePacket(nal(Buffer.from([0x74]))), 'unsupported_codec');
  } finally { await rm(directory, { recursive: true }); }
});
