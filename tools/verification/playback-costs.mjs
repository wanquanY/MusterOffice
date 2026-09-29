/** Compare pinned packaged SDKs on full native-reference pixels and metadata.
 * CPU clocks live only in this diagnostic host. No runtime API is timed by proxy.
 * Inputs: output, old bundle, old pin, new bundle, new pin, pinned case manifest. */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {pathToFileURL} from 'node:url';
import {Worker} from 'node:worker_threads';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
const sha=b=>createHash('sha256').update(b).digest('hex');
const [output,oldBundle,oldPin,newBundle,newPin,manifestPath]=process.argv.slice(2);
assert(output&&oldBundle&&oldPin&&newBundle&&newPin&&manifestPath);
await fs.mkdir(output,{recursive:false});
const inputs={};
async function read(name){const b=await fs.readFile(name);inputs[name]={sha256:sha(b),byteLength:b.length};return b;}
async function load(record){const b=await read(record.path);assert.equal(sha(b),record.sha256);assert.equal(b.length,record.byteLength);return b;}
class Host {
  constructor(worker){
    this.worker=worker;this.next=0;
    this.ready=new Promise((resolve,reject)=>{this.start={resolve,reject};});
    worker.on('message',r=>{
      if(this.start){assert.equal(r.ready,true);this.start.resolve(r);this.start=null;return;}
      const p=this.pending;assert(p&&p.id===r.id,'uncorrelated diagnostic result');this.pending=null;clearTimeout(p.timer);
      if(r.error)p.reject(Object.assign(Error(r.error.message),r.error));else p.resolve(r);
    });
    worker.on('error',e=>this.fail(e));worker.on('exit',code=>this.fail(Error('Worker exit '+code)));
  }
  fail(e){if(this.start){this.start.reject(e);this.start=null;}if(this.pending){clearTimeout(this.pending.timer);this.pending.reject(e);this.pending=null;}}
  call(q){assert(!this.pending);const id=++this.next;return new Promise((resolve,reject)=>{
    this.pending={id,resolve,reject,timer:setTimeout(()=>this.fail(Error('Diagnostic deadline')),60000)};
    try{this.worker.postMessage({id,q});}catch(e){this.fail(e);}
  });}
  async close(){this.fail(Error('Host closed'));await this.worker.terminate();}
}
const bundles=[];
for(const [name,directory,pin] of [['baseline',oldBundle,oldPin],['current',newBundle,newPin]]){
  const m=await read(path.join(directory,'bundle-manifest.json'));assert.equal(sha(m),pin);
  const manifest=JSON.parse(m);assert.equal(manifest.releaseCleared,false);
  for(const f of manifest.files)await load({...f,path:path.join(directory,f.path)});
  const code={};const start=performance.now();
  for(const [key,file]of [['kernel','mo_wasm_bg.wasm'],['raster','mo-skia.wasm'],['text','mo-hb.wasm']])
    code[key]=await WebAssembly.compile(await read(path.join(directory,'runtime',file)));
  bundles.push({name,directory,code,compileMs:performance.now()-start,pin});
}
const cases=JSON.parse(await read(manifestPath)).cases,observations=[];
const scenarios=['baseline/remote','baseline/local','baseline/yielded','current/remote','current/local','current/yielded'];
for(const c of cases){
  const request=JSON.parse(await load(c.prepare)),sample=JSON.parse(await load(c.samples))[0];
  const expectedPixels=await load(c.pixels),expectedInfo=JSON.parse(await load(c.metadata));
  const resources=c.kind==='source'?{source:Uint8Array.from(await load(c.source)),fonts:Uint8Array.from(await load(c.fonts))}:{};
  const hosts=new Map(),runs=[],preparation=[];
  try{
    for(const b of bundles){
      const host=new Host(new Worker(new URL('./playback-cost-worker.mjs',import.meta.url),{workerData:{code:b.code,
        entry:pathToFileURL(path.resolve(b.directory,'index.mjs')).href,dispatch:pathToFileURL(path.resolve(b.directory,'examples/dispatch.mjs')).href}}));
      hosts.set(b.name,host);const ready=await host.ready;
      const started=performance.now(),prepared=await host.call({operation:'prepare',kind:c.kind,request,...resources});
      const prepareMs=performance.now()-started;
      const warmup=await host.call({operation:'localSample',...sample,workUnits:7,yieldMs:0});
      assert.deepEqual(warmup.value.info,expectedInfo);assert.deepEqual(Buffer.from(warmup.value.pixels),expectedPixels);
      preparation.push({version:b.name,ready,prepareMs,prepare:prepared.measurement,firstFrame:warmup.measurement});
    }
    for(let repetition=0;repetition<30;repetition++){
      const offset=repetition%6;
      const order=[...scenarios.slice(offset),...scenarios.slice(0,offset)];if(Math.floor(repetition/6)%2)order.reverse();
      for(const scenario of order){
        const [version,mode]=scenario.split('/'),host=hosts.get(version),measurements=[];
        const started=performance.now();let result;
        if(mode==='remote'){
          measurements.push((await host.call({operation:'beginSample',...sample})).measurement);
          let steps=0;
          do{result=await host.call({operation:'stepSample',workUnits:7});measurements.push(result.measurement);assert(++steps<=1000000);}while(!result.value.complete);
          result=await host.call({operation:'takeSample'});measurements.push(result.measurement);
        }else{
          result=await host.call({operation:'localSample',...sample,workUnits:7,yieldMs:mode==='yielded'?2:0});measurements.push(result.measurement);
        }
        const elapsedMs=performance.now()-started;
        assert.deepEqual(result.value.info,expectedInfo);assert.deepEqual(Buffer.from(result.value.pixels),expectedPixels);
        runs.push({version,mode,repetition,elapsedMs,measurements});
      }
    }
    for(const host of hosts.values())await host.call({operation:'dispose'});
  }finally{for(const host of hosts.values())await host.close();}
  observations.push({name:c.name,kind:c.kind,frameBytes:expectedPixels.length,preparation,runs});
}
for(const [name,record]of Object.entries(inputs)){const b=await fs.readFile(name);assert.equal(sha(b),record.sha256);assert.equal(b.length,record.byteLength);}
const stats=a=>{a.sort((a,b)=>a-b);return {count:a.length,p50:a[Math.ceil(a.length*.5)-1],p95:a[Math.ceil(a.length*.95)-1],max:a.at(-1)};};
const summary=observations.flatMap(c=>scenarios.map(s=>{
  const [version,mode]=s.split('/'),runs=c.runs.filter(r=>r.version===version&&r.mode===mode);
  return {name:c.name,version,mode,elapsedMs:stats(runs.map(r=>r.elapsedMs)),
    longestCallMs:stats(runs.map(r=>mode==='remote'?Math.max(...r.measurements.map(m=>m.workMs)):
      Math.max(r.measurements[0].stages.beginMs,r.measurements[0].stages.maxStepMs,r.measurements[0].stages.takeMs))),
    takeMs:stats(runs.map(r=>mode==='remote'?r.measurements.at(-1).workMs:r.measurements[0].stages.takeMs)),
    messages:stats(runs.map(r=>r.measurements.length))};
}));
const report={status:'passed',hardware:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,totalMemory:os.totalmem()},
  versions:process.versions,bundles:bundles.map(({code,...b})=>b),inputs,summary,observations,
  scope:'Two owned 1440x1080 fixtures, one prepared Worker per version/case, first frame separate then 30 frames per mode. Six-mode order rotated/reversed; component/WASM memories warm, OS cache/load and GC uncontrolled. Full native-reference pixels+metadata checked outside measured interval. No W-D/media/Office/product, RSS, hard cancellation SLA or release acceptance.'};
await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({status:report.status,frames:observations.reduce((n,c)=>n+c.runs.length+2,0),summary},null,2));
