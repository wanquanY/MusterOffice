/** Explicit fixture + real Worker/WebCodecs; independent FFmpeg oracle. No UI acceptance. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { chromium } from 'playwright-core';
import { verify } from '../video-browser-sdk/build.mjs';
const [browserPath, fixturePath, destination, sdkDirectory, sdkPin] = process.argv.slice(2);
assert(browserPath && fixturePath && destination, 'usage: check.mjs BROWSER BFRAME_VFR_FIXTURE NEW_OUTPUT_DIRECTORY');
const root = fileURLToPath(new URL('../../', import.meta.url));
await mkdir(destination, { recursive: false });
const fixture = await readFile(fixturePath);
assert(Boolean(sdkDirectory) === Boolean(sdkPin), 'SDK directory and external pin must be supplied together');
if (sdkDirectory) await verify(sdkDirectory, sdkPin);
const bundle = (await build({ absWorkingDir: root, entryPoints: ['tools/verification/video-browser-media.worker.ts'],
  bundle: true, format: 'esm', platform: 'browser', write: false, minify: true,
  plugins: sdkDirectory ? [{ name: 'verified-sdk', setup(build) {
    build.onResolve({ filter: /packages\/video-browser-media\/src\/index\.js$/ }, () => ({ path: path.resolve(sdkDirectory, 'index.mjs') }));
  } }] : [],
})).outputFiles[0].contents;
const baseline = JSON.parse(execFileSync('ffprobe', ['-v', 'error', '-select_streams', 'v:0', '-show_frames', '-show_streams', '-of', 'json', fixturePath], { maxBuffer: 8 << 20 }));
const oracleFrames = baseline.frames.filter(x => x.media_type === 'video');
const stream = baseline.streams[0], frameSize = stream.width * stream.height * 4;
assert.equal(stream.time_base.split('/')[0], '1');
const oraclePixels = execFileSync('ffmpeg', ['-v', 'error', '-i', fixturePath, '-map', '0:v:0', '-vsync', '0', '-f', 'rawvideo', '-pix_fmt', 'rgba', 'pipe:1'], { maxBuffer: 64 << 20 });
const oraclePcm = execFileSync('ffmpeg', ['-v', 'error', '-i', fixturePath, '-map', '0:a:0', '-f', 'f32le', 'pipe:1'], { maxBuffer: 64 << 20 });
const server = createServer((req, res) => {
  if (req.url === '/fixture') { res.setHeader('Content-Type', 'application/octet-stream'); res.end(fixture); }
  else if (req.url === '/worker.js') { res.setHeader('Content-Type', 'text/javascript'); res.end(bundle); }
  else { res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><title>Native browser media verification</title>'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
  browser = await chromium.launch({ executablePath: browserPath, headless: true });
  const page = await browser.newPage(), origin = `http://127.0.0.1:${server.address().port}`;
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
  await page.goto(origin);
  const value = await page.evaluate(async () => {
    const blob = await (await fetch('/fixture')).blob();
    const worker = new Worker('/worker.js', { type: 'module' });
    let timer;
    try { return await new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(new Error('media Worker verification deadline')), 45000);
      worker.onmessage = e => resolve(e.data); worker.onerror = e => reject(new Error(e.message));
      worker.postMessage(blob);
    }); } finally { clearTimeout(timer); worker.terminate(); }
  });
  await writeFile(path.join(destination, 'diagnostic.json'), JSON.stringify({ ...value, result: value.result && { ...value.result, frames: value.result.frames.map(x => ({ pts: x.pts, timescale: x.timescale })), audioBlocks: value.result.audioBlocks.map(({ values, ...x }) => x), audioSeek: value.result.audioSeek.map(({ values, ...x }) => ({ ...x, samples: values.length })) } }, null, 2));
  assert.equal(value.ok, true, JSON.stringify(value));
  const result = value.result;
  assert.equal(result.frames.length, oracleFrames.length);
  let sum = 0, maximum = 0;
  result.frames.forEach((frame, index) => {
    assert.equal(BigInt(frame.pts) * BigInt(stream.time_base.split('/')[1]), BigInt(oracleFrames[index].pts) * BigInt(frame.timescale));
    assert.equal(frame.rgba.length, frameSize);
    let local = 0;
    for (let i = 0; i < frameSize; i++) local += Math.abs(frame.rgba[i] - oraclePixels[index * frameSize + i]);
    const mae = local / frameSize; sum += mae; maximum = Math.max(maximum, mae);
  });
  assert(maximum < 5, `WebCodecs/FFmpeg RGBA difference ${maximum}`);
  for (const seek of result.seeks) {
    const index = seek.index - (seek.side === 'before' ? 1 : 0);
    assert.equal(seek.found, index >= 0);
    assert.equal(seek.observed, index >= 0 ? result.frames[index].pts : null);
  }
  const samples = result.audioBlocks.flatMap(x => x.values);
  const capturedPcm = Buffer.alloc(samples.length * 4);
  samples.forEach((value, index) => capturedPcm.writeFloatLE(value, index * 4));
  await writeFile(path.join(destination, 'decoded.f32le'), capturedPcm);
  await writeFile(path.join(destination, 'reference.f32le'), oraclePcm);
  assert.equal(samples.length * 4, oraclePcm.length);
  let square = 0, peak = 0;
  for (let i = 0; i < samples.length; i++) {
    const delta = Math.abs(samples[i] - oraclePcm.readFloatLE(i * 4)); square += delta ** 2; peak = Math.max(peak, delta);
  }
  const rms = Math.sqrt(square / samples.length);
  assert(rms < 0.0001 && peak < 0.001, `AAC decoder mismatch rms=${rms} peak=${peak}`);
  assert(result.audioSeek.length > 0);
  for (const block of result.audioSeek) for (let i = 0; i < block.values.length; i++)
    assert(Math.abs(block.values[i] - oraclePcm.readFloatLE((block.first + i) * 4)) < 0.001, 'audio seek lacks decoder preroll');
  assert.equal(result.beyond, false); assert.equal(result.callbackClosed, true); assert.equal(result.early, 1);
  assert.equal(result.busy, 'busy'); assert.equal(result.closed, 'closed'); assert.equal(result.cancelled, 'cancelled'); assert.equal(result.limit, 'resource_limit');
  assert.equal(result.abortedOpen, 'cancelled'); assert.equal(result.deadline, 'deadline');
  assert.equal(result.stats.videoDecoders + result.stats.audioDecoders + result.stats.videoFrames + result.stats.audioData, 0);
  assert.deepEqual(errors, []);
  const sha = data => createHash('sha256').update(data).digest('hex');
  const report = {
    format: 'musteroffice.browser-media-check/1', status: 'passed', browser: browser.version(),
    platform: process.platform, architecture: process.arch, fixtureSha256: sha(fixture), bundleSha256: sha(bundle),
    mediabunny: '1.61.3', worker: true, decodedVideoFrames: result.frames.length, exactSeeks: result.seeks.length,
    sdkManifestSha256: sdkPin ?? null,
    rgbaMeanError: sum / result.frames.length, rgbaMaxFrameError: maximum,
    pcmSamples: samples.length, pcmRmsError: rms, pcmPeakError: peak, ...result.stats,
    scope: 'Real Chromium Worker / H264 B-frame VFR and AAC decode. Independent FFmpeg PTS/pixels/PCM; before/after boundaries, EOF, early break, consumer failure, busy, cancellation, resource limits and release. Not full editor/render/export, platform matrix, total codec memory budget or product acceptance.',
  };
  await writeFile(path.join(destination, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
