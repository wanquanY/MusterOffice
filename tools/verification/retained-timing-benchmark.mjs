/** Actual prepared owner cost with equal public responses; no snapshot recompile per sample. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import os from 'node:os';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import factory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
const root='.codex-work/retained-timing',require=createRequire(import.meta.url);
const previous=require('../../.codex-work/container-lifecycle/wasm-node/mo_wasm.js'),current=require('../../.codex-work/retained-timing/wasm-node/mo_wasm.js');
const hash=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:hash(b)};};
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const base=JSON.parse(fs.readFileSync('fixtures/presentations/playback/page.json')),results=[];
const summary=xs=>{const s=xs.toSorted((a,b)=>a-b);return {median:s[10],p95:s[19],min:s[0],max:s.at(-1)};};
for(const [name,count,operation,miss]of [['compile-small',2,'compile',false],['compile-100',100,'compile',false],['compile-1000',1000,'compile',false],['render-small',2,'render',false],['render-1000',1000,'render',false],['compile-1000-changing-prefix',1000,'compile',true]]){
 const page=structuredClone(base),slide=page.page.slide,t=page.page.document.timelines[slide],node=t.nodes[0];
 t.format='musteroffice.timeline/0.1-draft';delete t.tree;
 t.nodes=Array.from({length:count},(_,i)=>({...structuredClone(node),id:'node-'+i,start:{kind:'at',offset:{ticks:'0',timescale:1}},duration:{ticks:'2',timescale:1},repeatMilli:1000}));
 const initialize=JSON.stringify({operation:'initialize',document:page.page.document});const init=current.dispatch_json(initialize);assert.equal(init,previous.dispatch_json(initialize));const parsed=JSON.parse(init);assert.equal(parsed.status,'initialized');
 const snapshot=parsed.snapshot,binding={session:name,revision:snapshot.revision,generation:'1'},prepare={operation:'prepare',request:{snapshot,slide,binding,viewport:page.viewport,defaults:page.defaults}};
 const old=new previous.PlaybackSession(),now=new current.PlaybackSession();assert.equal(old.command(JSON.stringify(prepare)),now.command(JSON.stringify(prepare)));
 const sample={operation,sample:{binding,at:{ticks:'1',timescale:3},history:null}};
 const requests=[0,1].map(i=>{
  const q=structuredClone(sample);
  if(miss)q.sample.history={binding,through:{ticks:'1',timescale:1},events:[{generation:'1',sequence:1,at:{ticks:String(i),timescale:10},event:{kind:'click',target:null}}]};
  return JSON.stringify(q);
 });
 const invoke=(owner,json)=>{
  if(operation==='compile')return {metadata:owner.command(json),pixels:Buffer.alloc(0)};
  const r=owner.render(json,component);return {metadata:r.metadata,pixels:Buffer.from(r.take_pixels())};
 };
 const expected=requests.map(json=>{const a=invoke(old,json),b=invoke(now,json);assert.deepEqual(a,b);assert.equal(JSON.parse(b.metadata).status,operation==='compile'?'compiled':'rendered');return b;});
 for(let i=0;i<3;i++)for(const owner of [old,now])invoke(owner,requests[i%2]);
 const rounds=21,batch=miss?4:count===1000?2:5,oldMs=[],newMs=[];
 const measure=owner=>{const start=performance.now();for(let i=0;i<batch;i++){const at=i%2,r=invoke(owner,requests[at]);assert.equal(r.metadata,expected[at].metadata);assert.deepEqual(r.pixels,expected[at].pixels);}return (performance.now()-start)/batch;};
 for(let round=0;round<rounds;round++){
  if(round%2){newMs.push(measure(now));oldMs.push(measure(old));}else{oldMs.push(measure(old));newMs.push(measure(now));}
 }
 const timing=JSON.parse(now.command(JSON.stringify({operation:'inspectTiming',binding}))).info.sampler;
 if(miss){assert.equal(timing.schedulesReused,'1');assert.equal(timing.schedulesBuilt,String(4+rounds*batch));}
 else{assert.equal(timing.schedulesBuilt,'1');assert.equal(timing.schedulesReused,String(4+rounds*batch));}
 const inputPath=root+'/benchmark-'+name+'.json';fs.writeFileSync(inputPath,JSON.stringify({prepare,requests}));
 const before=summary(oldMs),after=summary(newMs);
 results.push({name,count,authorObjects:Object.keys(page.page.document.objects).length,viewport:page.viewport,operation,cache:miss?'changing-prefix':'same-prefix',input:entry(inputPath),responseSha256:expected.map(r=>hash(r.metadata)),pixelsSha256:expected.map(r=>hash(r.pixels)),warmups:3,batch,rounds,oldMs,newMs,before,after,ratio:after.median/before.median,timing});
 old.free();now.free();
}
const report={format:'musteroffice.retained-timing-owner-cost/1',environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,node:process.version},scope:'Warm WASM prepared author owner, same process interleaved old/new modules, identical input and exact response/pixel equality. Compile includes JSON, complete input-history validation, timing, placement, scene preparation, serialization and assertions. Render additionally uses the same actual C++ raster component and copies/asserts pixels. Preparation excluded. No font/media/imported-source workload, Native IPC throughput, cold start, RSS, display compositor or installer; not measured slideshow FPS.',artifacts:{previous:entry('.codex-work/container-lifecycle/wasm-node/mo_wasm_bg.wasm'),current:entry(root+'/wasm-node/mo_wasm_bg.wasm'),cppWasm:entry('.codex-work/gradient-coordinates/component/mo-skia.wasm')},cases:results};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(results.map(c=>({name:c.name,before:c.before,after:c.after,ratio:c.ratio}))));
