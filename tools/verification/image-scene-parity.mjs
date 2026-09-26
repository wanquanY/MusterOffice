import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
const componentDirectory=process.argv[4]??'.codex-work/image-brush/component';
const adapterPath=process.argv[5]??'.codex-work/image-brush/ts-raster/index.js';
const {RasterComponent}=await import(pathToFileURL(path.resolve(adapterPath)));
const {default:factory}=await import(pathToFileURL(path.resolve(componentDirectory,'mo-skia.mjs')));
const root=process.argv[2]??'.codex-work/image-scene',out=root+'/'+(process.argv[6]?'scene-runtime':'runtime');
const fixtureRoot=process.argv[3]??root+'/cases';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(componentDirectory+'/mo-skia.wasm')));
const wasm=createRequire(import.meta.url)(path.resolve(root,'wasm-node/mo_wasm.js'));
let calls=0;
const backend={rasterImages(frame,images){calls++;return raster.rasterImages(frame,images);},invalidate(){raster.invalidate();}};
function run(request,images,scene=true){
 const head=Buffer.alloc(8);head.writeUInt32LE(Buffer.byteLength(request));head.writeUInt32LE(images.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',[scene?'--image-scene':'--images'],{input:Buffer.concat([head,Buffer.from(request),images]),env:{},timeout:60000,maxBuffer:90*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());const length=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+length).toString(),pixels=n.stdout.subarray(8+length);
 assert.equal(pixels.length,n.stdout.readUInt32LE(4));calls=0;
 const w=scene?wasm.render_image_scene(request,images,backend):wasm.render_image_paths(request,images,backend);
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);
 const response=JSON.parse(metadata);assert.equal(calls,response.status==='rendered'?1:0);assert(!raster.invalid);
 return {metadata,pixels,response};
}
const manifest=JSON.parse(fs.readFileSync(fixtureRoot+'/manifest.json')),cases=[],references=new Map();
for(const c of manifest.cases){
 const request=read(c.request).toString(),images=read(c.images),r=run(request,images);
 const file=out+'/'+c.name;fs.writeFileSync(file+'.json',r.metadata);
 const record={...c,response:entry(file+'.json')};
 if(c.success){
  assert.equal(r.response.status,'rendered',c.name);
  if(c.expectedPixels)assert.deepEqual(r.pixels,read(c.expectedPixels),c.name);
  else{
   if(!references.has(c.originalRequest.path)){
    const baseline=run(read(c.originalRequest).toString(),images,false);assert.equal(baseline.response.status,'rendered');
    const name=out+'/world-'+references.size;fs.writeFileSync(name+'.json',baseline.metadata);fs.writeFileSync(name+'.rgba',baseline.pixels);
    references.set(c.originalRequest.path,{request:c.originalRequest,images:c.images,response:entry(name+'.json'),pixels:entry(name+'.rgba')});
   }
   record.reference=references.get(c.originalRequest.path);assert.deepEqual(r.pixels,read(record.reference.pixels));
  }
  const q=JSON.parse(request),info=r.response.info;
  assert.equal(info.images.resources,q.images.length);assert.equal(info.images.draws,q.raster.scene.instances.filter(i=>i.brush.kind==='image').length);
  assert.equal(info.images.brushes,1);assert.equal(info.scene.work.compiledPaths,1);
  assert.equal(info.resourcesSha256,entry(c.images.path).sha256);assert.equal(info.scene.raster.sha256,createHash('sha256').update(r.pixels).digest('hex'));
  fs.writeFileSync(file+'.rgba',r.pixels);record.pixels=entry(file+'.rgba');
 }else{assert.equal(r.response.status,'error',c.name);assert.equal(r.pixels.length,0);}
 cases.push(record);
}
const base=manifest.cases.find(c=>c.success),good=JSON.parse(read(base.request)),images=read(base.images),invalid=[];
const mutations=[q=>q.extra=true,q=>q.raster.scene.extra=true,q=>q.raster.scene.instances[0].brush.image.extra=true,q=>q.raster.scene.transforms[0].affine.linear[0]=0,q=>q.raster.scene.instances[0].transform=-1,q=>delete q.images[0].sha256];
for(let i=0;i<7;i++){
 const q=structuredClone(good);if(i<6)mutations[i](q);const request=i<6?JSON.stringify(q):'{"images":[],"images":[]}';
 const r=run(request,images);assert.equal(r.response.error.code,'INPUT_INVALID');
 const prefix=out+'/json-'+i;fs.writeFileSync(prefix+'.request.json',request);fs.writeFileSync(prefix+'.response.json',r.metadata);
 invalid.push({request:entry(prefix+'.request.json'),images:base.images,response:entry(prefix+'.response.json'),duplicateMember:i===6});
}
let invalidated=0;
const broken=wasm.render_image_scene(JSON.stringify(good),images,{rasterImages(){return {status:0,pixels:new Uint8Array([255,0,0,0])};},invalidate(){invalidated++;}});
assert.equal(JSON.parse(broken.metadata).error.code,'COMPONENT_INVALID');assert.equal(broken.take_pixels().length,0);assert.equal(invalidated,1);
const counts={sceneRequests:cases.length,success:cases.filter(c=>c.success).length,preflightFailures:cases.filter(c=>!c.success).length,jsonRejections:invalid.length,pairedCalls:cases.length+invalid.length+references.size,worldReferenceCalls:references.size,wasmHostRejections:1};
fs.writeFileSync(root+'/'+(process.argv[6]??'parity')+'.json',JSON.stringify({format:'musteroffice.image-scene-parity/1',counts,cases,invalid,references:[...references.values()]},null,2)+'\n');console.log(JSON.stringify(counts));
