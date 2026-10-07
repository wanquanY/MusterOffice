/** Real-codec test entry. Instrumentation is confined to this verification Worker. */
import { BrowserMediaInput, BrowserVideoBudget, type BrowserVideoSample, type BrowserVideoFrameLease } from '../../packages/video-browser-media/src/index.js';
const stats = { videoDecoders: 0, audioDecoders: 0, videoFrames: 0, audioData: 0, maxVideoFrames: 0, maxAudioData: 0, videoConfigured: 0, audioConfigured: 0, videoChunks: 0, audioChunks: 0 };
const held = new WeakSet<object>();
function hold(value: VideoFrame | AudioData, kind: 'videoFrames' | 'audioData') {
  if (held.has(value)) return;
  held.add(value); stats[kind]++;
  const peak = kind === 'videoFrames' ? 'maxVideoFrames' : 'maxAudioData';
  stats[peak] = Math.max(stats[peak], stats[kind]);
  const close = value.close.bind(value);
  value.close = () => { if (held.delete(value)) stats[kind]--; close(); };
  const clone = value.clone.bind(value);
  value.clone = (() => { const result = clone(); hold(result, kind); return result; }) as typeof value.clone;
}
for (const [name, counter, samples] of [
  ['VideoDecoder', 'videoDecoders', 'videoFrames'], ['AudioDecoder', 'audioDecoders', 'audioData'],
] as const) {
  const Platform = globalThis[name];
  // Verification only: constructors still invoke the browser's real codec.
  Object.assign(globalThis, { [name]: new Proxy(Platform, {
    construct(Target, args) {
      const config = args[0], output = config.output;
      const decoder = Reflect.construct(Target, [{ ...config, output(value: VideoFrame | AudioData) { hold(value, samples); output(value); } }]);
      stats[counter]++;
      stats[name === 'VideoDecoder' ? 'videoConfigured' : 'audioConfigured']++;
      const decode = decoder.decode.bind(decoder);
      decoder.decode = (chunk: EncodedVideoChunk | EncodedAudioChunk) => { stats[name === 'VideoDecoder' ? 'videoChunks' : 'audioChunks']++; decode(chunk); };
      let closed = false; const close = decoder.close.bind(decoder);
      decoder.close = () => { if (!closed) { closed = true; stats[counter]--; } close(); };
      return decoder;
    },
  }) });
}
const fraction = (n: string | number, d = 1) => ({ numerator: String(n), denominator: String(d) });
async function run(blob: Blob) {
  const media = await BrowserMediaInput.open(blob);
  const info = media.info;
  const video = info.tracks.find(x => x.kind === 'video')!, audio = info.tracks.find(x => x.kind === 'audio')!;
  const frames: { pts: string; timescale: number; rgba: number[] }[] = [];
  await media.videoRange(video.id, fraction(0), fraction(4), async (sample, span) => {
    const rgba = new Uint8Array(sample.width * sample.height * 4);
    await sample.copyRgba(rgba);
    frames.push({ pts: span.start.numerator, timescale: Number(span.start.denominator), rgba: Array.from(rgba) });
  });
  const seeks = [];
  for (const index of [0, 1, 2, 12, 25, 48, 60, frames.length - 1]) {
    for (const side of ['before', 'after'] as const) {
      const frame = frames[index]!;
      let observed: string | null = null;
      const found = await media.videoAt(video.id, { time: fraction(frame.pts, frame.timescale), side }, (_sample, span) => { observed = span.start.numerator; });
      seeks.push({ index, side, found, observed });
    }
  }
  const beyond = await media.videoAt(video.id, { time: fraction(5), side: 'after' }, () => { throw new Error('stretched frame past EOF'); });
  const audioBlocks: { timestamp: number; frames: number; sampleRate: number; values: number[] }[] = [];
  await media.audioRange(audio.id, fraction(0), fraction(4), (sample, span) => {
    const data = new Float32Array(sample.numberOfFrames * sample.numberOfChannels);
    sample.copyInterleaved(data);
    audioBlocks.push({ timestamp: Number(span.start.numerator) / Number(span.start.denominator), frames: sample.numberOfFrames, sampleRate: sample.sampleRate, values: Array.from(data) });
  });
  const audioSeek: { first: number; values: number[] }[] = [];
  await media.audioRange(audio.id, fraction(1011, 1000), fraction(1121, 1000), (sample, span) => {
    const data = new Float32Array(sample.numberOfFrames * sample.numberOfChannels);
    sample.copyInterleaved(data);
    audioSeek.push({ first: Number(BigInt(span.start.numerator) * BigInt(sample.sampleRate) / BigInt(span.start.denominator)), values: Array.from(data) });
  });
  const continuous = await BrowserMediaInput.open(blob);
  const initialAudioDecoders = stats.audioConfigured, initialAudioChunks = stats.audioChunks;
  const sequentialAudio: { first: number; values: number[] }[] = [];
  for (let offset = 0; offset < 3000; offset += 30) {
    await continuous.audioRange(audio.id, fraction(offset, 1000), fraction(offset + 30, 1000), (sample, span) => {
      const data = new Float32Array(sample.numberOfFrames * sample.numberOfChannels); sample.copyInterleaved(data);
      sequentialAudio.push({ first: Number(BigInt(span.start.numerator) * BigInt(sample.sampleRate) / BigInt(span.start.denominator)), values: Array.from(data) });
    });
  }
  const sequential = { decoderCreations: stats.audioConfigured - initialAudioDecoders, decodedChunks: stats.audioChunks - initialAudioChunks, windows: 100 };
  continuous.close();
  let callbackClosed = false;
  let borrowed: BrowserVideoSample | undefined;
  try { await media.videoRange(video.id, fraction(0), fraction(4), sample => { borrowed = sample; throw new Error('consumer failure'); }); }
  catch (error) { if ((error as Error).message !== 'consumer failure') throw error; }
  try { borrowed!.cloneFrame().close(); } catch { callbackClosed = true; }
  const early = await media.videoRange(video.id, fraction(0), fraction(4), () => false);
  let busy = '';
  await media.videoAt(video.id, { time: fraction(1), side: 'after' }, async () => {
    try { await media.videoAt(video.id, { time: fraction(0), side: 'after' }, () => {}); }
    catch (error) { busy = (error as { code: string }).code; }
  });
  media.close();
  let closed = '';
  try { await media.videoAt(video.id, { time: fraction(0), side: 'after' }, () => {}); }
  catch (error) { closed = (error as { code: string }).code; }
  const cancellation = new AbortController(), other = await BrowserMediaInput.open(blob);
  let cancelled = '';
  try { await other.videoRange(video.id, fraction(0), fraction(4), () => { cancellation.abort(); }, cancellation.signal); }
  catch (error) { cancelled = (error as { code: string }).code; }
  let limit = '';
  try { await BrowserMediaInput.open(blob, { limits: { maxPixels: 1 } }); }
  catch (error) { limit = (error as { code: string }).code; }
  const preCancelled = new AbortController(); preCancelled.abort();
  let abortedOpen = '';
  try { await BrowserMediaInput.open(blob, { signal: preCancelled.signal }); }
  catch (error) { abortedOpen = (error as { code: string }).code; }
  const expiring = await BrowserMediaInput.open(blob, { limits: { operationDeadlineMs: 1000 } });
  let deadline = '';
  try { await expiring.videoRange(video.id, fraction(0), fraction(4), () => new Promise(() => {})); }
  catch (error) { deadline = (error as { code: string }).code; }
  const tiny = new BrowserVideoBudget(1024), tinyInput = await BrowserMediaInput.open(blob, { videoBudget: tiny });
  const configuredBefore = stats.videoConfigured;
  let denied = '';
  try { await tinyInput.videoAt(video.id, { time: fraction(0), side: 'after' }, () => {}); }
  catch (error) { denied = (error as { code: string }).code; }
  const deniedBeforeConfigure = configuredBefore === stats.videoConfigured;
  tinyInput.close();
  const packetLimited = await BrowserMediaInput.open(blob, { limits: { maxEncodedPacketBytes: 1 } });
  const configuredBeforePacket = stats.videoConfigured;
  let packetDenied = '';
  try { await packetLimited.videoAt(video.id, { time: fraction(0), side: 'after' }, () => {}); }
  catch (error) { packetDenied = (error as { code: string }).code; }
  const packetDeniedBeforeConfigure = configuredBeforePacket === stats.videoConfigured;
  packetLimited.close();
  const expiringAudio = await BrowserMediaInput.open(blob, { limits: { operationDeadlineMs: 1000 } });
  let audioDeadline = '';
  try { await expiringAudio.audioRange(audio.id, fraction(0), fraction(4), () => new Promise(() => {})); }
  catch (error) { audioDeadline = (error as { code: string }).code; }
  const shared = new BrowserVideoBudget(16 * 1024 ** 2);
  const one = await BrowserMediaInput.open(blob, { videoBudget: shared }), two = await BrowserMediaInput.open(blob, { videoBudget: shared });
  let concurrentDenied = '', clone: BrowserVideoFrameLease | undefined;
  await one.videoAt(video.id, { time: fraction(0), side: 'after' }, async sample => {
    clone = sample.cloneFrame();
    try { await two.videoAt(video.id, { time: fraction(0), side: 'after' }, () => {}); }
    catch (error) { concurrentDenied = (error as { code: string }).code; }
  });
  one.close();
  for (let i = 0; i < 100 && shared.usedBytes > 128 * 128 * 8; i++) await new Promise(resolve => setTimeout(resolve, 10));
  const retainedCloneBytes = shared.usedBytes, cloneWidth = clone!.frame.codedWidth;
  clone!.close(); clone!.close();
  await two.videoAt(video.id, { time: fraction(1), side: 'after' }, () => {});
  two.close();
  // Give actual decoder callbacks/iterator cleanup time to drain after cancellation.
  for (let i = 0; i < 100 && (stats.videoDecoders || stats.audioDecoders || stats.videoFrames || stats.audioData); i++)
    await new Promise(resolve => setTimeout(resolve, 10));
  return { info, frames, seeks, beyond, audioBlocks, audioSeek, sequentialAudio, sequential, callbackClosed, early, busy, closed, cancelled, limit, abortedOpen, deadline, audioDeadline,
    budget: { denied, deniedBeforeConfigure, packetDenied, packetDeniedBeforeConfigure, concurrentDenied, retainedCloneBytes, cloneWidth, finalBytes: shared.usedBytes, peakBytes: shared.peakBytes, capacityBytes: shared.capacityBytes }, stats };
}
self.onmessage = async event => {
  try { self.postMessage({ ok: true, result: await run(event.data) }); }
  catch (error) { self.postMessage({ ok: false, error: String(error), stack: (error as Error).stack, stats }); }
};
