/** Public raster/scene parity plus exact background restoration controls. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/compositing/ts-raster/index.js';
import factory from '../../.codex-work/compositing/component/mo-skia.mjs';
import {fixtures,scene} from './compositing-fixtures.mjs';
const root='.codex-work/compositing',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const raster=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
assert(raster.supportsCompositing&&raster.supportsClips&&raster.supportsImages);
const wasm=createRequire(import.meta.url)('../../.codex-work/compositing/wasm-node/mo_wasm.js');
const images=Buffer.from(Array.from({length:20},(_,i)=>i%3===0?[0,0,0,0]:i%3===1?[128,40,10,128]:[20,80,200,255]).flat());
const resources=[{width:5,height:4,alpha:'premultiplied',sha256:sha(images)}];
let paired=0,calls=0,frame;
const backend={raster(f){calls++;frame=f.slice();return raster.raster(f);},rasterImages(f,b){calls++;frame=f.slice();return raster.rasterImages(f,b);},invalidate(){raster.invalidate();}};
function run(q,isScene,image){
 const request=JSON.stringify(image?{raster:q,images:resources}:q),j=Buffer.from(request),b=image?images:Buffer.alloc(0),h=Buffer.alloc(image?8:4);h.writeUInt32LE(j.length);if(image)h.writeUInt32LE(b.length,4);
 const n=spawnSync('target/debug/mo-raster-worker',image?[isScene?'--image-scene':'--images']:isScene?['--scene']:[],{input:Buffer.concat([h,j,b]),env:{},timeout:60000,maxBuffer:90*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());const ml=n.stdout.readUInt32LE(),metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 calls=0;frame=null;const w=image?(isScene?wasm.render_image_scene(request,b,backend):wasm.render_image_paths(request,b,backend)):(isScene?wasm.render_scene(request,backend):wasm.render_paths(request,backend));
 assert.equal(w.metadata,metadata);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);assert(!raster.invalid);paired++;
 return {request,metadata,pixels,response:JSON.parse(metadata),calls,frame};
}
const records=[],controls=[];let rectanglePixels=0,edgePixels=0;
for(const c of fixtures()){
 const p=run(c.request,false,c.images),s=run(scene(c.request),true,c.images);
 assert.equal(p.response.status,'rendered',c.name+': '+p.metadata);assert.equal(s.response.status,'rendered');assert.equal(p.calls,1);assert.equal(s.calls,1);assert.equal(p.frame[1],8);assert.equal(s.frame[1],8);assert.deepEqual(p.pixels,s.pixels,c.name);
 if(c.constant){
  const control=structuredClone(c.request);control.draws[2].brush=c.constant;
  const r=run(control,false,false);assert.equal(r.response.status,'rendered');assert.deepEqual(p.pixels,r.pixels,c.name+' source AA/clip control');edgePixels+=32*24;
  controls.push({name:c.name,kind:'source-solid-full-frame',request:put(out+'/'+c.name+'.control.json',r.request),response:put(out+'/'+c.name+'.control-response.json',r.metadata),pixels:put(out+'/'+c.name+'.control.rgba',r.pixels)});
 }
 if(c.exactRectangle){
  const bg=run({...c.request,draws:c.request.draws.slice(0,1)},false,c.images);assert.equal(bg.response.status,'rendered');
  for(let y=0;y<24;y++)for(let x=0;x<32;x++){
   const i=(y*32+x)*4,[l,t,r,b]=c.window;
   assert.deepEqual(p.pixels.subarray(i,i+4),x>=l&&x<r&&y>=t&&y<b?bg.pixels.subarray(i,i+4):Buffer.from([0,170,0,255]),`${c.name} ${x},${y}`);rectanglePixels++;
  }
  controls.push({name:c.name,kind:'device-background-rectangle',request:put(out+'/'+c.name+'.background.json',bg.request),response:put(out+'/'+c.name+'.background-response.json',bg.metadata),pixels:put(out+'/'+c.name+'.background.rgba',bg.pixels)});
 }
 if(c.name==='three-prefixes-immutable'){
  for(const [x,y,color] of [[2,2,[128,0,0,128]],[30,13,[0,170,0,255]],[1,13,[0,0,0,0]],[30,2,[255,136,0,255]]])assert.deepEqual([...p.pixels.subarray((y*32+x)*4,(y*32+x+1)*4)],color);
 }
 if(c.name==='snapshot-source-over')assert.deepEqual([...p.pixels.subarray((8*32+8)*4,(8*32+9)*4)],[128,85,0,255]);
 const path=out+'/'+c.name;
 records.push({name:c.name,images:c.images,request:put(path+'.path.json',p.request),sceneRequest:put(path+'.scene.json',s.request),response:put(path+'.response.json',p.metadata),sceneResponse:put(path+'.scene-response.json',s.metadata),frame:put(path+'.frame',Buffer.from(p.frame.buffer)),sceneFrame:put(path+'.scene-frame',Buffer.from(s.frame.buffer)),pixels:put(path+'.rgba',p.pixels)});
}
const negatives=[];
for(const [name,mutate] of [
 ['future',q=>q.draws[0].brush={kind:'snapshot',afterDraws:1}],
 ['copy-budget',q=>{q.viewport.width=8192;q.viewport.height=2048;q.draws[2].brush.afterDraws=0;q.draws.push({...q.draws[2],brush:{kind:'snapshot',afterDraws:1}});}],
 ['snapshot-count',q=>{q.draws=Array.from({length:65},(_,i)=>({...q.draws[0],brush:{kind:'snapshot',afterDraws:i}}));}],
 ['unknown-blend',q=>q.draws[2].blend='clear'],
 ['snapshot-unknown-field',q=>q.draws[2].brush.resource=0],
]){
 const q=structuredClone(fixtures()[0].request);mutate(q);const r=run(q,false,false);assert.equal(r.response.status,'error',name);assert.equal(r.calls,0);assert.equal(r.pixels.length,0);
 negatives.push({name,request:put(out+'/'+name+'.invalid.json',r.request),response:put(out+'/'+name+'.invalid-response.json',r.metadata)});
}
const cliChecks=[],cliDir=fs.mkdtempSync(root+'/primitive-cli-');
for(const [command,key,responseKey] of [['render-paths','request','response'],['render-scene','sceneRequest','sceneResponse']]){
 const c=records[0],output=cliDir+'/'+command+'.rgba';
 const run=()=>spawnSync('target/debug/mo-cli',[command,c[key].path,output],{env:{},timeout:60000,encoding:'utf8',maxBuffer:90*1024*1024});
 const r=run();assert.equal(r.status,0,r.stderr);assert.equal(r.stdout.trimEnd(),fs.readFileSync(c[responseKey].path,'utf8'));assert.deepEqual(fs.readFileSync(output),fs.readFileSync(c.pixels.path));
 const before=entry(output);assert.notEqual(run().status,0);assert.deepEqual(entry(output),before);cliChecks.push({command,pixels:before,exclusivePublication:true});
}
const rejected=cliDir+'/rejected.rgba',bad=spawnSync('target/debug/mo-cli',['render-paths',negatives[0].request.path,rejected],{env:{},timeout:60000,encoding:'utf8'});
assert.equal(bad.status,0,bad.stderr);assert.equal(JSON.parse(bad.stdout).status,'error');assert(!fs.existsSync(rejected));
const result={format:'musteroffice.compositing-parity/1',pairedCalls:paired,rectanglePixels,edgePixels,cases:records,controls,negatives,cliChecks,cliRejectionOutputAbsent:true,images:put(out+'/images.rgba',images),worker:entry('target/debug/mo-raster-worker'),rustWasm:entry(root+'/wasm-node/mo_wasm_bg.wasm'),component:entry(root+'/component/mo-skia.wasm'),adapter:entry(root+'/ts-raster/index.js')};
fs.writeFileSync(root+'/parity.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({paired,positive:records.length,rectanglePixels,edgePixels,negative:negatives.length}));
