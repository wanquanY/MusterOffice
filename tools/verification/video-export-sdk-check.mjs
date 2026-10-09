/** Public component, actual WASM, isolated Worker; no private crate imports. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';

if (isMainThread) {
  const [componentArg, vectorArg, outputArg] = process.argv.slice(2);
  if (!componentArg || !vectorArg || !outputArg) throw new Error('COMPONENT NATIVE_VECTORS NEW_OUTPUT required');
  const component = resolve(componentArg), output = resolve(outputArg);
  await mkdir(output, { recursive: false });
  const vectors = await readFile(vectorArg), wasm = await readFile(join(component, 'runtime/mo_video_export_wasm_bg.wasm'));
  const module = await WebAssembly.compile(wasm);
  const worker = new Worker(new URL(import.meta.url), { workerData: { component, module, vectors: JSON.parse(vectors) } });
  const timeout = setTimeout(() => { void worker.terminate(); }, 30000);
  let received;
  const report = await new Promise((yes, no) => {
    worker.once('message', value => { received = value; });
    worker.once('error', no);
    worker.once('exit', code => code === 0 && received ? yes(received) : no(new Error(`Worker exit ${code}`)));
  }).finally(() => clearTimeout(timeout));
  Object.assign(report, { node: process.version,
    componentManifestSha256: createHash('sha256').update(await readFile(join(component, 'bundle-manifest.json'))).digest('hex'),
    nativeVectorSha256: createHash('sha256').update(vectors).digest('hex'),
    wasmBytes: wasm.length, wasmGzipBytes: gzipSync(wasm).length });
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
} else {
  const { component, module, vectors } = workerData;
  const { createExportRuntime } = await import(pathToFileURL(join(component, 'index.mjs')).href);
  const runtime = await createExportRuntime(module);
  assert.equal(runtime.profile, 'musteroffice.project-export-worker/1-draft');
  await assert.rejects(createExportRuntime(module), /fresh Worker/);
  const rejected = (run, code) => assert.throws(run, value => JSON.parse(String(value)).code === code);
  const open = v => runtime.ProjectExport.open(JSON.stringify(v.document), v.documentDigest, JSON.stringify(v.options));
  const probe = JSON.parse(runtime.ProjectExport.probeFixture());
  assert.equal(probe.format, 'musteroffice.video.export-probe-fixture/1');
  assert.deepEqual(probe.document.resources, {});
  const measurement = open(probe), measurementInfo = JSON.parse(measurement.info());
  assert.deepEqual([measurementInfo.width, measurementInfo.height, measurementInfo.videoFrames,
    measurementInfo.audioChannels, measurementInfo.audioSamples], [128, 96, 6, 2, '9600']);
  assert.equal(measurementInfo.documentDigest, probe.documentDigest);
  let probeSteps = 0;
  for (;;) {
    const { work } = JSON.parse(measurement.next());
    if (work.kind === 'complete') break;
    assert.ok(++probeSteps <= 10);
    measurement.accept(JSON.stringify(work));
  }
  assert.deepEqual(JSON.parse(measurement.progress()), { videoFrames:6, audioSamples:'9600' });
  measurement.free();
  let steps = 0, completed = 0;
  for (const c of vectors.cases) {
    const project = open(c), info = JSON.parse(project.info());
    assert.equal(info.documentDigest, c.documentDigest);
    assert.equal(info.outputProfile, 'h264WebCodecsAacMp4');
    for (const [key, value] of Object.entries(c.expected)) assert.deepEqual(info[key], value, key);
    assert.equal(typeof info.videoDuration, 'string');
    assert.equal(typeof info.audioSamples, 'string');
    assert.deepEqual(JSON.parse(project.progress()), { videoFrames: 0, audioSamples: '0' });
    for (const expected of c.steps) {
      const first = project.next();
      assert.deepEqual(JSON.parse(first), expected);
      assert.equal(project.next(), first, 'pending work remains stable');
      const progress = project.progress();
      rejected(() => project.accept(JSON.stringify({ ...expected.work, unexpected: true })), 'invalid_request');
      assert.equal(project.progress(), progress);
      if (expected.work.kind === 'complete') {
        rejected(() => project.accept(JSON.stringify(expected.work)), 'invalid_request');
        completed++; break;
      }
      const altered = { ...expected.work };
      if (altered.kind === 'video') altered.duration = String(BigInt(altered.duration) + 1n);
      else altered.firstSample = String(BigInt(altered.firstSample) + 1n);
      rejected(() => project.accept(JSON.stringify(altered)), 'invalid_request');
      assert.equal(project.progress(), progress);
      project.accept(JSON.stringify(expected.work));
      rejected(() => project.accept(JSON.stringify(expected.work)), 'invalid_request');
      steps++;
    }
    assert.deepEqual(JSON.parse(project.progress()), c.progress);
    assert.equal(project.info(), JSON.stringify(info));
    project.free();
  }
  const c = vectors.cases[0];
  rejected(() => open({ ...c, documentDigest: 'sha256:' + '0'.repeat(64) }), 'invalid_request');
  for (const options of [
    { ...c.options, unexpected: true }, { ...c.options, videoBitRate: 0 },
    { ...c.options, gopFrames: 0 }, { ...c.options, audioBlockFrames: 0 },
    { ...c.options, audioBlockFrames: 65537 }, { ...c.options, sourcePeakNits: 1 },
    { ...c.options, audio: { ...c.options.audio, unexpected: true } },
  ]) rejected(() => open({ ...c, options }), 'invalid_request');
  rejected(() => open(vectors.unrepresentableTail), 'unsupported_capability');
  rejected(() => runtime.ProjectExport.open(' '.repeat((32 << 20) + 1), c.documentDigest, '{}'), 'resource_limit');
  const project = open(c);
  rejected(() => project.accept(' '.repeat(1025)), 'resource_limit');
  assert.deepEqual(JSON.parse(project.progress()), { videoFrames: 0, audioSamples: '0' });
  project.free();
  parentPort.postMessage({ status: 'passed', cases: completed, exactStepsCompared: steps,
    checks: ['native/wasm exact schedule and transport residuals', 'mono/stereo/omit',
      'fractional frame rates and short tail', 'pending/reordered/duplicate/unknown ACK',
      'document digest binding', 'preflight options/transport/budget refusal', 'fresh Worker factory', 'resource-free actual-export capability fixture'] });
}
