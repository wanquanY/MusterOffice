/** Interleaved public timeline query cost; excludes pixels, I/O and media. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import os from 'node:os';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
const root='.codex-work/container-lifecycle',require=createRequire(import.meta.url);
const previous=require('../../.codex-work/end-conditions/wasm-node/mo_wasm.js');
const current=require('../../.codex-work/container-lifecycle/wasm-node/mo_wasm.js');
const hash=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:hash(b)};};
const product=JSON.parse(fs.readFileSync('.codex-work/end-conditions/product.json'));
const definitions=[];
for(const name of ['end-0/8','end-1/8','end-2/8','nested-cutoff/5']){
 const c=product.cases.find(c=>c.name===name);assert(c);assert.deepEqual(entry(c.request.path),c.request);
 definitions.push({name,request:JSON.parse(fs.readFileSync(c.request.path)),input:c.request});
}
for(const count of [100,1000]){
 const q=structuredClone(definitions[0].request),doc=q.snapshot.document,slide=q.slide,t=doc.timelines[slide],base=structuredClone(t.nodes[0]);t.format='musteroffice.timeline/0.1-draft';delete t.tree;
 t.nodes=Array.from({length:count},(_,i)=>({...structuredClone(base),id:'scale-'+i,start:{kind:'at',offset:{ticks:'0',timescale:1}},duration:{ticks:'2',timescale:1},repeatMilli:1000,endConditions:[{kind:'at',offset:{ticks:'3',timescale:1}}]}));
 const request=JSON.stringify({operation:'initialize',document:doc});const a=previous.dispatch_json(request),b=current.dispatch_json(request);assert.equal(a,b);const init=JSON.parse(a);assert.equal(init.status,'initialized');q.snapshot=init.snapshot;q.binding.revision=init.snapshot.revision;q.history=null;
 definitions.push({name:'flat-'+count,request:q});
}
const results=[];
for(const c of definitions){
 const json=JSON.stringify(c.request),old=previous.evaluate_timeline(json),now=current.evaluate_timeline(json);assert.equal(old,now);assert.equal(JSON.parse(now).status,'evaluated');
 const inputPath=root+'/benchmark-'+c.name.replaceAll('/','-')+'.request.json';fs.writeFileSync(inputPath,json);
 const batch=c.name==='flat-1000'?2:5,rounds=21;
 for(let i=0;i<3;i++){previous.evaluate_timeline(json);current.evaluate_timeline(json);}
 const oldMs=[],newMs=[];
 function measure(module){const start=performance.now();for(let i=0;i<batch;i++)assert.equal(module.evaluate_timeline(json),now);return (performance.now()-start)/batch;}
 for(let i=0;i<rounds;i++){
  if(i%2===0){oldMs.push(measure(previous));newMs.push(measure(current));}
  else{newMs.push(measure(current));oldMs.push(measure(previous));}
 }
 const summarize=xs=>{const x=xs.toSorted((a,b)=>a-b);return {median:x[Math.floor(x.length/2)],p95:x[Math.ceil(x.length*.95)-1],min:x[0],max:x.at(-1)};};
 const before=summarize(oldMs),after=summarize(newMs);
 results.push({name:c.name,input:entry(inputPath),parentInput:c.input??null,requestSha256:hash(json),responseSha256:hash(now),warmups:3,batch,rounds,oldMs,newMs,before,after,ratio:after.median/before.median});
}
const report={format:'musteroffice.container-lifecycle-query-cost/1',environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,node:process.version},scope:'Warm WASM public evaluate_timeline including JSON, snapshot validation, plan compile, scheduling, exact sampling and serialization; no render, fonts, media, disk, network, Native worker, cold start, RSS or installer. Interleaved same-process modules, same requests and exact equal responses.',artifacts:{previous:entry('.codex-work/end-conditions/wasm-node/mo_wasm_bg.wasm'),current:entry(root+'/wasm-node/mo_wasm_bg.wasm')},cases:results};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(results.map(c=>({name:c.name,before:c.before,after:c.after,ratio:c.ratio}))));
