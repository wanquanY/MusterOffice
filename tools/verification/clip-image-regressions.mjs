/** Current Rust Native/WASM re-execution of the sealed V6 path/scene corpus. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import factory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/clips',out=root+'/image-regressions';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const previous=JSON.parse(fs.readFileSync('.codex-work/image-domain/parity.json'));
const wasm=createRequire(import.meta.url)('../../.codex-work/clips/wasm-node/mo_wasm.js');
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
const cases=[];
for(const c of previous.cases)for(const scene of [false,true]){
 const request=scene?c.sceneRequest:c.request,q=read(request),images=read(c.images),h=Buffer.alloc(8);h.writeUInt32LE(q.length);h.writeUInt32LE(images.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',[scene?'--image-scene':'--images'],{input:Buffer.concat([h,q,images]),env:{},timeout:60000,maxBuffer:90*1024*1024});assert.equal(n.status,0,n.stderr.toString());
 const size=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 const expected=scene?c.sceneResponse:c.pathResponse;assert.equal(metadata,read(expected).toString(),c.name);
 const w=scene?wasm.render_image_scene(q.toString(),images,raster):wasm.render_image_paths(q.toString(),images,raster);assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!raster.invalid);
 if(c.success)assert.deepEqual(pixels,read(c.pixels));else assert.equal(pixels.length,0);
 const path=out+'/'+cases.length+'.json';fs.writeFileSync(path,metadata);cases.push({name:c.name,scene,request,images:c.images,priorResponse:expected,response:entry(path),...(c.success?{pixels:c.pixels}:{})});
}
const result={format:'musteroffice.shared-clips-image-regressions/1',pairedCalls:cases.length,previous:entry('.codex-work/image-domain/parity.json'),cases};
fs.writeFileSync(root+'/image-regressions.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({pairedCalls:cases.length}));
