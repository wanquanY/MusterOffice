/** Public Native/WASM clocks, editing, native export and owned page samplers. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/end-conditions',out=root+'/product';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/end-conditions/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,typeof b==='string'||Buffer.isBuffer(b)?b:JSON.stringify(b));return entry(p);};
const records=[],exports=[];let rasters=0,decodes=0,shapes=0,frame=null,owners=0;
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
const authorPath=root+'/author-fixtures.json',author=JSON.parse(fs.readFileSync(authorPath));let evaluated=0,authorFrames=0,sourceFrames=0,staticControls=0;
for(const [ci,c]of author.cases.entries()){
 const eventHistory=binding=>c.events?{binding,through:c.through,events:c.events.map(e=>({...e,generation:binding.generation}))}:null;
 const page=c.page,slide=page.page.slide,doc=page.page.document,init=pure(c.name+'/initialize','dispatch',{operation:'initialize',document:doc});assert.equal(init.response.status,'initialized',JSON.stringify(init.response));const snapshot=init.response.snapshot,binding={session:c.name,revision:snapshot.revision,generation:'11'};
 const live=ci<3||c.name==='nested-cutoff'?owner(false):null;
 if(live)assert.equal((await live.call(c.name+'/prepare',{operation:'prepare',request:{snapshot,slide,binding,viewport:page.viewport,defaults:page.defaults}})).response.status,'prepared');
 for(const [i,s]of c.samples.entries()){
  const q={snapshot,slide,binding,at:s.at,history:eventHistory(binding)},r=pure(c.name+'/'+i,'evaluate',q);assert.equal(r.response.status,'evaluated');const state=r.response.frame.state;assert.deepEqual(state.rotations,s.rotations,c.name+'/'+i+' Fraction');evaluated++;
  if(c.intervals)for(const n of state.nodes){assert.deepEqual(n.start,c.intervals[n.node].start);assert.deepEqual(n.end,c.intervals[n.node].end);}
  if(s.expectedNodes)for(const n of state.nodes){const e=s.expectedNodes[n.node];assert.deepEqual(n.start,e.start);assert.deepEqual(n.end,e.end);assert.equal(n.iteration,e.iteration??null);assert.deepEqual(n.progress,e.progress??null);}
  if(live){
   const one=once(c.name+'/'+i+'/one-shot',{playback:q,viewport:page.viewport,defaults:page.defaults},'--playback-page');assert.deepEqual(one.response.info.frame,r.response.frame);
   const kept=await live.call(c.name+'/'+i+'/retained',{operation:'render',sample:{binding,at:s.at,history:q.history}});assert.deepEqual(kept.response,one.response);assert.deepEqual(kept.pixels,one.pixels);assert.deepEqual(kept.frame,one.frame);authorFrames++;
   if(Object.values(s.rotations).every(v=>v.denominator==='1')){
    const control=structuredClone(page);delete control.page.document.timelines;for(const [id,v]of Object.entries(s.rotations))control.page.document.objects[id].transform.rotation=Number(v.numerator);
    const st=once(c.name+'/'+i+'/static',control,'--page');assert.deepEqual(st.pixels,one.pixels);assert.deepEqual(st.frame,one.frame);staticControls++;
   }
  }
 }
 if(live){assert.equal((await live.call(c.name+'/dispose',{operation:'dispose',binding})).response.status,'disposed');await live.close();}
 // Actual atomic public edits: trees travel through the same setTimeline surface.
 if(ci===1){
  const tx=operations=>({operation:'prepare',snapshot,transaction:{documentId:doc.id,requestId:'tree-edit',baseRevision:snapshot.revision,operations:operations.map((operation,i)=>({operationId:'op'+i,operation}))}});
  const replacement=structuredClone(doc.timelines[slide]);replacement.tree.containers[0].fill=replacement.tree.containers[0].fill==='remove'?'hold':'remove';
  const edited=pure('tree/edit','dispatch',tx([{kind:'setTimeline',slide,timeline:replacement}]));assert.equal(edited.response.status,'prepared');assert(edited.response.receipt.changes.changedTimelines.includes(slide));
  assert.equal(pure('tree/delete-reject','dispatch',tx([{kind:'deleteObject',object:'shape:1',policy:'rejectDependencies'}])).response.error.code,'REFERENCE_CONFLICT');
  const removed=pure('tree/delete-cascade','dispatch',tx([{kind:'deleteObject',object:'shape:1',policy:'cascade'}]));assert.equal(removed.response.status,'prepared');assert.deepEqual(removed.response.snapshot.document.timelines[slide].tree.containers[0].children,[]);
 }
 if(ci===1||ci===2||c.name==='nested-cutoff'){
  const old=JSON.parse(fs.readFileSync('fixtures/presentations/native-export/request.json'));const q={document:doc,defaults:old.defaults,resourceBindings:[]};const json=JSON.stringify(q),req=put(out+'/'+c.name+'.export.json',json),resource=put(out+'/'+c.name+'.resources.bin',Buffer.alloc(0)),path=out+'/'+c.name+'.pptx';
  const metadata=cli(['pptx-export',req.path,resource.path,path]);const n=fs.readFileSync(path),w=wasm.export_pptx(json,Buffer.alloc(0));assert.deepEqual(n,Buffer.from(w));const source=entry(path),index=JSON.parse(wasm.inspect_pptx(n)).index;
  const native=pure(c.name+'/native','timing',{expectedSourceSha256:source.sha256,slide:index.slides[0].part},source);assert.equal(native.response.status,'inspected');assert.equal(native.response.timing.native.timeline.format,'musteroffice.timeline/0.2-draft');assert.equal(native.response.timing.native.timeline.tree.containers.length,doc.timelines[slide].tree.containers.length);
  const imported=structuredClone(doc),timing=structuredClone(native.response.timing.native.timeline),objects=index.surfaces[index.slides[0].part].objects;
  const mapping=Object.fromEntries(Object.entries(native.response.timing.native.objectBindings).map(([id,nativeId])=>[id,objects.find(o=>o.nativeId===nativeId).name]));
  for(const n of timing.nodes){n.effect.target=mapping[n.effect.target];for(const condition of [n.start,...(n.endConditions??[])])if(condition.kind==='click'&&condition.target!==null)condition.target=mapping[condition.target];}
  imported.timelines[slide]=timing;const restored=pure(c.name+'/imported-initialize','dispatch',{operation:'initialize',document:imported});assert.equal(restored.response.status,'initialized');
  const ib={...binding,revision:restored.response.snapshot.revision};
  for(const [i,s]of c.samples.entries()){
   const r=pure(c.name+'/imported-'+i,'evaluate',{snapshot:restored.response.snapshot,slide,binding:ib,at:s.at,history:eventHistory(ib)});assert.equal(r.response.status,'evaluated');assert.deepEqual(r.response.frame.state.rotations,s.rotations);evaluated++;
  }
  exports.push({name:c.name,request:req,resources:resource,output:source,response:put(out+'/'+c.name+'.export-response.json',metadata),timing:native.record.response});
 }
}
// Malformed tree ownership, scoped dependency cycles and container click targets
// must fail at the same public admission boundary on both runtimes.
const original=author.cases[0].page.page.document,slide=author.cases[0].page.page.slide;
for(const [name,change]of [
 ['duplicate-owner',t=>t.tree.roots.push('a')],
 ['missing-child',t=>t.tree.containers[0].children[0]='missing'],
 ['wrong-version',t=>t.format='musteroffice.timeline/0.1-draft'],
 ['parent-end-cycle',t=>t.nodes[0].start={kind:'after',node:'seq',event:'begin',delay:{ticks:'0',timescale:1}}],
 ['unknown-container-click',t=>t.tree.containers[0].start={kind:'click',target:'missing',delay:{ticks:'0',timescale:1}}],
 ['missing-end-reference',t=>t.nodes[0].endConditions=[{kind:'after',node:'missing',event:'end',delay:{ticks:'0',timescale:1}}]],
 ['unknown-end-click',t=>t.nodes[0].endConditions=[{kind:'click',target:'missing',delay:{ticks:'0',timescale:1}}]],
 ['end-self-cycle',t=>t.nodes[0].endConditions=[{kind:'after',node:t.nodes[0].id,event:'end',delay:{ticks:'0',timescale:1}}]],
 ['zero-repeat',t=>t.nodes[0].repeatMilli=0],
 ['negative-repeat-duration',t=>t.nodes[0].repeatDuration={ticks:'-1',timescale:1}],
 ['unknown-repeat-token',t=>t.nodes[0].repeatMilli='infinity'],
 ['zero-speed',t=>t.nodes[0].timeTransform.speedMilliPercent=0],
 ['excess-easing',t=>t.nodes[0].timeTransform.accelerationMilliPercent=100001],
 ['unknown-clock-field',t=>t.nodes[0].timeTransform.hiddenCurve=1],
 ]){
 const doc=structuredClone(original);change(doc.timelines[slide]);
 if(name==='parent-end-cycle')doc.timelines[slide].tree.containers[0].start={kind:'after',node:doc.timelines[slide].nodes[0].id,event:'begin',delay:{ticks:'0',timescale:1}};
 assert.equal(pure('invalid/'+name,'dispatch',{operation:'initialize',document:doc}).response.status,'error');
}
// Undefined infinite reverse has no invented origin and produces no raster.
{
 const doc=structuredClone(original),t=doc.timelines[slide];t.format='musteroffice.timeline/0.1-draft';delete t.tree;
 t.nodes=t.nodes.filter(n=>n.id==='a');t.nodes[0].repeatMilli='indefinite';delete t.nodes[0].repeatDuration;delete t.nodes[0].endConditions;
 t.nodes[0].timeTransform.speedMilliPercent=-125000;t.nodes[0].start={kind:'at',offset:{ticks:'1',timescale:1}};
 const init=pure('unbounded-reverse/initialize','dispatch',{operation:'initialize',document:doc});assert.equal(init.response.status,'initialized');
 const snapshot=init.response.snapshot,binding={session:'undefined-reverse',revision:snapshot.revision,generation:'11'};
 assert.equal(pure('unbounded-reverse/evaluate','evaluate',{snapshot,slide,binding,at:{ticks:'1',timescale:1},history:null}).response.status,'error');
 const page=author.cases[0].page,q={snapshot,slide,binding,at:{ticks:'1',timescale:1},history:null};
 const one=once('unbounded-reverse/one-shot',{playback:q,viewport:page.viewport,defaults:page.defaults},'--playback-page',null,null,'error');assert.equal(one.pixels.length,0);
 const live=owner(false);assert.equal((await live.call('unbounded-reverse/prepare',{operation:'prepare',request:{snapshot,slide,binding,viewport:page.viewport,defaults:page.defaults}})).response.status,'prepared');
 const failed=await live.call('unbounded-reverse/retained',{operation:'render',sample:{binding,at:q.at,history:null}},null,null,'error');assert.equal(failed.pixels.length,0);
 // A semantic sample failure does not corrupt the reusable owner.
 await live.call('unbounded-reverse/before-start',{operation:'render',sample:{binding,at:{ticks:'0',timescale:1},history:null}});
 assert.equal((await live.call('unbounded-reverse/dispose',{operation:'dispose',binding})).response.status,'disposed');await live.close();

}
const sourcePath=root+'/source-fixtures.json',source=JSON.parse(fs.readFileSync(sourcePath));
for(const group of [...new Set(source.cases.map(c=>c.group))]){
 const cases=source.cases.filter(c=>c.group===group),q=JSON.parse(load(cases[0].request)),live=owner(true);
 assert.equal((await live.call(group+'/prepare',{operation:'prepare',request:{page:q.page,binding:q.sample.binding}},cases[0].source,cases[0].fonts)).response.status,'prepared');
 for(const c of cases){
  const q=JSON.parse(load(c.request)),one=once(c.name+'/one-shot',q,'--pptx-playback-page',c.source,c.fonts);assert.deepEqual(one.response.info.playback.evaluated.state.rotations,c.expectedRotations,c.name+' independent Fraction');
  const kept=await live.call(c.name+'/retained',{operation:'render',sample:q.sample});const expected=structuredClone(one.response);expected.info.page.gatherCopyBytes=0;for(const key of ['componentCalls','fontUploadBytes','requestWords'])expected.info.page.textWork[key]=0;
  assert.deepEqual(kept.response,expected);assert.deepEqual(kept.pixels,one.pixels);assert.deepEqual(kept.frame,one.frame);sourceFrames++;
  if(c.staticControl){const st=once(c.name+'/static',JSON.parse(load(c.staticControl.request)),'--pptx-resource-page',c.staticControl.source,c.fonts);assert.deepEqual(st.pixels,one.pixels,c.name);assert.deepEqual(st.frame,one.frame,c.name);staticControls++;}
 }
 assert.equal((await live.call(group+'/dispose',{operation:'dispose',binding:q.sample.binding})).response.status,'disposed');await live.close();
}
fs.rmSync(out+'/input.json');const report={format:'musteroffice.end-conditions-parity/1',authorFixtures:entry(authorPath),sourceFixtures:entry(sourcePath),owners,pairedCalls:records.length,evaluated,authorFrames,sourceFrames,staticControls,cases:records,exports,artifacts:[entry('target/debug/mo-cli'),entry('target/debug/mo-raster-worker'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({owners,pairedCalls:records.length,evaluated,authorFrames,sourceFrames,staticControls,exports:exports.length}));
