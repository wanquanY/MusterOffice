/** Actual production media entry + independent PCM/index components. Test-only
 * instrumentation counts real browser objects and preserves every codec call. */
import { BrowserMediaInput, BrowserMediaBudget, BrowserVideoBudget } from '/media-sdk/index.mjs';
import { createPcmRuntime } from '/pcm-sdk/index.mjs';
import { createMediaIndexRuntime } from '/index-sdk/index.mjs';
const assert=(value,detail)=>{if(!value)throw Error(detail);};
const floats=bytes=>{const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);return Array.from({length:bytes.length/4},(_,i)=>view.getFloat32(i*4,true));};
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const stats={created:0,decoders:0,configured:0,chunks:0,flushes:0,audioData:0,peakAudioData:0,core:0,carryPeak:0,finish:0};
const NativeAudioDecoder=AudioDecoder;
globalThis.AudioDecoder=new Proxy(NativeAudioDecoder,{construct(Target,args){
  const config=args[0],output=config.output;stats.created++;stats.decoders++;
  const decoder=Reflect.construct(Target,[{...config,output(data){
    stats.audioData++;stats.peakAudioData=Math.max(stats.peakAudioData,stats.audioData);let closed=false;const close=data.close.bind(data);
    data.close=()=>{if(!closed){closed=true;stats.audioData--;}close();};output(data);
  }}]);
  let closed=false;const close=decoder.close.bind(decoder),configure=decoder.configure.bind(decoder),decode=decoder.decode.bind(decoder),flush=decoder.flush.bind(decoder);
  decoder.close=()=>{if(!closed){closed=true;stats.decoders--;}close();};
  decoder.configure=c=>{stats.configured++;return configure(c);};decoder.decode=c=>{stats.chunks++;return decode(c);};
  decoder.flush=()=>{stats.flushes++;return flush();};return decoder;
}});
let infoHook;
function measured(runtime) {return {...runtime,MoviePcm:{open(...args){
  const movie=runtime.MoviePcm.open(...args),hook=infoHook;infoHook=undefined;if(!hook)return movie;
  return {info(){const value=movie.info();hook();return value;},free:movie.free.bind(movie),read:movie.read.bind(movie),clear:movie.clear.bind(movie),retainedBytes:movie.retainedBytes.bind(movie)};
}},PcmDecode:{open(options){const core=runtime.PcmDecode.open(options);stats.core++;
  let closed=false;return {packetTimestamp:core.packetTimestamp.bind(core),seal:core.seal.bind(core),retainedBytes:core.retainedBytes.bind(core),
    push(...args){core.push(...args);stats.carryPeak=Math.max(stats.carryPeak,core.retainedBytes());},
    finish(...args){const value=core.finish(...args);stats.finish++;return value;},
    free(){if(!closed){closed=true;stats.core--;core.free();}},
  };
}}};}
async function rejected(promise,code){try{await promise;}catch(e){assert(e?.code===code,`expected ${code}, got ${String(e)} ${e?.stack}`);return code;}throw Error('missing failure '+code);}
async function retired(pool){for(let i=0;i<100&&pool.usedBytes;i++)await delay(5);assert(pool.usedBytes===0,`unretired ${pool.usedBytes}`);}
function tileStore(){const tiles=new Map();return {tiles,write(reference,bytes){const ref=JSON.parse(reference);assert(!tiles.has(ref.index),'duplicate tile');tiles.set(ref.index,{reference,bytes});return true;}};}
function lookups(store,record){return [(source,index)=>{assert(source===record.source,'source digest');return store.tiles.get(index)?.reference;},
  (reference,max)=>{const tile=store.tiles.get(JSON.parse(reference).index);assert(tile.bytes.length<=max,'read bound');return tile.bytes;}];}
