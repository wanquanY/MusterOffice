// Development paint workload, not a full PPTX/Musterwork performance gate.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import {spawn} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import {request} from './gradient-raster-fixtures.mjs';
const root='.codex-work/gradient-raster',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
const worker=spawn('target/release/mo-raster-worker',[],{stdio:['pipe','pipe','inherit'],env:{}});
let chunks=[],received=0,expected=null,pending;
worker.stdout.on('data',chunk=>{
 chunks.push(chunk);received+=chunk.length;if(received<8)return;
 if(expected===null){const header=chunks[0].length>=8?chunks[0]:Buffer.concat(chunks,received);expected=8+header.readUInt32LE()+header.readUInt32LE(4);assert(expected<=80*1024*1024);}
 if(received<expected)return;assert.equal(received,expected);assert(pending);
 // Accumulate chunks and copy once; repeated Buffer.concat would make this
 // diagnostic host quadratic in frame size and corrupt the IPC measurement.
 const buffer=Buffer.concat(chunks,received),m=buffer.readUInt32LE(),done=pending;pending=undefined;
 const r={metadata:buffer.subarray(8,8+m).toString(),pixels:buffer.subarray(8+m)};chunks=[];received=0;expected=null;done.resolve(r);
});
worker.on('error',e=>pending?.reject(e));worker.on('exit',code=>{if(pending)pending.reject(Error('worker exit '+code));});
function native(json){return new Promise((resolve,reject)=>{assert(!pending);pending={resolve,reject};const h=Buffer.alloc(4);h.writeUInt32LE(Buffer.byteLength(json));worker.stdin.write(Buffer.concat([h,Buffer.from(json)]));});}
function wasmCall(json){const r=wasm.render_paths(json,component),metadata=r.metadata;return {metadata,pixels:Buffer.from(r.take_pixels())};}
const sha=b=>createHash('sha256').update(b).digest('hex');
const results=[];
try{
 for(const size of [256,1024])for(const kind of ['solid','linear','radial']){
  const q=request();q.viewport.width=q.viewport.height=size;q.viewport.scale={numerator:size,denominator:64};
  const g=q.draws[0].brush.gradient;
  if(kind==='solid')q.draws[0].brush={kind:'solid',rgba:[51,102,153,204]};
  if(kind==='radial')g.geometry={kind:'radial',center:{x:String(32n<<32n),y:String(32n<<32n)},radius:String(20n<<32n)};
  const json=JSON.stringify(q),baseline=await native(json);assert.equal(wasmCall(json).metadata,baseline.metadata);
  const timing={};
  for(const [name,run] of [['nativeWorker',native],['wasmRustAndComponent',wasmCall]]){
   for(let i=0;i<5;i++)await run(json);
   const samples=[];
   for(let i=0;i<25;i++){
    const start=process.hrtime.bigint(),r=await run(json);samples.push(Number(process.hrtime.bigint()-start)/1e6);
    assert.equal(r.metadata,baseline.metadata);assert.equal(sha(r.pixels),JSON.parse(baseline.metadata).info.sha256);
   }
   const sorted=samples.toSorted((a,b)=>a-b);
   timing[name]={samplesMs:samples,medianMs:sorted[12],p95Ms:sorted[Math.ceil(25*0.95)-1]};
  }
  results.push({size,kind,requestSha256:sha(json),responseSha256:sha(baseline.metadata),timing});
 }
}finally{worker.stdin.end();}
const report={format:'musteroffice.gradient-raster-benchmark/1',environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version},policy:{warmups:5,samples:25,cache:'warm process/module; each request reparsed, compiled, drawn, hashed and copied',fonts:'none',images:'none',includes:'Native worker IPC and WASM JS/Rust/component boundary respectively; excludes process/module startup',scope:'Owned full-frame rectangle, one evaluated paint; not imported PPTX, full page, RSS, installer or product latency; no previous-profile performance comparison'},artifacts:Object.fromEntries(['target/release/mo-raster-worker','.codex-work/wasm-node/mo_wasm_bg.wasm','.codex-work/skia/mo-skia.wasm','.codex-work/raster-component/index.js'].map(p=>[p,sha(fs.readFileSync(p))])),results};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(results.map(r=>({size:r.size,kind:r.kind,nativeMs:r.timing.nativeWorker.medianMs,wasmMs:r.timing.wasmRustAndComponent.medianMs}))));
