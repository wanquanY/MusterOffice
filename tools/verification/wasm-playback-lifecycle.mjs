/** Worker lifetime and module/owner failure boundaries against actual WASM. */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {Worker,isMainThread,parentPort,workerData} from 'node:worker_threads';
import {createHash} from 'node:crypto';

if(!isMainThread){
  const {createPlaybackRuntime}=await import(workerData.entry);
  globalThis.fetch=()=>{throw Error('unexpected fetch');};
  const reads={kernel:0,raster:0,text:0};let reentry;
  const code={};
  for(const key of Object.keys(reads))Object.defineProperty(code,key,{get(){
    reads[key]++;
    if(key==='kernel'){
      reentry=createPlaybackRuntime(workerData.code);
      reentry.catch(()=>{}); // Checked below; avoid an unhandled asynchronous rejection.
    }
    return workerData.code[key];
  }});
  const runtime=await createPlaybackRuntime(code);
  assert.deepEqual(reads,{kernel:1,raster:1,text:1});
  await assert.rejects(()=>reentry,/already initialized/);
  await assert.rejects(()=>createPlaybackRuntime(workerData.code),/already initialized/);
  const owner=runtime.playback.prepareAuthor(workerData.request);
  if(workerData.mode==='block'){
    owner.sample(workerData.at,{raster(){parentPort.postMessage({blocked:true});
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0);throw Error('unreachable');},invalidate(){}});
    parentPort.postMessage({lateFrame:true});
  }else{
    let invalidated=false;
    const bad={raster(){return {status:0,pixels:new Uint8Array()};},invalidate(){invalidated=true;}};
    assert.throws(()=>owner.sample(workerData.at,bad),e=>e.name==='PlaybackComputationError');
    assert(invalidated);assert.equal(owner.closed,false);
    const frame=owner.sample(workerData.at,runtime.raster);
    const oldBytes=Uint8Array.from(frame.pixels);
    // A healthy independent instance renders the same retained plan. This is
    // not recovery from a poisoned real component; that terminates its Worker.
    runtime.raster=await runtime.createRaster();
    assert.deepEqual(owner.sample(workerData.at,runtime.raster).pixels,oldBytes);
    owner.dispose();owner.close();assert(owner.closed);
    const fatalOwner=runtime.playback.prepareAuthor(workerData.request);
    runtime.raster.invalidate();
    assert.throws(()=>fatalOwner.sample(workerData.at,runtime.raster),e=>e.name==='PlaybackComputationError');
    assert(runtime.raster.invalid);
    // Parent terminates this actual Worker immediately after observing the fault.
    parentPort.postMessage({recovered:true,fatalComponentDetected:true,
      pixelSha256:createHash('sha256').update(oldBytes).digest('hex')});
  }
}else{
  const [output,bundle,pin,author]=process.argv.slice(2);assert(output&&bundle&&pin&&author);
  await fs.mkdir(output,{recursive:false});
  const sha=b=>createHash('sha256').update(b).digest('hex');
  const raw=await fs.readFile(path.join(bundle,'bundle-manifest.json'));assert.equal(sha(raw),pin);
  const manifest=JSON.parse(raw);
  for(const f of manifest.files)assert.equal(sha(await fs.readFile(path.join(bundle,f.path))),f.sha256);
  const code={};
  for(const [key,name] of [['kernel','mo_wasm_bg.wasm'],['raster','mo-skia.wasm'],['text','mo-hb.wasm']])
    code[key]=await WebAssembly.compile(await fs.readFile(path.join(bundle,'runtime',name)));
  const reference=JSON.parse(await fs.readFile(author));const first=reference.cases[0];
  const requestBytes=await fs.readFile(first.request.path);assert.equal(sha(requestBytes),first.request.sha256);
  const q=JSON.parse(requestBytes);
  const request={binding:q.playback.binding,snapshot:q.playback.snapshot,slide:q.playback.slide,viewport:q.viewport,defaults:q.defaults};
  const entry=pathToFileURL(path.resolve(bundle,'index.mjs')).href;
  const events=[];
  async function run(mode){
    const w=new Worker(new URL(import.meta.url),{workerData:{code,entry,request,at:q.playback.at,mode}});
    let timer;
    try{
      return await new Promise((resolve,reject)=>{
        timer=setTimeout(()=>reject(Error('lifecycle test deadline')),15000);
        w.on('error',reject);
        w.on('message',async m=>{
          events.push({mode,...m});
          if(m.blocked){try{const exitCode=await w.terminate();resolve({blocked:true,exitCode});}catch(error){reject(error);}}
          else if(m.recovered)resolve(m);else reject(Error('unexpected/late worker output'));
        });
      });
    }finally{clearTimeout(timer);await w.terminate();}
  }
  const blocked=await run('block');assert(blocked.blocked);assert.equal(events.filter(e=>e.mode==='block').length,1);
  const recovery=await run('recover');assert(recovery.recovered&&recovery.fatalComponentDetected);
  assert.equal(recovery.pixelSha256,first.pixels.sha256);
  // Host example deadline settles only after terminate(), and cannot be reused.
  const {openWorker}=await import(pathToFileURL(path.resolve(bundle,'examples/node-worker.mjs')));
  const host=await openWorker(bundle,pin);
  try{
    await assert.rejects(()=>host.call({operation:'prepare',kind:'author',request},[],0.001),/deadline/);
    await assert.rejects(()=>host.call({operation:'timing'}),/closed/);
  }finally{await host.close();}
  const report={format:'musteroffice.wasm-playback-lifecycle/1',status:'passed',bundleManifestSha256:pin,
    requestSha256:first.request.sha256,blockedWorkerTerminated:blocked,events,
    duplicateRuntimeRejected:true,reentrantRuntimeRejected:true,moduleAccessorsReadOnce:true,
    invalidExternalReplyPreservesPlanWithHealthyComponent:true,healthyComponentReplacementKeepsPixels:true,
    invalidatedRealComponentRequiresWorkerTermination:true,
    disposalIdempotentClose:true,hostDeadlineStopsBeforeReject:true,scope:'Actual Node Worker and WASM; no browser or full PPT claim.'};
  await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({status:report.status,blockedWorkerTerminated:true,recoveredPixelSha256:recovery.pixelSha256}));
}
