/** Current runtimes against frozen image-domain and shared-clip pixels/frames. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import factory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/image-paint',out=root+'/regressions';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/image-paint/wasm-node/mo_wasm.js');
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/clips/component/mo-skia.wasm')));
const cases=[];let updated=0;
const previous=['.codex-work/image-domain/parity.json','.codex-work/clips/parity.json'];
for(const filename of previous){
 const old=JSON.parse(fs.readFileSync(filename));
 for(const c of old.cases){
  if(filename.includes('clips/')&&!c.images)continue;
  for(const scene of [false,true]){
   const request=scene?c.sceneRequest:c.request,q=read(request),resource=filename.includes('clips/')?old.images:c.images,images=read(resource),h=Buffer.alloc(8);h.writeUInt32LE(q.length);h.writeUInt32LE(images.length,4);
   const n=spawnSync('target/debug/mo-raster-worker',[scene?'--image-scene':'--images'],{input:Buffer.concat([h,q,images]),env:{},timeout:60000,maxBuffer:90*1024*1024});assert.equal(n.status,0,n.stderr.toString());
   const size=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
   const expected=scene?c.sceneResponse:c.pathResponse,prior=JSON.parse(read(expected)),current=JSON.parse(metadata);assert.equal(current.status,prior.status,c.name);
   const changes={};
   if(current.status==='rendered'){
    for(const [label,a,b,key] of [['image',prior.info.images,current.info.images,'coordinateErrorBound'],...(scene?[['scene',prior.info.scene.work,current.info.scene.work,'combinedCoordinateErrorBound']]:[])]){
     if(a[key]!==b[key]){assert(BigInt(b[key])>=BigInt(a[key]),c.name);assert(BigInt(b[key])<=BigInt(JSON.parse(q).raster.viewport.coordinateTolerance),c.name);changes[label]={previous:a[key],current:b[key]};a[key]=b[key];}
    }
    assert.deepEqual(pixels,read(c.pixels),c.name);
   }else assert.equal(pixels.length,0);
   assert.deepEqual(current,prior,c.name); // every other field, including frame/resource digest, exact
   const w=scene?wasm.render_image_scene(q.toString(),images,raster):wasm.render_image_paths(q.toString(),images,raster);assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!raster.invalid);
   if(Object.keys(changes).length)updated++;
   const path=out+'/'+cases.length+'.json';fs.writeFileSync(path,metadata);cases.push({name:c.name,scene,request,images:resource,priorResponse:expected,response:entry(path),changes,...(current.status==='rendered'?{pixels:c.pixels}:{})});
  }
 }
}
fs.writeFileSync(root+'/precision-regressions.json',JSON.stringify({format:'musteroffice.image-precision-regressions/1',pairedCalls:cases.length,updatedErrorReports:updated,previous:previous.map(entry),cases},null,2)+'\n');console.log(JSON.stringify({pairedCalls:cases.length,updatedErrorReports:updated}));
