/** Render the Native world compiler's outputs with the shared Native/WASM engine.
 * The new world compiler itself is currently a library/Native verification path.
 */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import factory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/image-world',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/image-world/wasm-node/mo_wasm.js');
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/clips/component/mo-skia.wasm')));
const reference=JSON.parse(fs.readFileSync(root+'/reference.json')),fixtures=JSON.parse(fs.readFileSync(root+'/fixtures.json')),images=read(fixtures.pixels),cases=[];
let calls=0,frame;
const backend={rasterImages(f,b){calls++;frame=f.slice();return raster.rasterImages(f,b);},invalidate(){raster.invalidate();}};
for(const c of reference.cases){
 const q=read(c.request),h=Buffer.alloc(8);h.writeUInt32LE(q.length);h.writeUInt32LE(images.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',['--image-scene'],{input:Buffer.concat([h,q,images]),env:{},timeout:60000,maxBuffer:90*1024*1024});assert.equal(n.status,0,n.stderr.toString());
 const size=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 calls=0;frame=null;const w=wasm.render_image_scene(q.toString(),images,backend);assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert.equal(calls,1);assert(!raster.invalid);
 const response=JSON.parse(metadata);assert.equal(response.status,'rendered',c.name+': '+metadata);
 const total=BigInt(response.info.scene.work.combinedCoordinateErrorBound)+BigInt(c.upstreamGeometryBoundRaw);assert(total<=16777216n,c.name);
 const b=out+'/'+c.name;cases.push({...c,response:put(b+'.response.json',metadata),pixels:put(b+'.rgba',pixels),frame:put(b+'.frame',Buffer.from(frame.buffer)),combinedBoundRaw:total.toString()});
}
const old=JSON.parse(fs.readFileSync('.codex-work/image-layout/reference.json')),regressions=[];
for(const [i,c] of old.cases.entries()){
 read(c.source);read(c.request);const n=spawnSync('target/debug/examples/source_image_layout',[c.request.path,c.source.path,...(i===0?['--audit']:[])],{env:{},timeout:60000,maxBuffer:16*1024*1024});assert.equal(n.status,0,n.stderr.toString());assert.deepEqual(n.stdout,read(c.output),c.name);
 regressions.push({name:c.name,source:c.source,request:c.request,previousOutput:c.output,response:put(out+'/old-'+c.name+'.json',n.stdout)});
}
const result={format:'musteroffice.image-world-parity/1',scope:'New Native source/world compilation followed by shared renderer parity; not execution of the new world compiler through a WASM product API.',pairedCalls:cases.length,reference:entry(root+'/reference.json'),cases,oldNativeLayoutRequests:regressions.length,regressions,artifacts:['target/debug/mo-raster-worker','target/debug/examples/source_image_layout',root+'/wasm-node/mo_wasm_bg.wasm',root+'/wasm-node/mo_wasm.js','.codex-work/clips/component/mo-skia.wasm','.codex-work/clips/ts-raster/index.js'].map(entry)};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({pairedCalls:cases.length,oldNativeLayoutRequests:regressions.length}));
