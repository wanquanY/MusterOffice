/** Explicit owned deliveries through the public SDK in a real isolated Worker. */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';

const [output, corpus, cli, bundle, pin] = process.argv.slice(2);
assert(output && corpus && cli && bundle && pin);
await fs.mkdir(output, {recursive: false});
const sha = b => createHash('sha256').update(b).digest('hex');
const json = async file => JSON.parse(await fs.readFile(file));
const write = (file, value) => fs.writeFile(file, JSON.stringify(value, null, 2) + '\n', {flag: 'wx'});
const binary = file => fs.readFile(file);
assert.equal(sha(await binary(path.join(bundle, 'bundle-manifest.json'))), pin);
const {openWorker} = await import(pathToFileURL(path.resolve(bundle, 'examples/node-worker.mjs')));
const worker = await openWorker(bundle, pin);
const cases = [];
function native(args) {
  const result = spawnSync(cli, args, {encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, timeout: 30000});
  assert.equal(result.status, 0, result.stderr); assert.equal(result.stderr, '');
  return JSON.parse(result.stdout);
}
async function compare(name, request, contents, expected = 'prepared') {
  const root = path.join(output, name); await fs.mkdir(root);
  const req = path.join(root, 'request.json'), data = path.join(root, 'contents.bin');
  await write(req, request); await fs.writeFile(data, contents, {flag: 'wx'});
  const response = native(['delivery-playback', req, data]);
  assert.equal(response.status, expected, JSON.stringify(response));
  const payload = new Uint8Array(contents);
  let inputs;
  try {
    inputs = await worker.call({operation: 'prepareDeliveryInputs', request, contents: payload}, [payload.buffer]);
    assert.equal(expected, 'prepared'); assert.deepEqual(inputs, response.inputs);
  } catch (e) {
    if (expected !== 'error') throw e;
    assert.equal(e.name, 'DeliveryPlaybackError'); assert.deepEqual(e.diagnostic, response.error);
  }
  assert.equal(payload.byteLength, 0);
  await write(path.join(root, 'response.json'), response);
  cases.push({name, status: expected, requestSha256: sha(await binary(req)),
    contentsSha256: sha(contents), responseSha256: sha(await binary(path.join(root, 'response.json'))), frames: []});
  return {root, inputs, record: cases.at(-1)};
}
try {
  const names = await json(path.join(corpus, 'cases.json'));
  for (const name of names) {
    const request = await json(path.join(corpus, name, 'request.json'));
    const contents = await binary(path.join(corpus, name, 'contents.bin'));
    const {root, inputs, record} = await compare(name, request, contents);
    const asset = id => {
      const range = request.delivery.contents.find(v => v.assetId === id); assert(range);
      return new Uint8Array(contents.subarray(Number(range.byteOffset), Number(range.byteOffset) + Number(range.byteLength)));
    };
    const source = asset(inputs.source.id), fonts = inputs.fontBundle ? asset(inputs.fontBundle.id) : new Uint8Array();
    assert.equal(sha(source), inputs.source.sha256);
    const sourcePath = path.join(root, 'source.pptx'), fontPath = path.join(root, 'fonts.bin');
    await fs.writeFile(sourcePath, source, {flag: 'wx'}); await fs.writeFile(fontPath, fonts, {flag: 'wx'});
    for (const [index, selected] of inputs.pages.entries()) {
      const page = {profile: 'drawingml-resource-page-q32-v1-draft', ...selected.request, fonts: inputs.fonts};
      const binding = {session: `delivery:${name}:${index}`, revision: inputs.source.sha256, generation: '1'};
      const sourceInput = new Uint8Array(source), fontInput = new Uint8Array(fonts);
      const prepared = await worker.call({operation: 'prepare', kind: 'source', request: {page, binding},
        source: sourceInput, fonts: fontInput}, [sourceInput.buffer, fontInput.buffer]);
      assert.equal(prepared.inputsDetached, true);
      for (const ticks of ['0', '500']) {
        const at = {ticks, timescale: 1000};
        const requestPath = path.join(root, `page-${index}-${ticks}.json`);
        const pixelsPath = path.join(root, `page-${index}-${ticks}.rgba`);
        await write(requestPath, {page, sample: {binding, at, history: null}});
        const reference = native(['render-pptx-playback-page', requestPath, sourcePath, fontPath, pixelsPath]);
        assert.equal(reference.status, 'rendered', JSON.stringify(reference));
        const frame = await worker.call({operation: 'sample', at});
        assert.deepEqual(frame.info.playback, reference.info.playback);
        assert.equal(sha(frame.pixels), sha(await binary(pixelsPath)));
        assert.equal(frame.info.page.textWork.componentCalls, 0);
        assert.equal(frame.info.page.textWork.fontUploadBytes, 0);
        await worker.call({operation: 'beginSample', at});
        let steps = 0;
        while (!(await worker.call({operation: 'stepSample', workUnits: 7})).complete) {
          assert(++steps < 10000, 'bounded fixture did not complete');
        }
        const stepped = await worker.call({operation: 'takeSample'});
        assert.deepEqual(stepped, frame);
        record.frames.push({pageId: selected.pageId, at, pixelsSha256: sha(frame.pixels), bytes: frame.pixels.byteLength});
      }
      await worker.call({operation: 'dispose'});
    }
  }
  const base = await json(path.join(corpus, names[0], 'request.json'));
  const bytes = await binary(path.join(corpus, names[0], 'contents.bin'));
  const faults = [
    ['width-zero', q => {q.width = 0;}],
    ['width-budget', q => {q.width = 8193;}],
    ['wrong-revision', q => {q.delivery.expected.revision = '0'.repeat(64);}],
    ['missing-range', q => {q.delivery.contents.pop();}],
    ['swapped-pages', q => {q.delivery.bundle.previews.reverse();}],
    ['corrupt-bytes', (_q, b) => {b[0] ^= 1;}],
  ];
  for (const [name, edit] of faults) {
    const request = structuredClone(base), data = new Uint8Array(bytes); edit(request, data);
    await compare(name, request, data, 'error');
  }
  await compare('reuse-after-rejection', base, bytes);
} finally {await worker.close();}
await write(path.join(output, 'report.json'), {format: 'musteroffice.delivery-playback-parity/1',
  scope: 'Owned delivery derivation and existing source sampler; no full player or animation-quality acceptance.',
  bundleManifestSha256: pin, cliSha256: sha(await binary(cli)), cases});
console.log(JSON.stringify({cases: cases.length, frames: cases.reduce((n, c) => n + c.frames.length, 0)}));
