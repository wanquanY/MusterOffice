/** Retained interval ownership: actual Native/WASM work counters, frames and pixels. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/retained-timing',out=root+'/product';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/retained-timing/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,typeof b==='string'||Buffer.isBuffer(b)?b:JSON.stringify(b));return entry(p);};
const records=[];let rasters=0,decodes=0,shapes=0,frame=null,owners=0;
const raster={raster(f){rasters++;frame=f.slice();return component.raster(f);},rasterImages(f,b){rasters++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
const text={shapeBatch(...a){shapes++;return shaper.shapeBatch(...a);},measureBatch(...a){shapes++;return shaper.measureBatch(...a);},outlineBatch(...a){shapes++;return shaper.outlineBatch(...a);},invalidate(){shaper.invalidate();}};
const reset=()=>{rasters=decodes=shapes=0;frame=null;};
function cli(args,input){const r=spawnSync('target/debug/mo-cli',args,{input,env:{},timeout:60000,maxBuffer:80<<20});assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0);return r.stdout.toString().trimEnd();}
function record(name,operation,q,metadata,pixels=Buffer.alloc(0),extra={}){
 const p=out+'/'+records.length,record={name,operation,request:put(p+'.request.json',q),response:put(p+'.response.json',metadata),pixels:put(p+'.rgba',pixels),rasters,decodes,shapes,...extra,...(frame?{frame:put(p+'.frame',Buffer.from(frame.buffer))}:{})};records.push(record);return {record,response:JSON.parse(metadata),pixels,frame};
}
function pure(name,operation,q,source=null){
 reset();const json=JSON.stringify(q),temp=put(out+'/input.json',json);
 const n=cli(operation==='dispatch'?[]:operation==='evaluate'?['evaluate-timeline',temp.path]:['pptx-timing',temp.path,source.path],operation==='dispatch'?json:undefined);
 const w=operation==='dispatch'?wasm.dispatch_json(json):operation==='evaluate'?wasm.evaluate_timeline(json):wasm.inspect_pptx_timing(json,load(source));assert.equal(n,w,name);
 return record(name,operation,q,n,Buffer.alloc(0),source?{source}:{});
}
function packet(json,source,fonts){const j=Buffer.from(json),s=source?load(source):Buffer.alloc(0),f=fonts?load(fonts):Buffer.alloc(0),h=Buffer.alloc(source?12:4);h.writeUInt32LE(j.length);if(source){h.writeUInt32LE(s.length,4);h.writeUInt32LE(f.length,8);}return Buffer.concat([h,j,s,f]);}
function result(b){const ml=b.readUInt32LE(),pl=b.readUInt32LE(4);assert.equal(b.length,8+ml+pl);return {metadata:b.subarray(8,8+ml).toString(),pixels:b.subarray(8+ml)};}
function once(name,q,mode,source=null,fonts=null,expectedStatus='rendered'){
 const json=JSON.stringify(q),n=spawnSync('target/debug/mo-raster-worker',[mode],{input:packet(json,source,fonts),env:{},timeout:60000,maxBuffer:80<<20});assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);const native=result(n.stdout);reset();
 const fn={'--page':'render_page','--playback-page':'render_playback_page','--pptx-playback-page':'render_pptx_playback_page','--pptx-resource-page':'render_pptx_resource_page'}[mode];
 const w=source?wasm[fn](json,load(source),load(fonts),decoder,text,raster):wasm[fn](json,raster),metadata=w.metadata,pixels=Buffer.from(w.take_pixels());assert.equal(metadata,native.metadata,name);assert.deepEqual(pixels,native.pixels,name);assert(!component.invalid);assert(!shaper.invalid);
 const r=record(name,mode,q,metadata,pixels,source?{source,fonts}:{});assert.equal(r.response.status,expectedStatus,JSON.stringify(r.response));assert.equal(rasters,expectedStatus==='rendered'?1:0);return r;
}
function owner(sourceMode){
 const id=owners++,w=sourceMode?new wasm.PptxPlaybackSession():new wasm.PlaybackSession(),n=spawn('target/debug/mo-raster-worker',[sourceMode?'--pptx-playback-session':'--playback-session'],{env:{},stdio:['pipe','pipe','pipe']});let buffer=Buffer.alloc(0),pending=null,stderr='';
 n.stdout.on('data',b=>{buffer=Buffer.concat([buffer,b]);if(buffer.length>=8&&buffer.length>=8+buffer.readUInt32LE()+buffer.readUInt32LE(4)){assert(pending);const length=8+buffer.readUInt32LE()+buffer.readUInt32LE(4),r=result(buffer.subarray(0,length));buffer=buffer.subarray(length);clearTimeout(pending.timer);pending.resolve(r);pending=null;}});n.stderr.on('data',b=>stderr+=b);
 const exit=new Promise((resolve,reject)=>{n.on('error',reject);n.on('close',code=>{if(pending){clearTimeout(pending.timer);pending.reject(Error(stderr));pending=null;}resolve(code);});});
 return {async call(name,q,source=null,fonts=null,expectedStatus='rendered'){
  const json=JSON.stringify(q),input=sourceMode?(()=>{const j=Buffer.from(json),s=source?load(source):Buffer.alloc(0),f=fonts?load(fonts):Buffer.alloc(0),h=Buffer.alloc(12);h.writeUInt32LE(j.length);h.writeUInt32LE(s.length,4);h.writeUInt32LE(f.length,8);return Buffer.concat([h,j,s,f]);})():packet(json);
  const native=await new Promise((resolve,reject)=>{assert(!pending);pending={resolve,reject,timer:setTimeout(()=>{n.kill();reject(Error('owner timeout'));},30000)};n.stdin.write(input);});reset();let metadata,pixels=Buffer.alloc(0);
  if(q.operation==='render'){const r=w.render(json,raster);metadata=r.metadata;pixels=Buffer.from(r.take_pixels());}
  else if(sourceMode&&q.operation==='prepare'){const s=load(source),f=load(fonts);metadata=w.prepare(json,s,f,decoder,text);s.fill(0);f.fill(0);}
  else metadata=w.command(json);
  assert.equal(metadata,native.metadata,name);assert.deepEqual(pixels,native.pixels,name);assert(!component.invalid);assert(!shaper.invalid);const r=record(name,'owner-'+q.operation,q,metadata,pixels,{owner:id,...(source?{source,fonts}:{})});
  if(q.operation==='render'){assert.equal(r.response.status,expectedStatus,JSON.stringify(r.response));assert.equal(rasters,expectedStatus==='rendered'?1:0);assert.equal(decodes+shapes,0);}return r;
 },async close(){w.free();n.stdin.end();assert.equal(await exit,0,stderr);assert.equal(stderr,'');assert.equal(buffer.length,0);}};
}
const consumed=sample=>{
 const events=sample.history?.events??[],unique=[];
 for(const event of events){
  if(unique.at(-1)?.sequence===event.sequence)continue;
  if(BigInt(event.at.ticks)*BigInt(sample.at.timescale)>BigInt(sample.at.ticks)*BigInt(event.at.timescale))break;
  unique.push(event);
 }
 return unique;
};
function tracking(){return {built:0,reused:0,key:null,count:0,binding:null,intervals:0};}
function sampled(track,sample,frame){
 const prefix=consumed(sample),key=JSON.stringify([sample.binding,prefix]);
 if(key===track.key)track.reused++;else track.built++;
 Object.assign(track,{key,count:prefix.length,binding:sample.binding,intervals:frame.state.nodes.length+(frame.state.containers?.length??0)});
}
let inspected=0,authorFrames=0,sourceFrames=0;
async function inspect(live,name,binding,track){
 const r=await live.call(name,{operation:'inspectTiming',binding});assert.equal(r.response.status,'timingInspected');
 assert.deepEqual(r.response.info.binding,binding);const actual=r.response.info.sampler;
 for(const [key,value]of Object.entries({schedulesBuilt:track.built,schedulesReused:track.reused,retainedIntervals:track.intervals,retainedEvents:track.count}))assert.equal(actual[key],String(value),name+'/'+key);
 assert.deepEqual(actual.cachedBinding,track.binding);assert.equal(r.record.rasters+r.record.decodes+r.record.shapes,0);inspected++;return r.response;
}
async function advance(live,name,binding,track){
 const before=await inspect(live,name+'/before',binding,track);
 assert.equal((await live.call(name+'/failed',{operation:'advance',binding,generation:binding.generation})).response.error.code,'GENERATION_NOT_INCREASING');
 assert.deepEqual(await inspect(live,name+'/failed-unchanged',binding,track),before);
 const generation=String(BigInt(binding.generation)+1n),r=await live.call(name+'/advance',{operation:'advance',binding,generation});assert.equal(r.response.status,'advanced');
 const next=r.response.info.binding;Object.assign(track,{key:null,count:0,binding:null,intervals:0});await inspect(live,name+'/cleared',next,track);
 assert.equal((await live.call(name+'/stale',{operation:'inspectTiming',binding})).response.error.code,'BINDING_CONFLICT');return next;
}
async function dispose(live,name,binding){
 assert.equal((await live.call(name+'/dispose',{operation:'dispose',binding})).response.status,'disposed');
 assert.equal((await live.call(name+'/disposed',{operation:'inspectTiming',binding})).response.error.code,'DISPOSED');
 await live.close();
}
const authorPath='.codex-work/container-lifecycle/author-fixtures.json',author=JSON.parse(fs.readFileSync(authorPath));
for(const c of author.cases.filter((c,i)=>i<3||c.name==='nested-cutoff')){
 const page=c.page,slide=page.page.slide,init=pure(c.name+'/initialize','dispatch',{operation:'initialize',document:page.page.document});assert.equal(init.response.status,'initialized');
 const snapshot=init.response.snapshot,binding={session:c.name,revision:snapshot.revision,generation:'11'},live=owner(false),track=tracking();
 const history={binding,through:c.through,events:c.events.map(e=>({...e,generation:binding.generation}))};
 assert.equal((await live.call(c.name+'/prepare',{operation:'prepare',request:{snapshot,slide,binding,viewport:page.viewport,defaults:page.defaults}})).response.status,'prepared');
 await inspect(live,c.name+'/empty',binding,track);
 const samples=[...c.samples,...c.samples.slice(0,3).reverse()];
 for(const [i,s]of samples.entries()){
  const sample={binding,at:s.at,history},one=once(c.name+'/'+i+'/one-shot',{playback:{snapshot,slide,...sample},viewport:page.viewport,defaults:page.defaults},'--playback-page');
  const kept=await live.call(c.name+'/'+i+'/retained',{operation:'render',sample});assert.deepEqual(kept.response,one.response);assert.deepEqual(kept.pixels,one.pixels);assert.deepEqual(kept.frame,one.frame);assert.deepEqual(kept.response.info.frame.state.rotations,s.rotations);
  sampled(track,sample,kept.response.info.frame);await inspect(live,c.name+'/'+i+'/work',binding,track);authorFrames++;
 }
 const bad={binding,at:c.samples[0].at,history:structuredClone(history)};bad.history.events.at(-1).generation='999';
 assert.equal((await live.call(c.name+'/bad-future',{operation:'render',sample:bad},null,null,'error')).response.status,'error');await inspect(live,c.name+'/bad-unchanged',binding,track);
 const next=await advance(live,c.name,binding,track),sample={binding:next,at:c.samples[0].at,history:{...history,binding:next,events:history.events.map(e=>({...e,generation:next.generation}))}};
 const kept=await live.call(c.name+'/new-generation',{operation:'render',sample});sampled(track,sample,kept.response.info.frame);await inspect(live,c.name+'/new-work',next,track);authorFrames++;
 await dispose(live,c.name,next);
}
const sourcePath='.codex-work/container-lifecycle/source-fixtures.json',source=JSON.parse(fs.readFileSync(sourcePath));
for(const group of [...new Set(source.cases.map(c=>c.group))]){
 const cases=source.cases.filter(c=>c.group===group),first=cases[0],q=JSON.parse(load(first.request)),live=owner(true),track=tracking(),binding=q.sample.binding;
 assert.equal((await live.call(group+'/prepare',{operation:'prepare',request:{page:q.page,binding}},first.source,first.fonts)).response.status,'prepared');
 await inspect(live,group+'/empty',binding,track);
 for(const [i,c]of [...cases,...cases.slice(0,3).reverse()].entries()){
  const q=JSON.parse(load(c.request)),one=once(c.name+'/'+i+'/one-shot',q,'--pptx-playback-page',c.source,c.fonts);
  const kept=await live.call(c.name+'/'+i+'/retained',{operation:'render',sample:q.sample}),expected=structuredClone(one.response);expected.info.page.gatherCopyBytes=0;for(const k of ['componentCalls','fontUploadBytes','requestWords'])expected.info.page.textWork[k]=0;
  assert.deepEqual(kept.response,expected);assert.deepEqual(kept.pixels,one.pixels);assert.deepEqual(kept.frame,one.frame);assert.deepEqual(kept.response.info.playback.evaluated.state.rotations,c.expectedRotations);
  sampled(track,q.sample,kept.response.info.playback.evaluated);await inspect(live,c.name+'/'+i+'/work',binding,track);sourceFrames++;
 }
 const next=await advance(live,group,binding,track),sample=structuredClone(q.sample);sample.binding=next;
 if(sample.history){sample.history.binding=next;for(const e of sample.history.events)e.generation=next.generation;}
 const kept=await live.call(group+'/new-generation',{operation:'render',sample});sampled(track,sample,kept.response.info.playback.evaluated);await inspect(live,group+'/new-work',next,track);sourceFrames++;
 await dispose(live,group,next);
}
fs.rmSync(out+'/input.json');
const report={format:'musteroffice.retained-timing-parity/1',authorFixtures:entry(authorPath),sourceFixtures:entry(sourcePath),owners,pairedCalls:records.length,inspected,authorFrames,sourceFrames,cases:records,artifacts:[entry('target/debug/mo-cli'),entry('target/debug/mo-raster-worker'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({owners,pairedCalls:records.length,inspected,authorFrames,sourceFrames}));
