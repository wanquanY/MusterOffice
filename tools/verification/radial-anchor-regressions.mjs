/** Current public execution of the complete sealed V6 image and V7 clip corpus. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/rect-gradient/ts-raster/index.js';
import factory from '../../.codex-work/rect-gradient/component/mo-skia.mjs';
const root='.codex-work/radial-observation',out=root+'/image-regressions';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/radial-observation/wasm-node/mo_wasm.js');
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/rect-gradient/component/mo-skia.wasm')));
const records=[],previous=[];
for(const [kind,path] of [['image','.codex-work/image-domain/parity.json'],['clip','.codex-work/clips/parity.json'],['composite','.codex-work/compositing/parity.json'],['gradient','.codex-work/gradient-field/parity.json'],['office','.codex-work/office-gradient/parity.json'],['rect','.codex-work/rect-gradient/parity.json']]){
 const report=JSON.parse(fs.readFileSync(path));previous.push(entry(path));
 for(const c of report.cases)for(const scene of [false,true]){
  if(scene&&!c.sceneRequest)continue;
  const image=kind==='image'||c.images,request=scene?c.sceneRequest:c.request,q=read(request),data=image?read(kind==='image'?c.images:report.images):Buffer.alloc(0);
  const h=Buffer.alloc(image?8:4);h.writeUInt32LE(q.length);if(image)h.writeUInt32LE(data.length,4);
  const n=spawnSync('target/debug/mo-raster-worker',image?[scene?'--image-scene':'--images']:scene?['--scene']:[],{input:Buffer.concat([h,q,data]),env:{},timeout:60000,maxBuffer:90*1024*1024});
  assert.equal(n.status,0,n.stderr.toString());const size=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+size).toString(),pixels=n.stdout.subarray(8+size);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
  const expected=scene?c.sceneResponse:(c.pathResponse??c.response);assert.equal(metadata,read(expected).toString(),kind+'/'+c.name);
  const w=image?(scene?wasm.render_image_scene(q.toString(),data,raster):wasm.render_image_paths(q.toString(),data,raster)):(scene?wasm.render_scene(q.toString(),raster):wasm.render_paths(q.toString(),raster));
  assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!raster.invalid);
  if(kind!=='image'||c.success)assert.deepEqual(pixels,read(c.pixels));else assert.equal(pixels.length,0);
  const p=out+'/'+records.length+'.json';fs.writeFileSync(p,metadata);records.push({kind,name:c.name,scene,request,priorResponse:expected,response:entry(p),...(image?{images:kind==='image'?c.images:report.images}:{}),...(pixels.length?{pixels:c.pixels}:{})});
 }
}
const report={format:'musteroffice.radial-anchor-regressions/1',pairedCalls:records.length,previous,cases:records};fs.writeFileSync(root+'/image-regressions.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({pairedCalls:records.length,image:records.filter(c=>c.kind==='image').length,clip:records.filter(c=>c.kind==='clip').length}));
