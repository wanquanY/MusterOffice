/** Actual codec evidence used to design the exact finite browser mux contract.
 * Test-only bounded packet capture; it is not a production output adapter. */
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { chromium } from 'playwright-core';
import { createHash } from 'node:crypto';

const [componentArg, vectorsArg, outputArg] = process.argv.slice(2);
assert(componentArg && vectorsArg && outputArg, 'EXPORT_COMPONENT NATIVE_SCHEDULE_VECTORS NEW_OUTPUT required');
const root = resolve(componentArg), output = resolve(outputArg); await mkdir(output);
const manifestBytes = await readFile(root + '/bundle-manifest.json');
const routes = new Map([['/vectors.json', await readFile(vectorsArg)]]);
for (const row of JSON.parse(manifestBytes).files) if (/\.(js|mjs|wasm)$/.test(row.path)) routes.set('/sdk/' + row.path, await readFile(root + '/' + row.path));
routes.set('/worker.mjs', Buffer.from(`(${workerMain.toString()})()`));
const server = createServer((req, res) => {
  if (req.url === '/') { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<!doctype html><title>Codec evidence</title>'); return; }
  const value = routes.get(req.url); if (!value) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': req.url.endsWith('.wasm') ? 'application/wasm' : /\.m?js$/.test(req.url) ? 'text/javascript' : 'application/json' }); res.end(value);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true });
try {
  const page = await browser.newPage(), errors = []; page.on('pageerror', error => errors.push(String(error)));
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const cases = await page.evaluate(() => new Promise((yes, no) => {
    const worker = new Worker('/worker.mjs', { type: 'module' });
    const timer = setTimeout(() => { worker.terminate(); no(Error('codec evidence deadline')); }, 30000);
    const done = run => { clearTimeout(timer); worker.terminate(); run(); };
    worker.onmessage = event => done(() => event.data.error ? no(Error(event.data.error)) : yes(event.data.cases));
    worker.onerror = event => done(() => no(Error(event.message)));
  }));
  assert.deepEqual(errors, []);
  const report = { format: 'musteroffice.browser-encoder-probe/1', browser: browser.version(),
    planManifestSha256: createHash('sha256').update(manifestBytes).digest('hex'), cases,
    scope: 'Actual WebCodecs encode/decode timestamps, packet descriptions and PCM. No mux, output file or product acceptance.' };
  await writeFile(output + '/report.json', JSON.stringify(report) + '\n');
  console.log(JSON.stringify({ browser: report.browser, cases: cases.map(c => ({
    frameRate: [c.info.frameRateNumerator,c.info.frameRateDenominator], samples:c.info.audioSamples, channels:c.info.audioChannels,
    videoPackets:c.video.length, videoReordered:c.video.some((p,i)=>i>0&&p.timestamp<c.video[i-1].timestamp),
    audioPackets:c.audio.length, audioFirst:c.audio[0]?.timestamp, audioLast:c.audio.at(-1)?.timestamp,
    audioLastDuration:c.audio.at(-1)?.duration, decodedFrames:c.decoded.reduce((n,p)=>n+p.frames,0),
    audioConfig:c.audioConfig, videoConfig:c.videoConfig,
  })) }));
} catch (error) { await writeFile(output + '/failure.txt', String(error.stack ?? error)); throw error; }
finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }

async function workerMain() {
  try {
    const { createExportRuntime } = await import('/sdk/index.mjs');
    const runtime = await createExportRuntime(await WebAssembly.compileStreaming(fetch('/sdk/runtime/mo_video_export_wasm_bg.wasm')));
    const vectors = await (await fetch('/vectors.json')).json(), cases = [];
    const need = (ok, message) => { if (!ok) throw Error(message); };
    const sample = (at, channel, rate) => 0.25 * Math.sin(2 * Math.PI * (channel ? 937 : 431) * at / rate)
      * (at < 256 || at % 8192 < 128 ? 0 : 1);
    for (const vector of vectors.cases) {
      const plan = runtime.ProjectExport.open(JSON.stringify(vector.document), vector.documentDigest, JSON.stringify(vector.options));
      const info = JSON.parse(plan.info()), video = [], audio = [], decoded = [];
      let videoConfig, audioConfig, error;
      const failure = e => { error = e; };
      const capacity = async codec => {
        if (error) throw error;
        if (codec.encodeQueueSize < 4) return;
        await new Promise((yes, no) => {
          const check = () => { if (error || codec.state === 'closed') finish(() => no(error ?? Error('closed'))); else if (codec.encodeQueueSize < 4) finish(yes); };
          const finish = run => { clearTimeout(timer); codec.removeEventListener('dequeue', check); run(); };
          const timer = setTimeout(() => finish(() => no(Error('codec queue deadline'))), 3000);
          codec.addEventListener('dequeue', check); check();
        });
      };
      const capture = (destination, chunk) => {
        need(destination.length < 1000 && chunk.byteLength < 1 << 20, 'probe packet budget');
        const data = new Uint8Array(chunk.byteLength); chunk.copyTo(data);
        destination.push({ timestamp: chunk.timestamp, duration: chunk.duration, type: chunk.type, data: Array.from(data) });
      };
      const configCopy = config => ({ ...config, description: config.description ? Array.from(new Uint8Array(config.description)) : null });
      const videoEncoder = new VideoEncoder({ output(chunk, metadata) {
        try { capture(video, chunk); if (metadata.decoderConfig) videoConfig = configCopy(metadata.decoderConfig); } catch (e) { failure(e); }
      }, error: failure });
      const vc = { codec: 'avc1.640028', width: info.width, height: info.height,
        bitrate: info.videoBitRate, framerate: info.frameRateNumerator / info.frameRateDenominator,
        latencyMode: 'quality', avc: { format: 'avc' } };
      need((await VideoEncoder.isConfigSupported(vc)).supported, 'H264 unsupported'); videoEncoder.configure(vc);
      const audioEncoder = info.audioChannels ? new AudioEncoder({ output(chunk, metadata) {
        try { capture(audio, chunk); if (metadata.decoderConfig) audioConfig = configCopy(metadata.decoderConfig); } catch (e) { failure(e); }
      }, error: failure }) : null;
      if (audioEncoder) {
        const ac = { codec: 'mp4a.40.2', sampleRate: info.audioSampleRate, numberOfChannels: info.audioChannels, bitrate: info.audioBitRate, aac: { format: 'aac' } };
        need((await AudioEncoder.isConfigSupported(ac)).supported, 'AAC unsupported'); audioEncoder.configure(ac);
      }
      try {
        while (true) {
          const { work, transport } = JSON.parse(plan.next());
          if (work.kind === 'complete') break;
          if (work.kind === 'video') {
            await capacity(videoEncoder);
            const pixels = new Uint8Array(info.width * info.height * 3 / 2);
            for (let y = 0; y < info.height; y++) for (let x = 0; x < info.width; x++) pixels[y * info.width + x] = 16 + ((x + y + work.index * 5) % 220);
            pixels.fill(100, info.width * info.height, info.width * info.height * 5 / 4);
            pixels.fill(150, info.width * info.height * 5 / 4);
            const frame = new VideoFrame(pixels, { format: 'I420', codedWidth: info.width, codedHeight: info.height,
              timestamp: Number(transport.timestampUs), duration: Number(transport.durationUs),
              colorSpace: { primaries: 'bt709', transfer: 'bt709', matrix: 'bt709', fullRange: false } });
            try { videoEncoder.encode(frame, { keyFrame: work.keyFrame }); } finally { frame.close(); }
          } else {
            await capacity(audioEncoder);
            const data = new Float32Array(work.frames * info.audioChannels), start = Number(work.firstSample);
            for (let i = 0; i < work.frames; i++) for (let c = 0; c < info.audioChannels; c++) data[i * info.audioChannels + c] = sample(start + i, c, info.audioSampleRate);
            const frame = new AudioData({ format: 'f32', data, numberOfFrames: work.frames,
              sampleRate: info.audioSampleRate, numberOfChannels: info.audioChannels, timestamp: Number(transport.timestampUs) });
            try { audioEncoder.encode(frame); } finally { frame.close(); }
          }
          if (error) throw error;
          plan.accept(JSON.stringify(work));
        }
        await Promise.all([videoEncoder.flush(), audioEncoder?.flush()]);
        if (error) throw error;
        need(video.length === info.videoFrames, 'missing encoded video frame');
        if (audioEncoder) {
          const decoder = new AudioDecoder({ output(frame) {
            try {
              const pcm = new Float32Array(frame.numberOfFrames * frame.numberOfChannels);
              frame.copyTo(pcm, { format: 'f32', planeIndex: 0 });
              decoded.push({ timestamp: frame.timestamp, frames: frame.numberOfFrames, data: Array.from(pcm) });
            } catch (e) { failure(e); } finally { frame.close(); }
          }, error: failure });
          try {
            decoder.configure({ ...audioConfig, description: new Uint8Array(audioConfig.description) });
            for (const packet of audio) decoder.decode(new EncodedAudioChunk({ ...packet, data: new Uint8Array(packet.data) }));
            await decoder.flush(); if (error) throw error;
          } finally { if (decoder.state !== 'closed') decoder.close(); }
        }
        cases.push({ info, videoConfig, audioConfig, video, audio, decoded });
      } finally { plan.free(); if (videoEncoder.state !== 'closed') videoEncoder.close(); if (audioEncoder && audioEncoder.state !== 'closed') audioEncoder.close(); }
    }
    postMessage({ cases });
  } catch (error) { postMessage({ error: String(error.stack ?? error) }); }
}
