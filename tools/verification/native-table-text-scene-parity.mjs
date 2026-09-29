/** Replays already-compiled native table text scenes through both raster hosts.
 * This verifies scene lowering/rasterization, not WASM table text preparation.
 */
import assert from 'node:assert/strict';
import {readFileSync, readdirSync, mkdirSync, writeFileSync} from 'node:fs';
import {resolve, join, dirname} from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';

assert.equal(process.argv.length, 8, 'corpus cli wasm.js raster.js skiaDir output');
const [corpus, cli, wasmPath, rasterPath, skiaDir, output] = process.argv.slice(2).map(p=>resolve(p));
mkdirSync(output, {recursive:false});
const inputs = new Map();
const sha = bytes=>createHash('sha256').update(bytes).digest('hex');
function read(path) {
  const bytes=readFileSync(path);
  inputs.set(path, {path, sha256:sha(bytes), byteLength:bytes.length});
  return bytes;
}
function save(name, bytes) {
  const path=join(output,name);
  writeFileSync(path,bytes,{flag:'wx'});
  return {path,sha256:sha(bytes),byteLength:Buffer.byteLength(bytes)};
}
for(const path of [cli,join(dirname(cli),'mo-raster-worker'),wasmPath,wasmPath.replace(/\.js$/,'_bg.wasm'),rasterPath,join(skiaDir,'mo-skia.mjs')]) read(path);
const wasm=createRequire(import.meta.url)(wasmPath);
const {RasterComponent}=await import(pathToFileURL(rasterPath));
const {default:factory}=await import(pathToFileURL(join(skiaDir,'mo-skia.mjs')));
const raster=await RasterComponent.create(factory,new WebAssembly.Module(read(join(skiaDir,'mo-skia.wasm'))));
const cases=[];
try {
  for(const name of readdirSync(corpus).filter(n=>n.endsWith('.scene.json')).sort()) {
    const stem=name.slice(0,-11);
    const q={viewport:JSON.parse(read(join(corpus,stem+'.request.json'))).viewport,
      scene:JSON.parse(read(join(corpus,name)))};
    const request=save(stem+'.request.json',JSON.stringify(q));
    const pixels=join(output,stem+'.rgba');
    const n=spawnSync(cli,['render-scene',request.path,pixels],{timeout:60000,maxBuffer:4<<20});
    assert.ifError(n.error);assert.equal(n.status,0,n.stderr.toString());
    const w=wasm.render_scene(JSON.stringify(q),raster);
    const metadata=n.stdout.toString().trim();
    assert.equal(metadata,w.metadata);
    assert.equal(JSON.parse(metadata).status,'rendered');
    const native=readFileSync(pixels),reference=read(join(corpus,stem+'.rgba'));
    assert.deepEqual(native,reference,stem+' native library');
    assert.deepEqual(Buffer.from(w.take_pixels()),reference,stem+' WASM scene');
    cases.push({name:stem,request,response:save(stem+'.response.json',metadata),
      pixelSha256:sha(native),pixelBytes:native.length});
  }
  assert.equal(cases.length,4);
  for(const [path,receipt] of inputs) assert.equal(sha(readFileSync(path)),receipt.sha256,path);
  save('report.json',JSON.stringify({scope:'compiled table text scenes only; no WASM table preparation or application acceptance',
    cases,inputs:[...inputs.values()]},null,2)+'\n');
  process.stdout.write(JSON.stringify({status:'passed',scenes:cases.length})+'\n');
} finally { raster.invalidate(); }
