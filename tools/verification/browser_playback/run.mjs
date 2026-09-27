/** Actual browser integration. No display engine, private data or product UI. */
import {openBrowserWorker} from './sdk/examples/browser-worker.mjs';
import {lifecycle} from './lifecycle.mjs';
import {check,equal,rejects,sha} from './assert.mjs';
async function bytes(record) {
  const response=await fetch(record.url);check(response.ok,'Input response');
  const bytes=new Uint8Array(await response.arrayBuffer());
  check(bytes.length===record.byteLength && await sha(bytes)===record.sha256,'Input identity');
  return bytes;
}
async function post(path, body, type) {
  const response=await fetch(path,{method:'POST',headers:{'Content-Type':type},body});
  check(response.ok,'Report rejected: '+await response.text());
}

async function run() {
  const data=await (await fetch('./corpus.json')).json();
  const modules={};
  for (const [key,record] of Object.entries(data.modules)) modules[key]=await WebAssembly.compile(await bytes(record));
  const host=await openBrowserWorker(modules);
  const observations=[];let cooperativeCancellations=0;
  try {
    for (const owner of data.owners) {
      const command={operation:'prepare',kind:owner.kind,request:owner.request};const transfer=[];
      if (owner.kind==='source') for (const key of ['source','fonts']) {
        command[key]=await bytes(owner[key]);transfer.push(command[key].buffer);
      }
      const prepared=await host.call(command,transfer);
      if (owner.kind==='source') check(command.source.byteLength===0 && command.fonts.byteLength===0
        && prepared.inputsDetached===true,'Source/font ownership');
      if (data.stepped) for(const after of [0,1,4]) {
        check((await host.call({operation:'beginSample',...owner.frames[0].sample})).begun,'Frame prepared');
        await rejects(()=>host.call({operation:'timing'}),e=>e.code==='BUSY');
        for(let i=0;i<after;i++)await host.call({operation:'stepSample',workUnits:1});
        check((await host.call({operation:'cancelSample'})).cancelled,'Cancellation acknowledged');
        await rejects(()=>host.call({operation:'takeSample'}),e=>e.message==='No active sampled frame');
        check(!host.closed,'Healthy cancellation preserves worker');
        cooperativeCancellations++;
      }
      const frames=[];
      for (const {index,name,sample} of owner.frames) {
        let frame;
        if(data.stepped) {
          check((await host.call({operation:'beginSample',...sample})).begun,'Frame prepared');
          while(!(await host.call({operation:'stepSample',workUnits:7})).complete) { /* Explicit host scheduling. */ }
          frame=await host.call({operation:'takeSample'});
        } else frame=await host.call({operation:'sample',...sample});
        check(frame.pixels instanceof Uint8Array && frame.pixels.length>0,'Frame bytes');
        if(owner.kind==='source') check(frame.info.page.textWork.componentCalls===0
          && frame.info.page.textWork.fontUploadBytes===0 && frame.info.page.gatherCopyBytes===0,'No repeated font/source work');
        const pixelSha256=await sha(frame.pixels);
        await post('./frames/'+index,frame.pixels,'application/octet-stream');
        frames.push({index,name,info:frame.info,pixelSha256,byteLength:frame.pixels.length});
      }
      check(equal((await host.call({operation:'timing'})).binding,owner.request.binding),'Timing binding');
      await rejects(()=>host.call({operation:'advance',generation:owner.request.binding.generation}),
        e=>e.name==='PlaybackComputationError' && e.diagnostic.code==='GENERATION_NOT_INCREASING');
      check(!host.closed,'Semantic error keeps Worker');
      const advanced=await host.call({operation:'advance',generation:'9007199254740993'});
      check(advanced.planId===prepared.info.planId && advanced.binding.generation==='9007199254740993','Exact generation');
      check((await host.call({operation:'dispose'})).disposed===true,'Disposed');
      observations.push({kind:owner.kind,frames,inputsDetached:prepared.inputsDetached});
    }
  } finally {host.close();}
  const faults=await lifecycle(modules,data.owners[0]);
  return {status:'passed',userAgent:navigator.userAgent,crossOriginIsolated,stepped:data.stepped,cooperativeCancellations,
    frameCount:observations.reduce((n,o)=>n+o.frames.length,0),observations,lifecycle:faults,
    scope:'Actual headless browser module Worker with fixed Rust/Skia/HarfBuzz and owned fixtures; no product UI or performance acceptance.'};
}

try {await post('./report',JSON.stringify(await run()),'application/json');document.body.textContent='passed';}
catch(error) {
  document.body.textContent=error.stack;
  await post('./report',JSON.stringify({status:'failed',error:String(error.stack)}),'application/json');
}
