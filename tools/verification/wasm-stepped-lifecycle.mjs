/** Actual WASM ownership and injected reply faults; each case has a fresh Worker. */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {Worker,isMainThread,parentPort,workerData} from 'node:worker_threads';
import {createHash} from 'node:crypto';
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
if(!isMainThread) {
  globalThis.fetch=()=>{throw Error('Implicit fetch denied');};
  const {createPlaybackRuntime}=await import(workerData.entry);
  const runtime=await createPlaybackRuntime(workerData.code);
  const kernel=await import(workerData.kernel);
  const {kind,mode,request,sample}=workerData;
  const inputs={source:workerData.source,fonts:workerData.fonts,decoder:runtime.raster,shaping:runtime.shaping};
  function rawOwner() {
    const owner=kind==='author'?new kernel.PlaybackSession():new kernel.PptxPlaybackSession();
    const q=JSON.stringify({operation:'prepare',request});
    const result=kind==='author'?owner.command(q):owner.prepare(q,inputs.source,inputs.fonts,inputs.decoder,inputs.shaping);
    assert.equal(JSON.parse(result).status,'prepared');return owner;
  }
  const q=JSON.stringify({operation:'render',sample:{binding:request.binding,...sample}});
  function rendered(owner) {
    const pending=owner.prepare_render(q);assert.equal(pending.failure,'');
    const started=pending.begin(runtime.raster);assert.equal(started.status,0);
    assert.throws(()=>pending.begin(runtime.raster),/already started/);
    while(!started.execution.step(7).complete) {}
    const reply=started.execution.take();started.execution.close();
    assert.equal(sha(reply.pixels),workerData.pixelSha256);
    return {pending,reply};
  }
  function finish(owner,pending,reply) {
    const result=owner.complete_render(pending,reply);
    const invalidates=result.invalidates_backend;
    const response=JSON.parse(result.metadata),pixels=result.take_pixels();
    assert.throws(()=>pending.free()); // wasm-bindgen consumed this exact allocation.
    if(response.status==='error')assert.equal(pixels.length,0);
    if(invalidates)runtime.raster.invalidate();
    return {response,invalidates};
  }
  if(mode==='cancel') {
    const owner=kind==='author'?runtime.playback.prepareAuthor(request):runtime.playback.prepareSource(request,inputs);
    let count=0;
    for(const after of [0,1,4,16]) {
      const execution=owner.beginSample(sample.at,runtime.raster,sample.history);
      assert.throws(()=>owner.timing(),e=>e.code==='BUSY');
      for(let i=0;i<after;i++)execution.step(1);
      execution.close();execution.close();assert(execution.closed);
      assert(!owner.closed&&!runtime.raster.invalid);
      const next=owner.beginSample(sample.at,runtime.raster,sample.history);
      while(!next.step(7)) {}
      assert.equal(sha(next.take().pixels),workerData.pixelSha256);
      assert(next.closed);count++;
    }
    owner.dispose();parentPort.postMessage({passed:true,cancellations:count});
  } else {
    const owner=rawOwner();
    if(mode==='prepare-error') {
      const pending=owner.prepare_render('{');
      assert.equal(JSON.parse(pending.failure).status,'error');
      assert.throws(()=>pending.begin(runtime.raster),/preparation failed/);pending.free();
      const {pending:valid,reply}=rendered(owner);
      assert.equal(finish(owner,valid,reply).response.status,'rendered');
    } else {
      const {pending,reply}=rendered(owner);
      if(mode==='other-owner') {
        const other=rawOwner();const result=finish(other,pending,reply);
        assert.equal(result.response.error.code,'BINDING_CONFLICT');assert(!result.invalidates);other.free();
      } else if(mode==='advance'||mode==='dispose') {
        const generation=(BigInt(request.binding.generation)+1n).toString();
        const result=JSON.parse(owner.command(JSON.stringify({operation:mode,binding:request.binding,...(mode==='advance'?{generation}:{})})));
        assert.equal(result.status,mode==='advance'?'advanced':'disposed');
        const completed=finish(owner,pending,reply);assert(!completed.invalidates);
        assert.equal(completed.response.error.code,mode==='advance'?'BINDING_CONFLICT':'DISPOSED');
      } else {
        let incoming=reply;
        if(mode==='status')incoming={status:NaN,pixels:reply.pixels};
        if(mode==='status-getter')incoming={get status(){throw Error('status getter');},pixels:reply.pixels};
        if(mode==='pixels-getter')incoming={status:0,get pixels(){throw Error('pixels getter');}};
        if(mode==='type')incoming={status:0,pixels:{length:2**32}};
        if(mode==='length')incoming={status:0,pixels:new Uint8Array()};
        if(mode==='alpha'){reply.pixels[0]=255;reply.pixels[3]=0;}
        if(mode==='failure-bytes')incoming={status:1,pixels:reply.pixels};
        if(mode==='recoverable')incoming={status:3,pixels:new Uint8Array()};
        const result=finish(owner,pending,incoming);
        assert.equal(result.response.status,'error');assert.equal(result.invalidates,mode!=='recoverable');
        if(mode==='recoverable') {
          assert(!runtime.raster.invalid);const next=rendered(owner);
          assert.equal(finish(owner,next.pending,next.reply).response.status,'rendered');
        } else assert(runtime.raster.invalid); // Parent destroys this Worker, never replaces the poisoned component.
      }
    }
    owner.free();parentPort.postMessage({passed:true,quarantined:runtime.raster.invalid});
  }
} else {
  const [output,bundle,pin,reference]=process.argv.slice(2);assert(output&&bundle&&pin&&reference);
  await fs.mkdir(output,{recursive:false});const inputs={};
  async function read(name){const bytes=await fs.readFile(name);inputs[name]=sha(bytes);return bytes;}
  const manifest=JSON.parse(await read(path.join(bundle,'bundle-manifest.json')));
  assert.equal(inputs[path.join(bundle,'bundle-manifest.json')],pin);
  for(const file of manifest.files)assert.equal(sha(await read(path.join(bundle,file.path))),file.sha256);
  const code={};
  for(const [key,name] of [['kernel','mo_wasm_bg.wasm'],['raster','mo-skia.wasm'],['text','mo-hb.wasm']])
    code[key]=await WebAssembly.compile(await read(path.join(bundle,'runtime',name)));
  const observations=[];
  for(const kind of ['author','source']) {
    const directory=path.join(reference,kind+'-000');
    const request=JSON.parse(await read(path.join(directory,'prepare.json')));
    const sample=JSON.parse(await read(path.join(directory,'samples.json')))[0];
    const pixels=await read(path.join(directory,'output/0000.rgba'));
    const source=kind==='source'?await read(path.join(directory,'source.bin')):null;
    const fonts=kind==='source'?await read(path.join(directory,'fonts.bin')):null;
    for(const mode of ['cancel','prepare-error','other-owner','advance','dispose','status','status-getter','pixels-getter','type','length','alpha','failure-bytes','recoverable']) {
      const worker=new Worker(new URL(import.meta.url),{workerData:{kind,mode,request,sample,source,fonts,code,pixelSha256:sha(pixels),
        entry:pathToFileURL(path.resolve(bundle,'index.mjs')).href,kernel:pathToFileURL(path.resolve(bundle,'runtime/mo_wasm.js')).href}});
      let timer,result,exitCode;
      try {result=await new Promise((resolve,reject)=>{
        timer=setTimeout(()=>reject(Error('Stepped lifecycle deadline')),15000);
        worker.on('error',reject);worker.once('message',resolve);
      });assert(result.passed);}finally{clearTimeout(timer);exitCode=await worker.terminate();}
      observations.push({kind,mode,...result,exitCode});
    }
  }
  for(const [name,digest]of Object.entries(inputs))assert.equal(sha(await fs.readFile(name)),digest);
  const report={status:'passed',observations,inputs,bundleManifestSha256:pin,
    scope:'26 fresh real WASM Workers; injected output faults test validation/quarantine, not alternate rendering or complete cancellation SLA.'};
  await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({status:report.status,cases:observations.length}));
}
