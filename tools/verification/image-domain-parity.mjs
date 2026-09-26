import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {RasterComponent} from '../../.codex-work/image-domain/ts-raster/index.js';
import factory from '../../.codex-work/image-domain/component/mo-skia.mjs';
const root='.codex-work/image-domain',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const module=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const raster=await RasterComponent.create(factory,module);assert(raster.supportsImageDomains);
const wasm=createRequire(import.meta.url)('../../.codex-work/image-domain/wasm-node/mo_wasm.js');
let calls=0,frame;
const backend={rasterImages(f,b){calls++;frame=f.slice();return raster.rasterImages(f,b);},invalidate(){raster.invalidate();}};
function run(request,bytes,scene){
 const h=Buffer.alloc(8);h.writeUInt32LE(Buffer.byteLength(request));h.writeUInt32LE(bytes.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',[scene?'--image-scene':'--images'],{input:Buffer.concat([h,Buffer.from(request),bytes]),env:{},timeout:60000,maxBuffer:90*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());const size=n.stdout.readUInt32LE();
 const metadata=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 calls=0;frame=null;
 const w=scene?wasm.render_image_scene(request,bytes,backend):wasm.render_image_paths(request,bytes,backend);
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!raster.invalid);
 return {metadata,pixels,response:JSON.parse(metadata),calls,frame};
}
const records=[];
let maxDeviation=0;
for(const c of JSON.parse(fs.readFileSync(root+'/cases/manifest.json')).cases){
 const bytes=read(c.images),path=run(read(c.request).toString(),bytes,false),scene=run(read(c.sceneRequest).toString(),bytes,true);
 assert.equal(path.response.status,c.success?'rendered':'error',c.name);assert.equal(scene.response.status,path.response.status,c.name);
 assert.deepEqual(scene.pixels,path.pixels,c.name);
 const record={...c};
 for(const [name,r] of [['path',path],['scene',scene]]){
  assert.equal(r.calls,c.success?1:0,c.name);if(!c.success)assert.equal(r.pixels.length,0);
  const p=out+'/'+c.name+'.'+name;fs.writeFileSync(p+'.json',r.metadata);record[name+'Response']=entry(p+'.json');
 }
 if(c.success){
  const expected=read(c.expectedPixels);assert.equal(path.pixels.length,expected.length);let delta=0;
  for(let i=0;i<expected.length;i++)delta=Math.max(delta,Math.abs(expected[i]-path.pixels[i]));
  assert(delta<=1,`${c.name} pixel deviation ${delta}`);maxDeviation=Math.max(maxDeviation,delta);
  assert.equal(path.frame[1],6);assert.equal(path.response.info.images.resources,1);
  const prefix=out+'/'+c.name;fs.writeFileSync(prefix+'.rgba',path.pixels);fs.writeFileSync(prefix+'.frame',Buffer.from(path.frame.buffer));
  record.pixels=entry(prefix+'.rgba');record.frame=entry(prefix+'.frame');record.maxChannelDeviation=delta;
 }
 records.push(record);
}
const result={format:'musteroffice.image-domain-parity/1',pairedCalls:records.length*2,success:records.filter(c=>c.success).length,
 preflightFailures:records.filter(c=>!c.success).length,maxChannelDeviation:maxDeviation,component:entry(root+'/component/mo-skia.wasm'),
 imports:WebAssembly.Module.imports(module),cases:records};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({pairedCalls:result.pairedCalls,success:result.success,preflightFailures:result.preflightFailures,maxChannelDeviation:maxDeviation}));
