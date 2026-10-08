/** Real Worker/WebCodecs/WASM conformance. Explicit local fixture; no product UI claim. */
import assert from 'node:assert/strict';
import { readFile,writeFile,mkdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
const [browserPath,pcmSdk,pcmPin,indexSdk,indexPin,output,fixture] = process.argv.slice(2);
assert(browserPath&&pcmSdk&&pcmPin&&indexSdk&&indexPin&&output&&fixture,'usage: check BROWSER PCM_SDK PCM_PIN INDEX_SDK INDEX_PIN NEW_OUTPUT FIXTURE');
const root=fileURLToPath(new URL('../../',import.meta.url));
for(const [tool,directory,pin] of [['video-pcm-sdk',pcmSdk,pcmPin],['media-index-sdk',indexSdk,indexPin]])
  execFileSync('python3',[`tools/${tool}/build.py`,'--verify',directory,'--sha256',pin],{cwd:root});
await mkdir(output,{recursive:false});
const sha=bytes=>createHash('sha256').update(bytes).digest('hex'),bytes=await readFile(fixture);
const stream=JSON.parse(execFileSync('ffprobe',['-v','error','-select_streams','a:0','-show_streams','-show_data','-of','json',fixture])).streams[0];
assert.equal(stream.codec_name,'aac');assert.equal(stream.time_base,`1/${stream.sample_rate}`);
assert(stream.channels>=1&&stream.channels<=2);
const description=Array.from(Buffer.from(stream.extradata.trim().split('\n').map(line=>line.split(': ')[1].split('  ')[0].replaceAll(' ','')).join(''),'hex'));
const raw=execFileSync('ffmpeg',['-v','error','-ignore_editlist','1','-i',fixture,'-map','0:a:0','-f','f32le','pipe:1'],{maxBuffer:16<<20});
const demux=execFileSync('ffmpeg',['-v','error','-i',fixture,'-map','0:a:0','-f','f32le','pipe:1'],{maxBuffer:16<<20});
// This three-second corpus has one rate-one media edit. FFmpeg applies the
// leading edit but emits the final codec block's padding; independently cap at
// its reported track duration, rather than the WASM result's claimed length.
const movieFrames=Number(stream.duration_ts);assert(Number.isSafeInteger(movieFrames)&&movieFrames>=96000&&movieFrames<=1_000_000);
const expected=demux.subarray(0,movieFrames*stream.channels*4);assert.equal(expected.length,movieFrames*stream.channels*4);
const files=new Map();
for(const [prefix,folder] of [['/pcm-sdk/',pcmSdk],['/index-sdk/',indexSdk]]) {
  const manifest=JSON.parse(await readFile(path.join(folder,'bundle-manifest.json')));
  for(const entry of manifest.files)files.set(prefix+entry.path,path.join(folder,entry.path));
}
const worker=await readFile(new URL('video-pcm-browser.worker.mjs',import.meta.url));
const server=createServer(async(req,res)=>{try {
  if(req.url==='/worker.mjs') {res.setHeader('Content-Type','text/javascript');res.end(worker);}
  else if(req.url==='/fixture')res.end(bytes);
  else if(files.has(req.url)) {res.setHeader('Content-Type',req.url.endsWith('.wasm')?'application/wasm':'text/javascript');res.end(await readFile(files.get(req.url)));}
  else {res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>PCM Worker verification</title>');}
} catch {res.statusCode=404;res.end();}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));let browser;
try {
  browser=await chromium.launch({executablePath:browserPath,headless:true});const version=browser.version();
  const page=await browser.newPage(),origin=`http://127.0.0.1:${server.address().port}`;
  await page.route('**/*',route=>new URL(route.request().url()).origin===origin?route.continue():route.abort());await page.goto(origin);
  const input={sha:sha(bytes),decoderProfile:sha(Buffer.from(`actual WebCodecs AAC ${version}; ${JSON.stringify(description)}`)),trackId:Number(stream.id),sampleRate:Number(stream.sample_rate),channels:stream.channels,description};
  const actual=await page.evaluate(async input=>{
    const pcmModule=await WebAssembly.compile(await(await fetch('/pcm-sdk/runtime/mo_video_pcm_wasm_bg.wasm')).arrayBuffer());
    const indexModule=await WebAssembly.compile(await(await fetch('/index-sdk/runtime/mo_media_index_wasm_bg.wasm')).arrayBuffer());
    input.blob=await(await fetch('/fixture')).blob();const worker=new Worker('/worker.mjs',{type:'module'});let timer;
    try {return await new Promise((resolve,reject)=>{timer=setTimeout(()=>reject(Error('PCM Worker deadline')),45000);
      worker.onmessage=e=>resolve(e.data);worker.onerror=e=>reject(Error(e.message));worker.postMessage({pcmModule,indexModule,input});});}
    finally {clearTimeout(timer);worker.terminate();}
  },input);
  await writeFile(path.join(output,'actual.json'),JSON.stringify(actual));assert(actual.ok,JSON.stringify(actual));
  const result=actual.result;
  const compare=(values,reference)=>{assert.equal(values.length*4,reference.length);let sum=0,peak=0;
    values.forEach((v,i)=>{const error=Math.abs(v-reference.readFloatLE(i*4));sum+=error*error;peak=Math.max(peak,error);});
    const rms=Math.sqrt(sum/values.length);assert(rms<0.00001&&peak<0.0001,`PCM mismatch ${rms}/${peak}`);return{values:values.length,rms,peak};};
  const rawComparison=compare(result.raw,raw),movieComparison=compare(result.values,expected);
  assert.equal(Number(result.info.frames),movieFrames);assert.equal(result.opened,0);assert.equal(result.peakOpen,1);assert.equal(result.chunks,Number(stream.nb_frames));assert(result.peakPending<=8);
  assert.equal(result.record.sampleZero.ticks,'0');assert.equal(Number(result.record.sampleFrames),raw.length/4/stream.channels);assert.equal(result.tiles,Math.ceil(raw.length/4/stream.channels/4096));
  assert.equal(result.rejects.length,9);assert.equal(result.faultChecks,25);assert(result.retainedPeak<=4096*stream.channels*4);
  await writeFile(path.join(output,'raw-oracle.f32le'),raw);await writeFile(path.join(output,'movie-oracle.f32le'),expected);
  const report={status:'passed',browser:version,platform:process.platform,arch:process.arch,fixtureSha256:input.sha,pcmManifestSha256:pcmPin,indexManifestSha256:indexPin,
    raw:rawComparison,movie:movieComparison,chunks:result.chunks,outputs:result.outputs,peakPendingPackets:result.peakPending,peakAudioData:result.peakOpen,remainingAudioData:result.opened,tiles:result.tiles,
    retainedCarryPeakBytes:result.retainedPeak,movieCacheBytes:result.cacheBytes,randomWindows:result.windows,repeatedAndGapFrames:result.repeatedFrames,rejectedFaultChecks:result.faultChecks,
    independentFfmpeg:execFileSync('ffmpeg',['-version'],{encoding:'utf8'}).split('\n')[0],
    scope:'Real raw browser AAC -> shared Rust PCM EOF tiles -> Rust movie audio. Explicit three-second AAC LC fixture. Not Product persistence, scheduling, playback, mixed-source export or cross-browser acceptance.'};
  await writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
} finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
