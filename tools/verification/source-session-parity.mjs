/** Persistent Native and WASM source owners against frozen one-shot outputs. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/source-session',out=root+'/frames';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/source-session/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,typeof b==='string'||Buffer.isBuffer(b)?b:JSON.stringify(b));return entry(p);};
let rasters=0,decodes=0,shapes=0,frame=null;
const raster={raster(f){rasters++;frame=f.slice();return component.raster(f);},rasterImages(f,b){rasters++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
const text={shapeBatch(...a){shapes++;return shaper.shapeBatch(...a);},measureBatch(...a){shapes++;return shaper.measureBatch(...a);},outlineBatch(...a){shapes++;return shaper.outlineBatch(...a);},invalidate(){shaper.invalidate();}};
function native(){
 const child=spawn('target/debug/mo-raster-worker',['--pptx-playback-session'],{env:{},stdio:['pipe','pipe','pipe']});
 let buffered=Buffer.alloc(0),pending=null,stderr='';
 const drain=()=>{if(!pending||buffered.length<8)return;const ml=buffered.readUInt32LE(),pl=buffered.readUInt32LE(4);assert(ml<=(64<<20)&&pl<=(256<<20));if(buffered.length<8+ml+pl)return;const r={metadata:buffered.subarray(8,8+ml).toString(),pixels:buffered.subarray(8+ml,8+ml+pl)};buffered=buffered.subarray(8+ml+pl);const p=pending;pending=null;clearTimeout(p.timer);p.resolve(r);};
 child.stdout.on('data',b=>{buffered=Buffer.concat([buffered,b]);drain();});child.stderr.on('data',b=>stderr+=b);
 const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('close',(code,signal)=>{if(pending){clearTimeout(pending.timer);pending.reject(new Error(stderr));pending=null;}resolve({code,signal});});});
 return {send:(json,source,fonts)=>new Promise((resolve,reject)=>{assert.equal(pending,null);pending={resolve,reject,timer:setTimeout(()=>{child.kill();reject(new Error('source owner timeout'));},30000)};const h=Buffer.alloc(12);h.writeUInt32LE(Buffer.byteLength(json));h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);child.stdin.write(Buffer.concat([h,Buffer.from(json),source,fonts]));drain();}),close:async()=>{child.stdin.end();const r=await exit;assert.equal(r.code,0,stderr);assert.equal(stderr,'');assert.equal(buffered.length,0);}};
}
let owner=null,owners=0;const records=[],controls=[];
async function open(){assert.equal(owner,null);owner={w:new wasm.PptxPlaybackSession(),n:native(),id:owners++};}
async function close(){owner.w.free();await owner.n.close();owner=null;}
async function call(name,q,source,fonts){
 const json=typeof q==='string'?q:JSON.stringify(q);const operation=typeof q==='object'?q.operation:null;
 const sb=source?load(source):Buffer.alloc(0),fb=fonts?load(fonts):Buffer.alloc(0);const original=[sha(sb),sha(fb)];
 const n=await owner.n.send(json,sb,fb);rasters=decodes=shapes=0;frame=null;let metadata,pixels=Buffer.alloc(0);
 if(operation==='prepare'||source||fonts)metadata=owner.w.prepare(json,sb,fb,decoder,text);
 else if(operation==='render'){const r=owner.w.render(json,raster);metadata=r.metadata;pixels=Buffer.from(r.take_pixels());}
 else metadata=owner.w.command(json);
 assert.equal(metadata,n.metadata,name);assert.deepEqual(pixels,n.pixels,name);assert.deepEqual([sha(sb),sha(fb)],original);sb.fill(0);fb.fill(0);
 assert(!component.invalid);assert(!shaper.invalid);const response=JSON.parse(metadata),prefix=out+'/'+records.length;
 const record={name,owner:owner.id,request:put(prefix+'.request.json',json),...(source?{source}:{}),...(fonts?{fonts}:{}),response:put(prefix+'.response.json',metadata),pixels:put(prefix+'.rgba',pixels),rasters,decodes,shapes,...(frame?{frame:put(prefix+'.frame',Buffer.from(frame.buffer))}:{})};records.push(record);
 if(response.status==='rendered'){assert.equal(rasters,1);assert.equal(sha(pixels),response.info.page.page.scene.raster.sha256);assert.equal(decodes+shapes,0);assert.equal(response.info.page.gatherCopyBytes,0);assert.equal(response.info.page.textWork.componentCalls,0);assert.equal(response.info.page.textWork.fontUploadBytes,0);assert.equal(response.info.page.textWork.requestWords,0);}
 else{assert.equal(rasters,0);assert.equal(pixels.length,0);}
 if(response.status==='prepared'){assert.equal(decodes,response.info.preparation.decodedImages);assert.equal(shapes,response.info.preparation.textWork.componentCalls);}
 if(operation!=='prepare')assert.equal(decodes+shapes,0);
 return {response,pixels,frame,record};
}
function compare(r,reference,name){
 const expected=structuredClone(reference);expected.info.page.textWork.componentCalls=0;expected.info.page.textWork.fontUploadBytes=0;expected.info.page.textWork.requestWords=0;expected.info.page.gatherCopyBytes=0;
 assert.deepEqual(r.response,expected,name+' metadata except actual preparation work');
}
const prior=JSON.parse(fs.readFileSync('.codex-work/source-playback/product.json'));
const mixed=JSON.parse(fs.readFileSync(root+'/fixtures.json'));
const groups=['image-text','group-image','masters','circle','text','click','mixed'];let replayFrames=0,mixedFrames=0;
for(const name of groups){
 const cases=name==='mixed'?mixed.cases:prior.cases.filter(c=>name==='click'?['click-after','click-before'].includes(c.name):c.name.startsWith(name+'-'));
 await open();const initial=JSON.parse(load(cases[0].request));const binding=initial.sample.binding;
 assert.equal((await call(name+' empty',{operation:'render',sample:initial.sample})).response.error.code,'NOT_PREPARED');
 const prepare={operation:'prepare',request:{page:initial.page,binding}};
 const first=await call(name+' prepare',prepare,cases[0].source,cases[0].fonts);assert.equal(first.response.status,'prepared',JSON.stringify(first.response));
 const again=await call(name+' duplicate',prepare,cases[0].source,cases[0].fonts);assert.equal(again.response.error.code,'ALREADY_PREPARED');assert.equal(again.record.decodes+again.record.shapes,0);
 assert.deepEqual((await call(name+' inspect',{operation:'inspect',binding})).response.info,first.response.info);
 for(const c of cases){
  const q=JSON.parse(load(c.request)),r=await call(c.name,{operation:'render',sample:q.sample});assert.equal(r.response.status,'rendered',JSON.stringify(r.response));
  if(name!=='mixed'){compare(r,JSON.parse(load(c.response)),c.name);assert.deepEqual(r.pixels,load(c.pixels));assert.deepEqual(Buffer.from(r.frame.buffer),load(c.frame));r.record.frozen=c.response;replayFrames++;}
  else{
   assert.deepEqual(r.response.info.playback.evaluated.state.rotations,c.expectedRotations);
   const result=wasm.render_pptx_playback_page(JSON.stringify(q),load(c.source),load(c.fonts),decoder,text,raster),metadata=result.metadata,pixels=Buffer.from(result.take_pixels());
   compare(r,JSON.parse(metadata),c.name);assert.deepEqual(pixels,r.pixels);assert.deepEqual(frame,r.frame);mixedFrames++;
   const control={name:c.name,oneShotResponse:put(out+'/'+c.name+'.oneshot.json',metadata),pixelSha256:sha(pixels)};
   if(c.staticControl){const s=wasm.render_pptx_resource_page(load(c.staticControl.request).toString(),load(c.staticControl.source),load(c.fonts),decoder,text,raster);assert.equal(JSON.parse(s.metadata).status,'rendered');const p=Buffer.from(s.take_pixels());assert.deepEqual(p,r.pixels);assert.deepEqual(frame,r.frame);control.staticControl=c.staticControl;control.staticPixelSha256=sha(p);}
   controls.push(control);
  }
 }
 const advanced=await call(name+' advance',{operation:'advance',binding,generation:'12'});assert.equal(advanced.response.status,'advanced');assert.equal(advanced.response.info.planId,first.response.info.planId);
 assert.equal((await call(name+' stale',{operation:'render',sample:initial.sample})).response.error.code,'BINDING_CONFLICT');
 const next=structuredClone(initial.sample);next.binding=advanced.response.info.binding;
 if(next.history){assert.equal((await call(name+' old history',{operation:'render',sample:next})).response.error.error.error.code,'EVENT_HISTORY_INVALID');next.history.binding=next.binding;for(const e of next.history.events)e.generation='12';}
 assert.equal((await call(name+' after advance',{operation:'render',sample:next})).response.status,'rendered');
 assert.equal((await call(name+' regression',{operation:'advance',binding:next.binding,generation:'11'})).response.error.code,'GENERATION_NOT_INCREASING');
 if(name==='mixed'){
  let invalid=false;const bad={raster(){throw Error('owned failure');},rasterImages(){throw Error('owned failure');},invalidate(){invalid=true;}};
  const r=owner.w.render(JSON.stringify({operation:'render',sample:next}),bad);assert.equal(JSON.parse(r.metadata).status,'error');assert.equal(r.take_pixels().length,0);assert(invalid);
  assert.equal((await call('mixed recover',{operation:'render',sample:next})).response.status,'rendered');
 }
 const dispose={operation:'dispose',binding:next.binding};assert.equal((await call(name+' dispose',dispose)).response.status,'disposed');assert.equal((await call(name+' dispose again',dispose)).response.status,'disposed');
 assert.equal((await call(name+' retired',prepare,cases[0].source,cases[0].fonts)).response.error.code,'DISPOSED');await close();
}
// Invalid native semantics, source identity, precision and malformed wire never publish an owner.
for(const name of ['unknown','missing-target','source-conflict','revision-conflict','viewport-invalid','precision']){
 const c=prior.cases.find(c=>c.name===name),q=JSON.parse(load(c.request));await open();const r=await call(name,{operation:'prepare',request:{page:q.page,binding:q.sample.binding}},c.source,c.fonts);assert.equal(r.response.status,'error');assert.equal((await call(name+' remains empty',{operation:'inspect',binding:q.sample.binding})).response.error.code,'NOT_PREPARED');await close();
}
const framing=[];
for(const [name,input]of [['partial-header',Buffer.from([1,2])],['oversize-request',Buffer.from([1,0,0,3,0,0,0,0,0,0,0,0])],['truncated-body',Buffer.from([2,0,0,0,0,0,0,0,0,0,0,0,123])]]){
 const r=spawnSync('target/debug/mo-raster-worker',['--pptx-playback-session'],{input,env:{},timeout:30000});assert.notEqual(r.status,0);assert.equal(r.stdout.length,0);framing.push({name,exitCode:r.status,stderr:r.stderr.toString()});
}
const report={format:'musteroffice.source-session-parity/1',owners,pairedCalls:records.length,cases:records,replayFrames,mixedFrames,controls,framing,wasmResourceFailureRecovery:true,inputsImmutable:true,artifacts:[entry('target/debug/mo-raster-worker'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({owners,pairedCalls:records.length,replayFrames,mixedFrames}));
