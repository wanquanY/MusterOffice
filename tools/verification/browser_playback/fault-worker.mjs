/** Test only: faults injected at the actual Rust -> raster callback boundary. */
import {createPlaybackRuntime} from './sdk/index.mjs';
import {failure} from './sdk/examples/dispatch.mjs';
globalThis.fetch=()=>{throw Error('Implicit fetch denied in fault Worker');};
let runtime;
onmessage=async({data:{id,command:q}})=>{
  try {
    if(q.operation==='initialize') {
      runtime=await createPlaybackRuntime(q.modules);
      postMessage({id,ok:true,value:{ready:true}});return;
    }
    if(q.operation==='wrong-id') {postMessage({id:id+1,ok:true,value:null});return;}
    if(q.operation==='malformed') {postMessage({id,ok:false,error:null});return;}
    if(q.operation==='throw') throw Error('Deliberate recoverable host failure');
    if(q.operation==='uncaught') {setTimeout(()=>{throw Error('Deliberate uncaught worker failure');},0);return;}
    const owner=runtime.playback.prepareAuthor(q.request);
    try {
      if(q.operation==='fatal')runtime.raster.invalidate();
      const raster=q.operation==='block'?{
        raster(...args) {
          const counter=q.counter?new Int32Array(q.counter):null;
          q.port.postMessage({entered:true});
          const end=performance.now()+q.holdMs;
          while(performance.now()<end) {if(counter)Atomics.add(counter,0,1);}
          q.port.postMessage({late:true});
          return runtime.raster.raster(...args);
        }, invalidate(){},
      }:runtime.raster;
      const frame=owner.sample(q.at,raster);
      postMessage({id,ok:true,value:frame},[frame.pixels.buffer]);
    } finally {owner.close();}
  } catch(error) {postMessage(failure(id,error,!runtime||runtime.raster.invalid||runtime.shaping.invalid));}
};
