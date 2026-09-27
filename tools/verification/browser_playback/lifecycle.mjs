import {openBrowserWorker} from './sdk/examples/browser-worker.mjs';
import {check,rejects,sha} from './assert.mjs';
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));

export async function lifecycle(modules,owner) {
  const workerUrl=new URL('./fault-worker.mjs',import.meta.url);
  const command={request:owner.request,at:owner.frames[0].sample.at};
  const events=[];
  // Cancel/timeout after real WASM has entered raster, not merely during startup.
  for(const [mode,holdMs] of [['abort',500],['abort',6000],['deadline',6000],['close',6000]]) {
    const host=await openBrowserWorker(modules,{workerUrl});
    const channel=new MessageChannel(),signal=new AbortController();let markerTimer;
    const received=[];
    const entered=new Promise((resolve,reject)=>{
      markerTimer=setTimeout(()=>reject(Error('Raster callback not reached')),3000);
      channel.port1.onmessage=({data})=>{received.push(data);if(data.entered){clearTimeout(markerTimer);resolve();}};
    });
    const counter=crossOriginIsolated?new SharedArrayBuffer(4):null;
    const outcome=host.call({operation:'block',...command,port:channel.port2,counter,holdMs},[channel.port2],
      mode==='deadline'?200:3000,signal.signal).then(value=>({value}),error=>({error}));
    try {
      await entered;
      await rejects(()=>host.call({operation:'timing'}),e=>/busy/.test(e.message));
      const requested=performance.now();
      if(mode==='abort')signal.abort();
      if(mode==='close')host.close();
      const {error}=await outcome;check(error,'Expected terminal failure');
      const rejectionMs=performance.now()-requested;
      check(mode==='abort'?error.name==='AbortError':error.message.includes(mode==='deadline'?'deadline':'stopped'),'Terminal reason');
      check(host.closed,'Host closed before rejection');
      await rejects(()=>host.call({operation:'timing'}),e=>/closed/.test(e.message));
      // A browser may first request orderly shutdown and force it later. This
      // observation must not become a fake terminate-completion acknowledgement.
      let executionObservation=null;
      if(counter) {
        let previous=Atomics.load(new Int32Array(counter),0),stable=0,lastChangeMs=0;
        const samples=[];
        while(performance.now()-requested<5000 && stable<4) {
          await wait(100);const count=Atomics.load(new Int32Array(counter),0),elapsedMs=performance.now()-requested;
          samples.push({elapsedMs,count});
          if(count===previous)stable++;else{stable=0;lastChangeMs=elapsedMs;}
          previous=count;
        }
        check(previous>0 && stable===4,'Execution counter failed to stop');
        executionObservation={lastChangeMs,stableAtMs:performance.now()-requested,samples};
      } else await wait(holdMs+300);
      const finishedCallback=received.some(e=>e.late);
      if(holdMs===6000)check(!finishedCallback,'Long callback survived worker termination');
      check(host.closed,'Closed host cannot expose late results');
      events.push({mode,holdMs,enteredActualRaster:true,closedBeforeReject:true,rejectionMs,
        finishedCallbackAfterCancellation:finishedCallback,executionObservation,
        noTerminationAcknowledgement:true});
    } finally {clearTimeout(markerTimer);host.close();channel.port1.close();channel.port2.close();}
  }
  for(const mode of ['wrong-id','malformed','uncaught','fatal','clone']) {
    const host=await openBrowserWorker(modules,{workerUrl});
    try {
      await rejects(()=>host.call({operation:mode,...command,...(mode==='clone'?{bad:()=>{}}:{})}),
        e=>mode==='fatal'?e.name==='PlaybackComputationError':Boolean(e.message));
      check(host.closed,'Fault must close host: '+mode);
      events.push({mode,closed:true});
    } finally {host.close();}
  }
  const host=await openBrowserWorker(modules,{workerUrl});
  try {
    await rejects(()=>host.call({operation:'throw'}),e=>/recoverable/.test(e.message));
    check(!host.closed,'Normal host error is not fatal');
    const signal=new AbortController();signal.abort();
    await rejects(()=>host.call({operation:'sample',...command},[],30000,signal.signal),e=>e.name==='AbortError');
    check(!host.closed,'Pre-aborted call leaves healthy worker');
    const active=new AbortController();
    const frame=await host.call({operation:'sample',...command},[],30000,active.signal);
    active.abort();check(!host.closed,'Settled call removes abort handler');
    const again=await host.call({operation:'sample',...command});
    check(await sha(frame.pixels)===await sha(again.pixels),'Healthy new owner after regular errors');
    events.push({mode:'recoverable-and-signal-cleanup',passed:true});
  } finally {host.close();}
  await rejects(()=>openBrowserWorker(modules,{workerUrl:new URL('./missing-worker.mjs',import.meta.url)}),e=>Boolean(e.message));
  events.push({mode:'script-load-failure',rejected:true});
  return events;
}
