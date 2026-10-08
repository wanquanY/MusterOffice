/** Test host only: real raw WebCodecs -> shipped Rust PCM Worker component.
 * In-memory tiles/oracle arrays are bounded by this small fixture corpus. */
import { createPcmRuntime } from '/pcm-sdk/index.mjs';
import { createMediaIndexRuntime } from '/index-sdk/index.mjs';
const assert = (value, detail) => { if (!value) throw Error(detail); };
let faultChecks=0;
const expectError = (run, code) => { try { run(); } catch (e) {
  const error = JSON.parse(String(e)); assert(error.code === code, String(e)); faultChecks++;return error.code;
} throw Error('expected '+code); };
const ratio = (n,d=1) => ({numerator:String(n),denominator:String(d)});
const floats = bytes => { const view = new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength); return Array.from({length:bytes.length/4},(_,i)=>view.getFloat32(i*4,true)); };
async function run(runtime,indexRuntime,input) {
  const reader = new FileReaderSync();
  const index = indexRuntime.MediaIndex.open(BigInt(input.blob.size),(offset,count)=>new Uint8Array(reader.readAsArrayBuffer(input.blob.slice(Number(offset),Number(offset)+count))),'');
  let decode, decoder, movie;
  const tiles = new Map(), raw = [], blocks = [];
  let failure, outputs = 0, opened = 0, peakOpen = 0, retainedPeak = 0, reads = 0;
  const pending=[];let wake,peakPending=0;
  try {
    const summary = JSON.parse(index.summary());
    let ordinal, track;
    for(let at=0;at<summary.tracks;at++) { const t=JSON.parse(index.query(JSON.stringify({type:'track',track:at}))).track;
      if(t.id===input.trackId && String(t.handler)==='115,111,117,110') {ordinal=at;track=t;break;} }
    assert(track && track.descriptions===1,'raw audio identity/configuration');
    const query = body=>JSON.parse(index.query(JSON.stringify({track:ordinal,...body})));
    const first=query({type:'samples',start:0,count:1}).items[0];
    const options=JSON.stringify({resource:'audio-fixture',content:input.sha,stream:`track-${track.id}`,decoderProfile:input.decoderProfile,
      trackId:track.id,rawOrigin:ratio(first.pts,track.timescale),sampleRate:input.sampleRate,channels:input.channels});
    const clock={trackId:track.id,movieTimescale:summary.movieTimescale,mediaTimescale:track.timescale,declaredTrackDuration:track.declaredTrackDuration,edits:[]};
    for(let start=0;start<track.edits;start+=128) clock.edits.push(...query({type:'edits',start,count:128}).items);
    decode=runtime.PcmDecode.open(options);
    const write=(reference,bytes)=>{ const value=JSON.parse(reference); assert(!tiles.has(value.index),'duplicate tile'); tiles.set(value.index,{reference,bytes});return true; };
    decoder=new AudioDecoder({error:e=>{failure=String(e);wake?.();}, output:data=>{
      opened++;peakOpen=Math.max(peakOpen,opened);
      try {
        assert(data.numberOfFrames<=8192 && data.sampleRate===input.sampleRate && data.numberOfChannels===input.channels,'actual output format');
        const values=new Float32Array(data.numberOfFrames*data.numberOfChannels);data.copyTo(values,{planeIndex:0,format:'f32'});
        const bytes=new Uint8Array(values.length*4),view=new DataView(bytes.buffer);
        values.forEach((v,i)=>view.setFloat32(i*4,v,true));
        assert(Number.isSafeInteger(data.timestamp),'output timestamp');
        decode.push(BigInt(data.timestamp),data.numberOfFrames,data.sampleRate,data.numberOfChannels,bytes,write);
        retainedPeak=Math.max(retainedPeak,decode.retainedBytes());
        raw.push(...values); blocks.push({timestamp:data.timestamp,frames:data.numberOfFrames});outputs++;
      } catch(e) { failure=String(e); } finally {
        while(pending.length && pending[0]<data.timestamp)pending.shift();
        data.close();opened--;wake?.();
      }
    }});
    const config={codec:'mp4a.40.2',sampleRate:input.sampleRate,numberOfChannels:input.channels,description:new Uint8Array(input.description)};
    assert((await AudioDecoder.isConfigSupported(config)).supported,'AAC codec unavailable');decoder.configure(config);
    let page=[],start=-1,chunks=0;
    for(let at=0;at<track.samples;at++) {
      if(at>=start+page.length) {start=at;page=query({type:'samples',start,count:128}).items;}
      const packet=page[at-start];assert(packet.description===0&&packet.pts===packet.dts,'raw AAC packet');
      const offset=Number(BigInt(packet.bytes.offset)),length=Number(BigInt(packet.bytes.length));
      assert(length>0&&length<=1<<20&&offset+length<=input.blob.size,'bounded packet');
      const bytes=await input.blob.slice(offset,offset+length).arrayBuffer();
      const timestamp=Number(decode.packetTimestamp(BigInt(packet.pts),track.timescale));assert(Number.isSafeInteger(timestamp),'input timestamp');
      pending.push(timestamp);peakPending=Math.max(peakPending,pending.length);
      decoder.decode(new EncodedAudioChunk({type:'key',timestamp,data:bytes}));chunks++;
      // Keep the newest packet charged until a later output arrives. No
      // intermediate flush: real AAC overlap state differs across flushes.
      while(pending.length>=8&&!failure)await new Promise(resolve=>{wake=resolve;});
      wake=undefined;assert(!failure,failure);
    }
    await decoder.flush();assert(!failure,failure);decoder.close();decoder=undefined;
    const record=decode.finish(write);for(const tile of tiles.values()) tile.reference=decode.seal(tile.reference);
    const lookup=(source,index)=>{assert(source===JSON.parse(record).source,'raw digest callback');return tiles.get(index)?.reference;};
    const read=(reference,max)=>{ reads++;const tile=tiles.get(JSON.parse(reference).index);assert(tile.bytes.length<=max,'tile read bound');return tile.bytes; };
    const open=clockValue=>runtime.MoviePcm.open(options,record,JSON.stringify(clockValue),lookup,read);
    movie=open(clock);const info=JSON.parse(movie.info()),values=[];
    assert(Number(info.frames)<=1_000_000,'fixture output cap');
    for(let first=0;first<Number(info.frames);first+=997) values.push(...floats(movie.read(BigInt(first),Math.min(997,Number(info.frames)-first))));
    const windows=[];
    for(const [first,count] of [[Number(info.frames)-1,1],[1000,127],[60000,311],[0,17],[80000,8192]]) {
      const actual=floats(movie.read(BigInt(first),count));assert(actual.every((v,i)=>Object.is(v,values[first*input.channels+i])),'random window mismatch');windows.push({first,count});
    }
    const cacheBytes=movie.retainedBytes();movie.clear();assert(movie.retainedBytes()===0,'clear retained cache/filter');
    const saved=tiles.get(0);tiles.delete(0);expectError(()=>movie.read(0n,1),'pending_source');tiles.set(0,saved);
    saved.bytes[0]^=1;expectError(()=>movie.read(0n,1),'invalid_request');saved.bytes[0]^=1;
    assert(floats(movie.read(0n,1))[0]===values[0],'retry after valid host tile restore');
    const noRead=reads;
    expectError(()=>movie.read(-1n,1),'invalid_request');
    expectError(()=>movie.read(BigInt(info.frames),1),'invalid_request');
    expectError(()=>movie.read(0n,8193),'resource_limit');assert(reads===noRead,'invalid read reached storage');
    movie.free();movie=undefined;
    // Synthetic clock vectors test the WASM boundary over real decoded PCM.
    // The production host must still bind its clock to its actual source.
    const mediaStart=clock.edits.find(e=>e.mediaTime!==null).mediaTime;
    const repeated={...clock,movieTimescale:1000,declaredTrackDuration:'1750',edits:[
      {movieStart:'0',movieDuration:'500',mediaTime:null,rate1616:65536},
      {movieStart:'500',movieDuration:'500',mediaTime:mediaStart,rate1616:65536},
      {movieStart:'1000',movieDuration:'250',mediaTime:null,rate1616:65536},
      {movieStart:'1250',movieDuration:'500',mediaTime:mediaStart,rate1616:65536},
    ]};
    const repeatedMovie=open(repeated);let repeatedFrames=0;
    try {const end=Number(JSON.parse(repeatedMovie.info()).frames),rate=input.sampleRate;
      for(let at=0;at<end;at+=997) {const n=Math.min(997,end-at),actual=floats(repeatedMovie.read(BigInt(at),n));
        for(let i=0;i<n;i++) {const t=(at+i)/rate,source=t>=.5&&t<1?at+i-rate/2:t>=1.25?at+i-rate*1.25:null;
          for(let c=0;c<input.channels;c++) assert(Object.is(actual[i*input.channels+c],source===null?0:values[source*input.channels+c]),'empty/repeated edit mismatch');}
        repeatedFrames+=n;}
    } finally {repeatedMovie.free();}
    const rejects=[];
    rejects.push(expectError(()=>runtime.MoviePcm.open(options,record,JSON.stringify({...clock,trackId:track.id+1}),lookup,read),'invalid_request'));
    rejects.push(expectError(()=>runtime.PcmDecode.open(options.slice(0,-1)+',"channels":1}'),'invalid_request'));
    for(const kind of ['gap','timestamp','nan','write','promise','short','large']) {
      const bad=runtime.PcmDecode.open(options), count=4096, pcm=new Uint8Array(count*input.channels*4);
      try {
        let time=0n;
        if(kind==='gap') {bad.push(0n,count,input.sampleRate,input.channels,pcm,()=>true);time=BigInt(Math.round((count+1)*1e6/input.sampleRate));}
        if(kind==='timestamp')time=2n;
        if(kind==='nan')new DataView(pcm.buffer).setFloat32(0,NaN,true);
        const bytes=kind==='short'?pcm.subarray(1):pcm;
        rejects.push(expectError(()=>bad.push(time,kind==='large'?8193:count,input.sampleRate,input.channels,bytes,
          kind==='write'?()=>{throw Error('host failure');}:kind==='promise'?()=>Promise.resolve(true):()=>true),'invalid_request'));
        expectError(()=>bad.finish(()=>true),'invalid_request');
      } finally {bad.free();}
    }
    const failedTail=runtime.PcmDecode.open(options);
    try {failedTail.push(0n,1,input.sampleRate,input.channels,new Uint8Array(input.channels*4),()=>true);
      expectError(()=>failedTail.finish(()=>false),'invalid_request');expectError(()=>failedTail.finish(()=>true),'invalid_request');
    } finally {failedTail.free();}
    const noAudio=runtime.PcmDecode.open(options);
    try {expectError(()=>noAudio.finish(()=>true),'invalid_request');} finally {noAudio.free();}
    // Simulate a decoder that discards its first complete priming block. Feed
    // actual decoded bytes with ORIGINAL output timestamps and require the same
    // movie samples. This is distinct from hiding a missing audible block.
    const trimmed=runtime.PcmDecode.open(options),trimTiles=new Map();let position=blocks[0].frames*input.channels;
    const trimWrite=(reference,bytes)=>{trimTiles.set(JSON.parse(reference).index,{reference,bytes});return true;};
    try {
      for(const block of blocks.slice(1)) {const data=new Uint8Array(block.frames*input.channels*4),view=new DataView(data.buffer);
        for(let i=0;i<data.length/4;i++)view.setFloat32(i*4,raw[position++],true);
        trimmed.push(BigInt(block.timestamp),block.frames,input.sampleRate,input.channels,data,trimWrite);}
      const trimRecord=trimmed.finish(trimWrite);for(const tile of trimTiles.values())tile.reference=trimmed.seal(tile.reference);
      const t=runtime.MoviePcm.open(options,trimRecord,JSON.stringify(clock),(_s,i)=>trimTiles.get(i)?.reference,r=>trimTiles.get(JSON.parse(r).index).bytes);
      try { for(const {first,count} of windows) assert(floats(t.read(BigInt(first),count)).every((v,i)=>Object.is(v,values[first*input.channels+i])),'double priming trim'); }
      finally {t.free();}
    } finally {trimmed.free();}
    const absent=runtime.PcmDecode.open(options);
    try {absent.push(BigInt(blocks[2].timestamp),1,input.sampleRate,input.channels,new Uint8Array(input.channels*4),()=>true);
      const shortRecord=absent.finish(()=>true);expectError(()=>runtime.MoviePcm.open(options,shortRecord,JSON.stringify(clock),lookup,read),'invalid_request');
    } finally {absent.free();}
    return {record:JSON.parse(record),info,raw,values,blocks,clock,chunks,outputs,peakPending,peakOpen,opened,tiles:tiles.size,reads,retainedPeak,cacheBytes,windows,repeatedFrames,faultChecks,rejects};
  } finally {if(decoder?.state!=='closed')decoder?.close();movie?.free();decode?.free();index.free();}
}
self.onmessage=async e=>{
  try {const runtime=await createPcmRuntime(e.data.pcmModule),index=await createMediaIndexRuntime(e.data.indexModule);
    const result=await run(runtime,index,e.data.input);self.postMessage({ok:true,result});
  } catch(e) {self.postMessage({ok:false,error:String(e),stack:e?.stack});}
};
