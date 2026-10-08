/** Shared Rust selections followed by actual browser decode, independently
 * checked against Fraction mapping and FFmpeg raw PTS/pixels. No product UI claim. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { chromium } from 'playwright-core';
import { verify } from '../video-browser-sdk/build.mjs';
const root = fileURLToPath(new URL('../../', import.meta.url));
const [browserPath, mediaSdk, mediaPin, indexSdk, indexPin, native, output, ...fixtures] = process.argv.slice(2);
assert(browserPath && mediaSdk && mediaPin && indexSdk && indexPin && native && output && fixtures.length,
  'usage: check BROWSER MEDIA_SDK MEDIA_PIN INDEX_SDK INDEX_PIN NATIVE_INDEX NEW_OUTPUT FIXTURE...');
await verify(mediaSdk, mediaPin);
execFileSync('python3', ['tools/media-index-sdk/build.py', '--verify', indexSdk, '--sha256', indexPin], { cwd: root });
await mkdir(output, { recursive: false });
const adversarial = JSON.parse(execFileSync('python3', ['tools/verification/video-browser-movie-fixtures.py', fixtures[0], path.join(output, 'adversarial')], { cwd: root }));
fixtures.push(adversarial.falseSync, adversarial.openGop);
const missingIdr = await readFile(adversarial.missingIdr);
const sha = bytes => createHash('sha256').update(bytes).digest('hex'), inputs = [];
async function nativeQuery(file, requests, name) {
  const location = path.resolve(output, name + '-requests.json'); await writeFile(location, JSON.stringify(requests));
  const values = JSON.parse(execFileSync(path.resolve(native), [path.resolve(file), location], { maxBuffer: 8 << 20 }));
  assert(values.every(v => v.ok), 'native reference query failed'); return values.map(v => v.value);
}
for (const [i, file] of fixtures.entries()) {
  const bytes = await readFile(file);
  const probe = JSON.parse(execFileSync('ffprobe', ['-v', 'error', '-ignore_editlist', '1', '-show_frames', '-show_streams', '-of', 'json', file], { maxBuffer: 8 << 20 }));
  const ordinal = probe.streams.findIndex(s => s.codec_type === 'video'), stream = probe.streams[ordinal];
  assert(ordinal >= 0);
  const frames = probe.frames.filter(f => f.media_type === 'video' && f.stream_index === stream.index);
  const initial = await nativeQuery(file, [
    { method: 'query', body: { type: 'track', track: ordinal } }, { method: 'videoTiming', body: ordinal },
    { method: 'query', body: { type: 'samples', track: ordinal, start: 0, count: 128 } },
    { method: 'query', body: { type: 'edits', track: ordinal, start: 0, count: 128 } },
  ], `${i}-metadata`);
  assert.equal(initial[0].track.samples, initial[2].items.length, 'fixture too large for this small corpus');
  const data = { ordinal, track: initial[0].track, summary: initial[1], samples: initial[2].items, edits: initial[3].items };
  const location = path.join(output, `${i}-mapping.json`); await writeFile(location, JSON.stringify(data));
  const requests = JSON.parse(execFileSync('python3', ['tools/verification/video-browser-movie-queries.py', location], { cwd: root }));
  const expected = await nativeQuery(file, requests, `${i}-selections`);
  data.queries = requests.map((request, at) => ({ ...request.body, selection: expected[at] }));
  await writeFile(location, JSON.stringify(data));
  execFileSync('python3', ['tools/verification/media-index-mapping-check.py', location], { cwd: root });
  const pixels = execFileSync('ffmpeg', ['-v', 'error', '-ignore_editlist', '1', '-copyts', '-i', file,
    '-map', `0:${stream.index}`, '-fps_mode', 'passthrough', '-avoid_negative_ts', 'disabled', '-pix_fmt', 'rgba', '-f', 'rawvideo', 'pipe:1'], { maxBuffer: 64 << 20 });
  assert.equal(pixels.length, frames.length * stream.width * stream.height * 4);
  inputs.push({ file, bytes, pixels, frames, requests, expected, id: data.track.id, packets: data.samples, edits: data.edits });
}
const worker = await readFile(new URL('video-browser-movie.worker.mjs', import.meta.url));
const cursorCases = await readFile(new URL('video-browser-cursor-cases.mjs', import.meta.url));
const sdkFiles = new Map();
for (const [route, directory] of [['/media-sdk/', mediaSdk], ['/index-sdk/', indexSdk]]) {
  const manifest = JSON.parse(await readFile(path.join(directory, 'bundle-manifest.json')));
  for (const entry of manifest.files) sdkFiles.set(route + entry.path, path.join(directory, entry.path));
}
const server = createServer(async (req, res) => {
  try {
    if (req.url === '/worker.mjs') { res.setHeader('Content-Type', 'text/javascript'); res.end(worker); }
    else if (req.url === '/video-browser-cursor-cases.mjs') { res.setHeader('Content-Type', 'text/javascript'); res.end(cursorCases); }
    else if (sdkFiles.has(req.url)) {
      res.setHeader('Content-Type', req.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript'); res.end(await readFile(sdkFiles.get(req.url)));
    } else if (req.url === '/missing-idr') { res.end(missingIdr);
    } else if (/^\/fixture\/\d+\/(bytes|pixels)$/.test(req.url)) {
      const [, , id, kind] = req.url.split('/'); res.end(inputs[Number(id)][kind]);
    } else { res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><title>Indexed movie decode verification</title>'); }
  } catch { res.statusCode = 404; res.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
  browser = await chromium.launch({ executablePath: browserPath, headless: true });
  const page = await browser.newPage(), origin = `http://127.0.0.1:${server.address().port}`, errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
  await page.goto(origin);
  const actual = await page.evaluate(async metadata => {
    const module = await WebAssembly.compile(await (await fetch('/index-sdk/runtime/mo_media_index_wasm_bg.wasm')).arrayBuffer());
    const fixtures = await Promise.all(metadata.map(async (row, i) => ({ ...row,
      blob: await (await fetch(`/fixture/${i}/bytes`)).blob(), pixels: await (await fetch(`/fixture/${i}/pixels`)).arrayBuffer() })));
    const worker = new Worker('/worker.mjs', { type: 'module' }); let timer;
    try { return await new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(Error('indexed movie Worker deadline')), 120000);
      worker.onmessage = e => resolve(e.data); worker.onerror = e => reject(Error(e.message));
      fetch('/missing-idr').then(r => r.blob()).then(missingIdr => worker.postMessage({ module, fixtures, missingIdr }, fixtures.map(f => f.pixels))).catch(reject);
    }); } finally { clearTimeout(timer); worker.terminate(); }
  }, inputs.map(({ id, frames, requests, packets }) => ({ id, frames: frames.map(f => ({ pts: f.pts, chroma:f.chroma_location })), requests, packets })));
  await writeFile(path.join(output, 'actual.json'), JSON.stringify(actual)); assert(actual.ok, JSON.stringify(actual));
  const reports = inputs.map((input, i) => {
    const rows = actual.results[i].rows; assert.equal(rows.length, input.expected.length);
    let pictures = 0, maximum = 0, sum = 0;
    rows.forEach((row, at) => {
      assert.deepEqual(row.selection, input.expected[at]);
      if (row.selection.type === 'sample') {
        pictures++; assert.equal(row.called, 1); assert.deepEqual(row.span, row.selection.movie);
        assert(row.mae !== null && row.mae < 5, `wrong frame/pixels fixture ${i} query ${at}: ${row.mae}`);
        maximum = Math.max(maximum, row.mae); sum += row.mae;
      } else { assert.equal(row.called, 0); assert.equal(row.configured, 0); }
    });
    // A normal forward corpus has one stream; the repeated edit corpus needs
    // exactly one restart. Dwell/before/after do not recreate decoder state.
    assert.equal(actual.results[i].configured, input.edits.length > 1 ? 2 : 1);
    assert(actual.results[i].chunks <= input.frames.length * (input.edits.length > 1 ? 2 : 1));
    assert.equal(actual.results[i].randomAccess.length, 3);
    return { file: path.basename(input.file), sha256: sha(input.bytes), movieQueries: rows.length, decodedPictures: pictures,
      rgbaMeanError: sum / pictures, rgbaMaxFrameError: maximum, movieInfo: actual.results[i].info,
      decoderCreations: actual.results[i].configured, decodedChunks: actual.results[i].chunks, randomAccess: actual.results[i].randomAccess };
  });
  assert.deepEqual(errors, []); assert.equal(actual.stats.liveFrames + actual.stats.liveDecoders, 0);
  assert.equal(actual.budget.used, 0); assert(actual.budget.peak <= actual.budget.capacity);
  assert.deepEqual(actual.packetLimit, { rejected: 'resource_limit', configured: 0 });
  assert.deepEqual(actual.missingIdr, { rejected: 'unsupported_media', configured: 0 });
  assert.equal(actual.deadline, 'deadline'); assert.equal(actual.cancelled, 'cancelled');
  const report = { status: 'passed', browser: browser.version(), platform: process.platform, architecture: process.arch,
    mediaManifestSha256: mediaPin, indexManifestSha256: indexPin, reports, stats: actual.stats, corpusStats: actual.corpusStats, cursor: actual.cursor, budget: actual.budget,
    openGopNonIdrCandidates: adversarial.nonIdrCandidates,
    failures: { packetLimit: actual.packetLimit, missingIdr: { ...actual.missingIdr, sha256: sha(missingIdr) }, deadline: actual.deadline, cancelled: actual.cancelled },
    scope: 'Actual Worker/Rust index/raw AVC/WebCodecs movie selection and independent FFmpeg pixels plus Fraction oracle. No audio movie decode, compositor, export or product acceptance.' };
  await writeFile(path.join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n'); console.log(JSON.stringify(report));
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
