/** Retained Native process and WASM owner execute the same command stream. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
const root='.codex-work/source-session/session',out=root+'/frames';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/source-session/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,typeof b==='string'||Buffer.isBuffer(b)?b:JSON.stringify(b));return entry(p);};
const module=new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm'));
const component=await RasterComponent.create(skiaFactory,module);
let calls=0;const backend={raster(f){calls++;return component.raster(f);},invalidate(){component.invalidate();}};
function native(){
 const child=spawn('target/debug/mo-raster-worker',['--playback-session'],{env:{},stdio:['pipe','pipe','pipe']});
 let buffered=Buffer.alloc(0),pending=null,stderr='';
 const drain=()=>{if(!pending||buffered.length<8)return;const ml=buffered.readUInt32LE(),pl=buffered.readUInt32LE(4);assert(ml<=64*1024*1024&&pl<=256*1024*1024);if(buffered.length<8+ml+pl)return;const reply={metadata:buffered.subarray(8,8+ml).toString(),pixels:buffered.subarray(8+ml,8+ml+pl)};buffered=buffered.subarray(8+ml+pl);const p=pending;pending=null;clearTimeout(p.timer);p.resolve(reply);};
 child.stdout.on('data',b=>{buffered=Buffer.concat([buffered,b]);drain();});child.stderr.on('data',b=>stderr+=b);
 const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('close',(code,signal)=>{if(pending){clearTimeout(pending.timer);pending.reject(new Error('worker exited with pending reply: '+stderr));pending=null;}resolve({code,signal});});});
 return {send:json=>new Promise((resolve,reject)=>{assert.equal(pending,null);pending={resolve,reject,timer:setTimeout(()=>{child.kill();reject(new Error('native session timeout'));},30000)};const h=Buffer.alloc(4);h.writeUInt32LE(Buffer.byteLength(json));child.stdin.write(Buffer.concat([h,Buffer.from(json)]));drain();}),close:async()=>{child.stdin.end();const result=await exit;assert.equal(result.code,0,stderr);assert.equal(stderr,'');assert.equal(buffered.length,0);}};
}
let current=null,ownerCount=0;const records=[];
async function open(){assert.equal(current,null);current={w:new wasm.PlaybackSession(),n:native(),index:ownerCount++};}
async function call(name,q){
 const json=typeof q==='string'?q:JSON.stringify(q),obj=typeof q==='string'?JSON.parse(q):q;
 const n=await current.n.send(json);calls=0;let metadata,pixels;
 if(obj.operation==='render'){const r=current.w.render(json,backend);metadata=r.metadata;pixels=Buffer.from(r.take_pixels());}else{metadata=current.w.command(json);pixels=Buffer.alloc(0);}
 assert.equal(metadata,n.metadata,name);assert.deepEqual(pixels,n.pixels,name);assert(!component.invalid);
 const response=JSON.parse(metadata);if(response.status==='rendered'){assert.equal(calls,1);assert.equal(sha(pixels),response.info.page.scene.raster.sha256);}else{assert.equal(calls,0);assert.equal(pixels.length,0);}
 const p=out+'/'+records.length;records.push({name,owner:current.index,request:put(p+'.request.json',json),response:put(p+'.response.json',metadata),pixels:put(p+'.rgba',pixels),calls});
 return {response,metadata,pixels};
}
async function close(){current.w.free();await current.n.close();current=null;}
const previousPath='.codex-work/playback-render/product.json',prior=JSON.parse(fs.readFileSync(previousPath));let key=null,binding=null;let replayed=0,rendered=0;
for(const c of prior.cases){
 const old=JSON.parse(load(c.request)),p=old.playback;
 const request={snapshot:p.snapshot,slide:p.slide,binding:p.binding,viewport:old.viewport,defaults:old.defaults};const k=JSON.stringify(request);
 if(k!==key){if(current){await call('dispose-before-new-plan',{operation:'dispose',binding});await close();}await open();const prepared=await call(c.name+'-prepare',{operation:'prepare',request});key=k;binding=p.binding;
  if(c.name==='stale-snapshot'){assert.equal(prepared.response.status,'error');assert.deepEqual(prepared.response.error.error,JSON.parse(load(c.response)).error);await close();key=null;continue;}
  assert.equal(prepared.response.status,'prepared');
 }
 const sample={binding:p.binding,at:p.at,history:p.history};
 for(const operation of ['compile','render']){
  const r=await call(c.name+'-'+operation,{operation,sample}),expected=JSON.parse(load(operation==='compile'?c.compiled:c.response));
  if(expected.status==='error')assert.deepEqual(r.response,{status:'error',error:{kind:'computation',error:expected.error}});else assert.deepEqual(r.response,expected);
  if(operation==='render'){assert.deepEqual(r.pixels,load(c.pixels));rendered+=r.response.status==='rendered'?1:0;}replayed++;
 }
}
await call('last-dispose',{operation:'dispose',binding});await close();
// A single live owner goes through faults, generation fencing, seek and terminal dispose.
const original=JSON.parse(load(prior.cases.find(c=>c.name==='interactive-3').request)),p=original.playback;
const request={snapshot:p.snapshot,slide:p.slide,binding:p.binding,viewport:original.viewport,defaults:original.defaults};
const sample={binding:p.binding,at:p.at,history:p.history};await open();
const fault=(r,code)=>{assert.equal(r.response.status,'error');assert.equal(r.response.error.kind,'session');assert.equal(r.response.error.code,code);};
fault(await call('empty',{operation:'compile',sample}),'NOT_PREPARED');
const corrupt=structuredClone(request);corrupt.snapshot.semanticDigest='0'.repeat(64);assert.equal((await call('corrupt-prepare',{operation:'prepare',request:corrupt})).response.status,'error');
const prepared=await call('prepare-after-failure',{operation:'prepare',request});assert.equal(prepared.response.status,'prepared');
fault(await call('second-prepare',{operation:'prepare',request}),'ALREADY_PREPARED');
await call('inspect-current',{operation:'inspect',binding:p.binding});
const base=await call('before-advance',{operation:'render',sample});
for(const generation of ['0','3'])fault(await call('non-increasing-'+generation,{operation:'advance',binding:p.binding,generation}),'GENERATION_NOT_INCREASING');
const advanced=await call('advance',{operation:'advance',binding:p.binding,generation:'9007199254740993'});assert.equal(advanced.response.info.planId,prepared.response.info.planId);const next=advanced.response.info.binding;
for(const operation of ['compile','render','inspect','advance','dispose']){
 const q=['compile','render'].includes(operation)?{operation,sample}:{operation,binding:p.binding,...(operation==='advance'?{generation:'9007199254740994'}:{})};
 fault(await call('stale-'+operation,q),'BINDING_CONFLICT');
}
const nextSample=structuredClone(sample);nextSample.binding=next;
const missing=await call('old-history',{operation:'render',sample:nextSample});assert.equal(missing.response.error.error.error.code,'EVENT_HISTORY_INVALID');
nextSample.history.binding=next;for(const event of nextSample.history.events)event.generation=next.generation;
const recovered=await call('rebound-history',{operation:'render',sample:nextSample});assert.deepEqual(recovered.pixels,base.pixels);assert.notEqual(recovered.response.info.frame.sha256,base.response.info.frame.sha256);
const back=structuredClone(nextSample);back.at={ticks:'1',timescale:3};await call('explicit-backward-sample',{operation:'render',sample:back});
const invalid=await call('unknown-field',{operation:'inspect',binding:next,extra:1});fault(invalid,'INPUT_INVALID');
await call('dispose',{operation:'dispose',binding:next});await call('dispose-idempotent',{operation:'dispose',binding:next});
fault(await call('disposed-render',{operation:'render',sample:nextSample}),'DISPOSED');fault(await call('disposed-prepare',{operation:'prepare',request}),'DISPOSED');
await close();
// Component failure invalidates that component only; prepared computation survives.
const w=new wasm.PlaybackSession();assert.equal(JSON.parse(w.command(JSON.stringify({operation:'prepare',request}))).status,'prepared');let invalidated=false;
const bad={raster(){return {status:0,pixels:new Uint8Array()};},invalidate(){invalidated=true;}};
const failure=w.render(JSON.stringify({operation:'render',sample}),bad);const failed=JSON.parse(failure.metadata);assert.equal(failed.error.error.error.code,'COMPONENT_INVALID');assert.equal(failure.take_pixels().length,0);assert(invalidated);
const again=w.render(JSON.stringify({operation:'render',sample}),backend);assert.deepEqual(Buffer.from(again.take_pixels()),base.pixels);w.free();
// Native framing failure must terminate without publishing any response.
const transport=[];
for(const [name,input]of [['truncated',Buffer.from([5,0,0,0,123])],['over-limit',Buffer.from([1,0,0,2])]]){
 const r=spawnSync('target/debug/mo-raster-worker',['--playback-session'],{input,env:{},timeout:30000});assert.notEqual(r.status,0);assert.equal(r.stdout.length,0);transport.push({name,exitCode:r.status,stderr:r.stderr.toString()});
}
const result={format:'musteroffice.playback-session-parity/1',previous:entry(previousPath),owners:ownerCount,cases:records,pairedCalls:records.length,replayedCalls:replayed,renderedReplayFrames:rendered,wasmComponentFailureRecovery:true,nativeFramingFailures:transport,artifacts:[entry('target/debug/mo-raster-worker'),entry('.codex-work/source-session/wasm-node/mo_wasm_bg.wasm'),entry('.codex-work/source-session/wasm-node/mo_wasm.js')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({owners:ownerCount,pairedCalls:records.length,replayedCalls:replayed,renderedReplayFrames:rendered}));
