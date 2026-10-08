/** Browser codec scheduling only. Exact work/ACK comes from ProjectExport;
 * PCM, composition and I420 are supplied by the shared render/mix components.
 * A successful result is a candidate, never a storage/publication receipt.
 */
export async function encodeProject({ plan, runtime, sink, createFrame, createPcm, signal, limits,
  codec = 'avc1.640028', budgets = {}, onProgress = () => {} }) {
  const info = JSON.parse(plan.info());
  limits = { ...limits };
  const budget = { maxFramesInFlight:16, maxVideoInputBytes:268435456, maxCodecQueue:4, maxBufferedPcmBytes:1048576, stallTimeoutMs:30000, ...budgets };
  const need = (ok, message) => { if (!ok) throw Error(message); };
  for (const [name, max] of [['maxFramesInFlight',64], ['maxVideoInputBytes',1073741824], ['maxCodecQueue',64], ['maxBufferedPcmBytes',16777216], ['stallTimeoutMs',60000]])
    need(Number.isInteger(budget[name]) && budget[name] > 0 && budget[name] <= max, 'invalid codec budget: ' + name);
  need(info.outputProfile === 'h264WebCodecsAacMp4' && /^avc1\.64[0-9a-f]{4}$/i.test(codec), 'explicit H264 High profile required');
  const frameBytes = info.width * info.height * 3 / 2;
  need(Number.isSafeInteger(frameBytes) && frameBytes > 0 && frameBytes <= budget.maxVideoInputBytes, 'video input byte budget');
  const config = { codec, width:info.width, height:info.height, bitrate:info.videoBitRate,
    framerate:info.frameRateNumerator / info.frameRateDenominator, latencyMode:'quality',
    hardwareAcceleration:'prefer-hardware', avc:{ format:'avc' } };
  const frameJobs = [], pcmJobs = [], waiters = new Set();
  const preparation = new AbortController();
  let encoder, mux, failure, description, configIdentity, bufferedPcmBytes = 0, submitted = 0, received = 0;
  let peakFramesInFlight = 0, peakBufferedPcmBytes = 0;
  const wake = () => { for (const check of [...waiters]) check(); };
  const fail = error => { failure ??= error instanceof Error ? error : Error(String(error)); wake(); };
  const aborted = () => fail(signal?.reason ?? new DOMException('Export cancelled', 'AbortError'));
  const check = () => { if (signal?.aborted) aborted(); if (failure) throw failure; };
  const wait = async predicate => {
    check(); if (predicate()) return;
    await new Promise((resolve, reject) => {
      const finish = action => { clearTimeout(timer); waiters.delete(test); action(); };
      const test = () => { if (failure) finish(() => reject(failure)); else if (predicate()) finish(resolve); };
      const timer = setTimeout(() => finish(() => reject(Error('browser encoder made no progress within its budget'))), budget.stallTimeoutMs);
      waiters.add(test); test();
    });
    check();
  };
  const prepare = async (produce, dispose = () => {}) => {
    let settled = false, abandoned = false, value, error;
    // The host receives cancellation for its own resource work. A late frame
    // still gets closed even if a driver/host promise ignores cancellation.
    Promise.resolve().then(() => { check(); return produce(preparation.signal); }).then(result => {
      if (abandoned) dispose(result); else { value = result; settled = true; wake(); }
    }, cause => { error = cause; settled = true; wake(); });
    try { await wait(() => settled); if (error) throw error; return value; }
    catch (cause) { abandoned = true; if (settled && value !== undefined) dispose(value); throw cause; }
  };
  const microseconds = value => {
    const n = Number(value); need(Number.isSafeInteger(n) && n >= 0, 'unsafe codec timestamp'); return n;
  };
  const metadata = value => {
    need(value && value.description && value.codec?.startsWith('avc1.') && value.codedWidth === info.width
      && value.codedHeight === info.height, 'unexpected AVC decoder configuration');
    const color = value.colorSpace;
    need(color?.primaries === 'bt709' && color.transfer === 'bt709' && color.matrix === 'bt709' && color.fullRange === false,
      'encoded color configuration differs from I420 input');
    const bytes = ArrayBuffer.isView(value.description)
      ? new Uint8Array(value.description.buffer, value.description.byteOffset, value.description.byteLength) : new Uint8Array(value.description);
    need(bytes.length >= 7 && bytes.length <= 65536 && bytes[0] === 1 && bytes[1] === 100 && (bytes[4] & 3) === 3,
      'unexpected AVC High description');
    const identity = JSON.stringify([value.codec, value.codedWidth, value.codedHeight, Array.from(bytes)]);
    if (configIdentity) need(identity === configIdentity, 'encoder configuration changed during export');
    else { configIdentity = identity; description = bytes.slice(); }
  };
  const output = (chunk, data) => {
    if (failure) return;
    try {
      check(); if (data.decoderConfig) metadata(data.decoderConfig);
      need(description && frameJobs.length, 'unbound video output');
      const { work, transport } = frameJobs[0];
      need(chunk.timestamp === microseconds(transport.timestampUs), 'encoder reordered or changed a presentation timestamp');
      need(chunk.duration == null || chunk.duration === microseconds(transport.durationUs), 'encoder changed a frame duration');
      need(chunk.byteLength > 0 && chunk.byteLength <= limits.maxPacketBytes, 'encoded packet budget');
      need(!work.keyFrame || chunk.type === 'key', 'requested random access frame missing');
      const packet = new Uint8Array(chunk.byteLength); chunk.copyTo(packet);
      if (!mux) {
        mux = runtime.open(info, limits, { writeAt:(at, bytes) => sink.writeAt(at, bytes), cancelled:() => signal?.aborted || sink.cancelled() }, description);
        for (const { firstSample, pcm } of pcmJobs) mux.audio(firstSample, pcm);
        pcmJobs.length = 0; bufferedPcmBytes = 0;
      }
      mux.video(work.pts, work.duration, packet, chunk.type === 'key');
      frameJobs.shift(); received++; wake();
    } catch (error) { fail(error); }
  };
  signal?.addEventListener('abort', aborted, { once:true });
  try {
    check(); need((await VideoEncoder.isConfigSupported(config)).supported, 'H264 configuration is unavailable on this browser'); check();
    encoder = new VideoEncoder({ output, error:fail }); encoder.addEventListener('dequeue', wake); encoder.configure(config);
    for (;;) {
      check();
      const { work, transport } = JSON.parse(plan.next());
      if (work.kind === 'complete') break;
      if (work.kind === 'video') {
        await wait(() => frameJobs.length < budget.maxFramesInFlight && (frameJobs.length + 1) * frameBytes <= budget.maxVideoInputBytes
          && encoder.encodeQueueSize < budget.maxCodecQueue);
        const frame = await prepare(signal => createFrame(work, transport, signal), frame => frame?.close());
        try {
          check();
          need(frame instanceof VideoFrame && frame.format === 'I420' && frame.codedWidth === info.width && frame.codedHeight === info.height
            && frame.timestamp === microseconds(transport.timestampUs) && frame.duration === microseconds(transport.durationUs), 'video frame differs from exact export work');
          need(frame.colorSpace.primaries === 'bt709' && frame.colorSpace.transfer === 'bt709' && frame.colorSpace.matrix === 'bt709'
            && frame.colorSpace.fullRange === false, 'I420 color profile required');
          frameJobs.push({ work, transport }); peakFramesInFlight = Math.max(peakFramesInFlight, frameJobs.length);
          encoder.encode(frame, { keyFrame:work.keyFrame }); submitted++;
        } finally { frame?.close(); }
      } else if (work.kind === 'audio') {
        const bytes = work.frames * info.audioChannels * 4;
        need(Number.isSafeInteger(bytes) && bytes > 0 && bytes <= budget.maxBufferedPcmBytes, 'PCM block budget');
        await wait(() => mux || bufferedPcmBytes + bytes <= budget.maxBufferedPcmBytes);
        const pcm = await prepare(signal => createPcm(work, signal)); check();
        need(pcm instanceof Float32Array && pcm.byteLength === bytes, 'PCM block differs from exact export work');
        if (mux) mux.audio(work.firstSample, pcm);
        else {
          pcmJobs.push({ firstSample:work.firstSample, pcm:pcm.slice() }); bufferedPcmBytes += bytes;
          peakBufferedPcmBytes = Math.max(peakBufferedPcmBytes, bufferedPcmBytes);
        }
      } else throw Error('unknown export work');
      check(); plan.accept(JSON.stringify(work)); onProgress(JSON.parse(plan.progress()));
    }
    let flushed = false;
    // Only one terminal flush. Waiting is bounded and abort-aware even when the
    // browser leaves the platform flush promise pending.
    encoder.flush().then(() => { flushed = true; wake(); }, fail);
    await wait(() => flushed);
    need(submitted === info.videoFrames && received === submitted && !frameJobs.length && !pcmJobs.length && mux,
      'encoder did not complete all exact work');
    check(); const stats = mux.finish(); check();
    return Object.freeze({ stats, documentDigest:info.documentDigest, peakFramesInFlight,
      peakVideoInputBytes:peakFramesInFlight * frameBytes, peakBufferedPcmBytes });
  } finally {
    preparation.abort(failure ?? new Error('Export computation ended'));
    signal?.removeEventListener('abort', aborted);
    if (encoder) { encoder.removeEventListener('dequeue', wake); if (encoder.state !== 'closed') encoder.close(); }
    mux?.close(); pcmJobs.length = 0; frameJobs.length = 0; waiters.clear();
  }
}
