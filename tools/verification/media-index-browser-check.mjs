/** Real Worker/FileReaderSync/WASM versus native Rust and independent ffprobe. */
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {createServer} from 'node:http';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const root=fileURLToPath(new URL('../../',import.meta.url));
const [browserPath,sdkPath,pin,native,destination,...fixtures]=process.argv.slice(2);
assert(browserPath && sdkPath && pin && native && destination && fixtures.length, 'usage: check BROWSER SDK PIN NATIVE NEW_OUTPUT FIXTURES...');
execFileSync('python3',['tools/media-index-sdk/build.py','--verify',sdkPath,'--sha256',pin],{cwd:root});
await mkdir(destination,{recursive:false});
const sdk=path.resolve(sdkPath), inputs=[];
const fraction=(n,d=1)=>({numerator:String(n),denominator:String(d)});
for (const [id,file] of fixtures.entries()) {
  const bytes=await readFile(file);
  const reference=JSON.parse(execFileSync('ffprobe',['-v','error','-ignore_editlist','1','-show_packets','-show_streams','-of','json',file],{maxBuffer:8<<20}));
  const requests=[{method:'summary'}];
  const add=(method,body)=>{requests.push({method,body});return requests.length-1;};
  const tracks=[];
  for(const [index,stream] of reference.streams.entries()) {
    const packets=reference.packets.filter(p=>p.stream_index===stream.index);
    const track=add('query',{type:'track',track:index}),pages=[];
    for(let start=0;start<packets.length;start+=128) pages.push(add('query',{type:'samples',track:index,start,count:128}));
    const edits=add('query',{type:'edits',track:index,start:0,count:128});
    const movie=add('movie',{track:index,uneditedMediaEnd:fraction(0),start:0,count:128});
    const queries=[];
    let timing;
    if(stream.codec_type==='video') {
      timing=add('videoTiming',index);
      for(const [n,d] of [[-1,10],[0,1],[1,30000],[1,10],[1001,30000],[1,2],[3,5],[1,1],[13,10],[3,2],[8,5],[3,1],[4,1]])
        for(const side of ['before','after']) queries.push(add('selectVideo',{track:index,time:fraction(n,d),side}));
    }
    const avc = [];
    if(stream.codec_name==='h264' && packets.length) {
      add('avcSample',{track:index,sample:0}); // not scanned is an explicit state error
      for(let through=0;through<packets.length;through+=128)
        add('scanAvc',{track:index,through:Math.min(through+127,packets.length-1),count:128});
      for(let sample=0;sample<packets.length;sample++) avc.push(add('avcSample',{track:index,sample}));
      for(const sample of [packets.length-1,0,Math.floor(packets.length/2)]) {
        add('avcSample',{track:index,sample}); add('avcBootstrap',{track:index,sample});
      }
    }
    tracks.push({index,stream,packets,track,pages,edits,movie,timing,queries,avc});
  }
  add('query',{type:'samples',track:0,start:0,count:129});
  add('query',{type:'track',track:0.5});
  const requestFile=path.join(destination,`${id}-requests.json`);
  await writeFile(requestFile,JSON.stringify(requests));
  const expected=JSON.parse(execFileSync(path.resolve(native),[path.resolve(file),path.resolve(requestFile)],{maxBuffer:8<<20}));
  inputs.push({file,bytes,requests,expected,tracks});
}
const worker=await readFile(new URL('media-index.worker.mjs',import.meta.url));
const manifest=JSON.parse(await readFile(path.join(sdk,'bundle-manifest.json')));
const permitted=new Set(manifest.files.map(f=>f.path));
const server=createServer(async(req,res)=>{
  try {
    if(req.url==='/worker.mjs'){res.setHeader('Content-Type','text/javascript');res.end(worker);}
    else if(req.url?.startsWith('/sdk/')) {
      const name=req.url.slice(5); assert(permitted.has(name));
      res.setHeader('Content-Type',name.endsWith('.wasm')?'application/wasm':'text/javascript');res.end(await readFile(path.join(sdk,name)));
    } else {res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Media index verification</title>');}
  }catch {res.statusCode=404;res.end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let browser;
try {
  browser=await chromium.launch({executablePath:browserPath,headless:true});
  const page=await browser.newPage(),origin=`http://127.0.0.1:${server.address().port}`;
  await page.route('**/*',route=>new URL(route.request().url()).origin===origin?route.continue():route.abort());
  await page.goto(origin);
  const result=await page.evaluate(async inputs=>{
    const module=await WebAssembly.compile(await (await fetch('/sdk/runtime/mo_media_index_wasm_bg.wasm')).arrayBuffer());
    // Wait for evidence that the synchronous callback is actually running,
    // then terminate that exact Worker. The next Worker must still open inputs.
    const stalled=new Worker('/worker.mjs',{type:'module'});
    await new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{stalled.terminate();reject(Error('stalled read not reached'));},5000);
      stalled.onerror=e=>{clearTimeout(timer);stalled.terminate();reject(Error(e.message));};
      stalled.onmessage=e=>{if(e.data.enteredRead){clearTimeout(timer);stalled.terminate();resolve();}};
      stalled.postMessage({module,stall:true});
    });
    const worker=new Worker('/worker.mjs',{type:'module'});let timer;
    try {return await new Promise((resolve,reject)=>{
      timer=setTimeout(()=>reject(Error('index Worker deadline')),30000);
      worker.onmessage=e=>resolve(e.data);worker.onerror=e=>reject(Error(e.message));
      worker.postMessage({module,fixtures:inputs.map(x=>({blob:new Blob([new Uint8Array(x.bytes)]),requests:x.requests}))});
    });}finally{clearTimeout(timer);worker.terminate();}
  },inputs.map(x=>({bytes:Array.from(x.bytes),requests:x.requests})));
  await writeFile(path.join(destination,'result.json'),JSON.stringify(result,null,2));
  assert(result.ok,JSON.stringify(result));assert.equal(result.profile,'musteroffice.media-index/1-draft');assert.deepEqual(result.rejected,['io','io','io','io']);
  const reports=[];
  for(const [id,input]of inputs.entries()) {
    const actual=result.results[id];assert.deepEqual(actual.values,input.expected);
    assert(actual.maxRead<=65536);assert(actual.totalRead<input.bytes.length/2);
    let samplesCount=0;
    const decoded=JSON.parse(execFileSync('ffprobe',['-v','error','-ignore_editlist','1','-show_frames','-of','json',input.file],{maxBuffer:8<<20}));
    for(const t of input.tracks) {
      const samples=t.pages.flatMap(i=>actual.values[i].value.items);samplesCount+=samples.length;
      assert.equal(samples.length,t.packets.length);
      const shift=Math.min(0x7fffffff,Math.max(0,-Math.min(...samples.map(s=>Number(BigInt(s.pts)-BigInt(s.dts))))));
      for(const [i,s]of samples.entries()){
        assert.equal(s.pts,String(t.packets[i].pts));assert.equal(BigInt(s.dts)-BigInt(shift),BigInt(t.packets[i].dts));
        assert.equal(s.bytes.offset,t.packets[i].pos);assert.equal(s.bytes.length,t.packets[i].size);
      }
      if(t.avc.length) {
        const frames=new Map(decoded.frames.filter(f=>f.stream_index===t.stream.index).map(f=>[f.pkt_pos,f]));
        for(const [sample,request]of t.avc.entries()) {
          const meta=actual.values[request].value, frame=frames.get(t.packets[sample].pos);
          if(!frame) continue;
          assert(meta.picture,'decoded frame has no shared picture metadata');
          const seq=meta.picture.sequence;
          assert.equal(seq.visible[2],frame.width);assert.equal(seq.visible[3],frame.height);
          if(frame.chroma_location && frame.chroma_location!=='unspecified' && seq.frameMbsOnly)
            assert.equal(meta.chromaLocation?.toLowerCase(),frame.chroma_location);
          if(seq.color.signalPresent && frame.color_range && frame.color_range!=='unknown')
            assert.equal(seq.color.fullRange,frame.color_range==='pc');
          for(const [field,tag]of [['primaries','color_primaries'],['transfer','color_transfer'],['matrix','color_space']])
            if(seq.color.descriptionPresent && seq.color[field]===1) assert.equal(frame[tag],'bt709');
        }
      }
      if(t.timing!==undefined){
        const data={summary:actual.values[t.timing].value,track:actual.values[t.track].value.track,samples,edits:actual.values[t.edits].value.items,
          queries:t.queries.map(i=>({...input.requests[i].body,selection:actual.values[i].value}))};
        const dataFile=path.join(destination,`${id}-mapping.json`);await writeFile(dataFile,JSON.stringify(data));
        execFileSync('python3',[path.join(root,'tools/verification/media-index-mapping-check.py'),dataFile]);
      }
    }
    reports.push({file:path.basename(input.file),sha256:createHash('sha256').update(input.bytes).digest('hex'),queries:input.requests.length,
      samples:samplesCount,readBytes:actual.totalRead,maxReadRequest:actual.maxRead,readCalls:actual.calls});
  }
  const report={status:'passed',browser:browser.version(),platform:process.platform,architecture:process.arch,sdkManifestSha256:pin,stalledReadTerminated:true,inputs:reports,
    scope:'Real Worker/FileReaderSync and exact Rust WASM index/time/AVC metadata queries, native parity including seek bootstraps and independent FFprobe packets/decoded chroma/fraction edit oracle. No browser codec/movie renderer or product UI acceptance.'};
  await writeFile(path.join(destination,'report.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
}finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