async function main(runtime,index,input){
  assert(BrowserMediaBudget===BrowserVideoBudget,'two separate budgets');
  const pool=new BrowserMediaBudget(64<<20),origin={resource:'materializer',content:input.sha,stream:`track-${input.trackId}`,decoderProfile:input.decoderProfile};
  const open=(blob=input.blob,limits,videoBudget)=>BrowserMediaInput.open(blob,{mediaIndexRuntime:index,mediaBudget:pool,...(limits?{limits}:{}),...(videoBudget?{videoBudget}:{})});
  const media=await open(),store=tileStore();let nested;
  const before={...stats};
  const value=await media.materializeMovieAudio(input.trackId,runtime,{...origin,trackId:999,sampleRate:1},(ref,bytes)=>{
    if(!nested)nested=rejected(media.materializeMovieAudio(input.trackId,runtime,origin,()=>true),'busy');
    return store.write(ref,bytes);
  });
  await nested;assert(value.decodedPackets===input.packets,'actual packet count');assert(value.writtenTiles===store.tiles.size,'tile count');
  assert(stats.configured-before.configured===1&&stats.flushes-before.flushes===1,'not one continuous decoder and EOF flush');
  const primary={configured:stats.configured-before.configured,chunks:stats.chunks-before.chunks,flushes:stats.flushes-before.flushes,
    decodedBlocks:value.decodedBlocks,carryPeak:stats.carryPeak,peakAudioData:stats.peakAudioData,completedLeaseBytes:pool.usedBytes,budgetPeak:pool.peakBytes};
  media.close();assert(pool.usedBytes>0&&stats.core===1,'owned receipt did not survive source close');
  const record=JSON.parse(value.record),info=JSON.parse(value.movieInfo);
  assert(JSON.parse(value.options).trackId===input.trackId&&JSON.parse(value.options).sampleRate===input.sampleRate,'untyped origin overrode source binding');
  for(const tile of store.tiles.values())tile.reference=value.seal(tile.reference);
  const raw=[...store.tiles.values()].flatMap(tile=>floats(tile.bytes));
  // Owned receipt admission competes with a new source before codec configure.
  const contender=await open(),room=pool.capacityBytes-pool.usedBytes-1,reserve=pool.reserve(room),configured=stats.configured;
  await rejected(contender.materializeMovieAudio(input.trackId,runtime,origin,()=>true),'resource_limit');
  assert(stats.configured===configured,'denial after configure');reserve();contender.close();
  const movie=runtime.MoviePcm.open(value.options,value.record,value.clock,...lookups(store,record)),values=[];let reads=0;
  try {for(let at=0;at<Number(info.frames);at+=997){values.push(...floats(movie.read(BigInt(at),Math.min(997,Number(info.frames)-at))));reads++;}
    for(const [first,count] of [[Number(info.frames)-1,1],[1000,127],[60000,311],[0,17],[80000,8192]])
      assert(floats(movie.read(BigInt(first),count)).every((v,i)=>Object.is(v,values[first*input.channels+i])),'random PCM mismatch');
  }finally{movie.free();}
  value.close();value.close();await retired(pool);await rejected(Promise.resolve().then(()=>value.seal(store.tiles.get(0).reference)),'closed');
  const cases={nested:'busy',contender:'resource_limit',staleReceipt:'closed'};
  // Same input retries after a host write failure; no unpublished prefix may
  // acquire a completion, and the exact host exception must be preserved.
  const retry=await open(),sentinel=Error('host write failure'),done=stats.finish;
  try{await retry.materializeMovieAudio(input.trackId,runtime,origin,()=>{throw sentinel;});throw Error('write succeeded');}
  catch(e){assert(e===sentinel,'host error identity lost');}
  await retired(pool);assert(stats.finish===done,'failed write completed');
  const retryStore=tileStore(),retried=await retry.materializeMovieAudio(input.trackId,runtime,origin,retryStore.write);
  assert(retried.record===value.record,'retry changed canonical identity');retried.close();retry.close();await retired(pool);cases.retry='same_record';
  const promised=await open();await rejected(promised.materializeMovieAudio(input.trackId,runtime,origin,()=>Promise.resolve(true)),'invalid_request');
  promised.close();await retired(pool);cases.asyncWrite='invalid_request';
  const stop=new AbortController(),cancelled=await open(),finished=stats.finish;
  await rejected(cancelled.materializeMovieAudio(input.trackId,runtime,origin,()=>{stop.abort();return true;},stop.signal),'cancelled');
  await retired(pool);assert(stats.finish===finished,'cancelled callback completed');cases.callbackCancel='cancelled';
  const lateStop=new AbortController(),late=await open();infoHook=()=>queueMicrotask(()=>lateStop.abort());
  await rejected(late.materializeMovieAudio(input.trackId,runtime,origin,()=>true,lateStop.signal),'cancelled');
  await retired(pool);assert(stats.core===0,'late completion leaked an owned handle');cases.lateCancel='cancelled';
  const denied=await open(input.blob,{maxEncodedPacketBytes:1}),created=stats.configured;
  await rejected(denied.materializeMovieAudio(input.trackId,runtime,origin,()=>true),'resource_limit');
  assert(stats.configured===created,'oversized first packet configured');denied.close();await retired(pool);cases.packet='resource_limit';
  // A Blob read deliberately remains live after cancellation/deadline. The
  // operation rejects promptly, but its byte admission must not be recycled.
  const reader=new FileReaderSync(),source=index.MediaIndex.open(BigInt(input.blob.size),(o,n)=>new Uint8Array(reader.readAsArrayBuffer(input.blob.slice(Number(o),Number(o)+n))),'');
  let ordinal;const summary=JSON.parse(source.summary());
  for(let i=0;i<summary.tracks;i++)if(JSON.parse(source.query(JSON.stringify({type:'track',track:i}))).track.id===input.trackId)ordinal=i;
  const packet=JSON.parse(source.query(JSON.stringify({type:'samples',track:ordinal,start:0,count:1}))).items[0];source.free();
  for(const mode of ['close','deadline']){
    let entered,release;const entry=new Promise(r=>entered=r),held=new Promise(r=>release=r);
    const offset=Number(packet.bytes.offset),length=Number(packet.bytes.length);
    class DelayedBlob extends Blob {slice(start,end,type){const slice=super.slice(start,end,type);
      if(start===offset&&end===offset+length)return new class extends Blob{async arrayBuffer(){entered();await held;return super.arrayBuffer();}}([slice]);return slice;}}
    const blob=new DelayedBlob([input.blob]),stalled=await open(blob,{operationDeadlineMs:500}),initial=stats.configured;
    const operation=stalled.materializeMovieAudio(input.trackId,runtime,origin,()=>true);await entry;
    if(mode==='close')stalled.close();
    await rejected(operation,mode==='close'?'closed':'deadline');
    assert(stats.configured===initial&&pool.usedBytes>0,'pending I/O lost its lease or configured early');
    const charged=pool.usedBytes;release();await retired(pool);cases[mode]={charged,after:pool.usedBytes};
  }
  const after=await open(),fresh=tileStore(),freshValue=await after.materializeMovieAudio(input.trackId,runtime,origin,fresh.write);freshValue.close();after.close();await retired(pool);
  assert(stats.audioData===0&&stats.decoders===0&&stats.core===0,'live resources after all operations');
  return {raw,values,record,info,primary,cases,stats,pool:{capacity:pool.capacityBytes,peak:pool.peakBytes,used:pool.usedBytes},tiles:store.tiles.size,reads};
}
self.onmessage=async e=>{try{
  const pcm=measured(await createPcmRuntime(e.data.pcmModule)),index=await createMediaIndexRuntime(e.data.indexModule);
  self.postMessage({ok:true,result:await main(pcm,index,e.data.input)});
}catch(e){self.postMessage({ok:false,error:String(e),stack:e?.stack,stats});}};
