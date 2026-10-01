// Compare the same explicit decode request through native and WASM public APIs.
// Inputs belong to the caller; this probe never changes or publishes source assets.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
const [worker, wasmDirectory, rasterDirectory, adapterPath, manifest] = process.argv.slice(2);
assert(manifest, 'worker wasm-node-directory raster-directory adapter-js cases-json required');
const wasm = createRequire(import.meta.url)(path.resolve(wasmDirectory, 'mo_wasm.js'));
const {RasterComponent} = await import(pathToFileURL(path.resolve(adapterPath)));
const {default: factory} = await import(pathToFileURL(path.resolve(rasterDirectory, 'mo-skia.mjs')));
const component = await RasterComponent.create(factory,
  new WebAssembly.Module(fs.readFileSync(path.join(rasterDirectory, 'mo-skia.wasm'))));
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const cases = JSON.parse(fs.readFileSync(manifest));
const result = [];
for (const c of cases) {
  if (c.expected === undefined && c.width !== undefined) continue; // unsupported legacy fixtures
  const bytes = fs.readFileSync(c.path);
  assert.equal(digest(bytes), c.sha256);
  const minimumSize = c.minimumSize ?? {width: Math.max(1, Math.floor(c.width / 2)), height: Math.max(1, Math.floor(c.height / 2))};
  for (const minimum of [undefined, minimumSize]) {
    const request = JSON.stringify({sourceSha256:c.sha256,...(minimum ? {minimumSize:minimum} : {})});
    const header = Buffer.alloc(8);
    header.writeUInt32LE(Buffer.byteLength(request));header.writeUInt32LE(bytes.length,4);
    const native = spawnSync(worker,['--decode-image'],{input:Buffer.concat([header,Buffer.from(request),bytes]),timeout:60000,maxBuffer:90*1024*1024});
    assert.equal(native.status,0,native.stderr.toString());
    const count=native.stdout.readUInt32LE(), metadata=native.stdout.subarray(8,8+count).toString(), pixels=native.stdout.subarray(8+count);
    assert.equal(pixels.length,native.stdout.readUInt32LE(4));
    const paired=wasm.decode_image(request,bytes,component);
    assert.equal(paired.metadata,metadata);
    assert.deepEqual(Buffer.from(paired.take_pixels()),pixels);
    assert(!component.invalid);
    const reply=JSON.parse(metadata);
    assert.equal(reply.status,'decoded',c.name);
    assert.equal(reply.info.sourceSha256,c.sha256);
    assert.equal(reply.info.pixelsSha256,digest(pixels));
    result.push({name:c.name??c.sha256,minimumSize:minimum??null,width:reply.info.width,height:reply.info.height,bytes:pixels.length,pixelsSha256:reply.info.pixelsSha256});
  }
}
console.log(JSON.stringify({format:'musteroffice.sampled-image-codec-parity/1',pairedCalls:result.length,cases:result},null,2));
