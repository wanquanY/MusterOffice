import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import {fixtures,base,solid} from './path-raster-fixtures.mjs';
const root='.codex-work/path-raster',require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
fs.mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex'),componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm'),module=new WebAssembly.Module(componentBytes);
const component=await RasterComponent.create(factory,module),cases=[],equivalents=[],responses=new Map();let frame,componentCalls=0;
const observed={raster(words){componentCalls++;frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function packet(json){const bytes=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(bytes.length);return Buffer.concat([h,bytes]);}
function parseReply(bytes){assert(bytes.length>=8);const m=bytes.readUInt32LE(),p=bytes.readUInt32LE(4);assert(bytes.length>=8+m+p);return {metadata:bytes.subarray(8,8+m).toString(),pixels:bytes.subarray(8+m,8+m+p),consumed:8+m+p};}
function native(json){const r=spawnSync('target/release/mo-raster-worker',[],{input:packet(json),timeout:30000,maxBuffer:80*1024*1024,env:{}});assert.equal(r.status,0,r.stderr?.toString());assert(!r.error);const parsed=parseReply(r.stdout);assert.equal(parsed.consumed,r.stdout.length);return parsed;}
function fromWasm(json,backend=observed){const r=wasm.render_paths(json,backend),metadata=r.metadata,pixels=Buffer.from(r.take_pixels());return {metadata,pixels};}
function probe(bytes){const h=Buffer.alloc(4);h.writeUInt32LE(bytes.length/4);const p=spawnSync('.codex-work/skia/mo-skia-probe',[],{input:Buffer.concat([h,bytes]),timeout:30000,maxBuffer:80*1024*1024});assert.equal(p.status,0,p.stderr?.toString());assert.equal(p.stdout.readUInt32LE(),0);assert.equal(p.stdout.readUInt32LE(4),p.stdout.length-8);return p.stdout.subarray(8);}
for(const c of fixtures()){
 const json=typeof c.q==='string'?c.q:JSON.stringify(c.q),n=native(json);frame=null;const before=componentCalls,w=fromWasm(json);
 assert.equal(w.metadata,n.metadata,c.name);assert.deepEqual(w.pixels,n.pixels,c.name);
 const r=JSON.parse(n.metadata);assert.equal(r.status==='error'?r.error.code:r.status,c.expected??'rendered',c.name);
 const record={name:c.name,validRequest:c.validRequest??true,requestPath:`${root}/${c.name}.request.json`,responsePath:`${root}/${c.name}.response.json`,requestSha256:sha(json),responseSha256:sha(n.metadata),status:r.status==='error'?r.error.code:r.status};
 fs.writeFileSync(record.requestPath,json);fs.writeFileSync(record.responsePath,n.metadata);
 if(r.status==='rendered'){
  assert.equal(componentCalls,before+1);assert(frame);assert.equal(r.info.frameSha256,sha(frame));assert.equal(r.info.sha256,sha(n.pixels));assert.equal(BigInt(r.info.byteLength),BigInt(n.pixels.length));assert.equal(n.pixels.length,c.q.viewport.width*c.q.viewport.height*4);
  assert.deepEqual(probe(frame),n.pixels,c.name+' standalone C++');
  if(c.originalPixels)assert.deepEqual(fs.readFileSync(c.originalPixels),n.pixels,c.name+' original shape reference');
  record.framePath=`${root}/${c.name}.frame.bin`;record.pixelsPath=`${root}/${c.name}.rgba`;record.frameSha256=sha(frame);record.pixelsSha256=sha(n.pixels);record.byteLength=n.pixels.length;record.work=r.info.work;
  fs.writeFileSync(record.framePath,frame);fs.writeFileSync(record.pixelsPath,n.pixels);
  if(c.paragraphSource)record.paragraphSource=c.paragraphSource;
 }else {assert.equal(componentCalls,before);assert.equal(n.pixels.length,0);assert.equal(frame,null);}
 if(c.equivalent)equivalents.push([c.name,c.equivalent]);responses.set(c.name,r);cases.push(record);
}
for(const [a,b] of equivalents){assert.equal(responses.get(a).info.frameSha256,responses.get(b).info.frameSha256);assert.equal(responses.get(a).info.sha256,responses.get(b).info.sha256);}
// One process can reject an input, then safely process another complete job.
const jobs=[JSON.stringify(solid()),'{',JSON.stringify(base())],multi=spawnSync('target/release/mo-raster-worker',[],{input:Buffer.concat(jobs.map(packet)),timeout:30000,maxBuffer:1024*1024,env:{}});
assert.equal(multi.status,0);let remaining=multi.stdout;
for(const json of jobs){const r=parseReply(remaining),expected=native(json);assert.equal(r.metadata,expected.metadata);assert.deepEqual(r.pixels,expected.pixels);remaining=remaining.subarray(r.consumed);}assert.equal(remaining.length,0);
// CLI publication: verified pixels only, exclusive destination, no stage leaks.
const cliRequest=root+'/cli.json',cliPixels=root+'/cli.rgba',failedPixels=root+'/failure.rgba';
for(const p of [cliPixels,failedPixels])fs.rmSync(p,{force:true});fs.writeFileSync(cliRequest,JSON.stringify(solid()));
function cli(output){return spawnSync('target/release/mo-cli',['render-paths',cliRequest,output],{encoding:'utf8',timeout:30000});}
const published=cli(cliPixels);assert.equal(published.status,0,published.stderr);const expected=native(JSON.stringify(solid()));assert.equal(published.stdout.trimEnd(),expected.metadata);assert.deepEqual(fs.readFileSync(cliPixels),expected.pixels);
assert.equal(fs.statSync(cliPixels).mode&0o777,0o600);assert.notEqual(cli(cliPixels).status,0);assert.deepEqual(fs.readFileSync(cliPixels),expected.pixels);
fs.writeFileSync(cliRequest,'{}');const rejected=cli(failedPixels);assert.equal(rejected.status,0);assert.equal(JSON.parse(rejected.stdout).error.code,'INPUT_INVALID');assert(!fs.existsSync(failedPixels));assert(!fs.readdirSync(root).some(n=>n.startsWith('.musteroffice-')));
// Fault replies test the Rust bridge itself, not just the TS module adapter.
const faults=[];
for(const [name,reply] of [['status',{status:NaN,pixels:new Uint8Array()}],['failure-with-bytes',{status:1,pixels:new Uint8Array(1)}],['short',{status:0,pixels:new Uint8Array()}],['long',{status:0,pixels:new Uint8Array(8)}],['array-like',{status:0,pixels:{length:0xffffffff}}],['wrong-typed-array',{status:0,pixels:new Uint32Array(4)}],['non-premultiplied',{status:0,pixels:Uint8Array.of(255,0,0,0)}],['pixels-getter',{status:0,get pixels(){throw Error('denied');}}],['host-throw',null]]){
 let discarded=0;const q=base();q.viewport.width=q.viewport.height=1;
 const r=fromWasm(JSON.stringify(q),{raster(){if(reply===null)throw Error('trap');return reply;},invalidate(){discarded++;}});
 assert.equal(JSON.parse(r.metadata).error.code,reply===null?'HOST_FAILURE':'COMPONENT_INVALID');assert.equal(r.pixels.length,0);assert.equal(discarded,1);faults.push(name);
}
// Actual component heap exhaustion traverses Rust error handling, with no pixels.
let raw;const exhausted=await RasterComponent.create(async options=>{raw=await factory(options);return raw;},module);
let heldMiB=0;while(raw._malloc(1024*1024))heldMiB++;assert(heldMiB>100);
const big=base();big.viewport.width=big.viewport.height=4096;
const failure=fromWasm(JSON.stringify(big),exhausted);assert.equal(JSON.parse(failure.metadata).error.code,'COMPONENT_FAILURE');assert.equal(failure.pixels.length,0);assert(exhausted.invalid);
const reuse=fromWasm(JSON.stringify(base()),exhausted);assert.equal(JSON.parse(reuse.metadata).error.code,'HOST_FAILURE');assert.equal(reuse.pixels.length,0);
const replacement=await RasterComponent.create(factory,module);assert.equal(fromWasm(JSON.stringify(base()),replacement).metadata,native(JSON.stringify(base())).metadata);
const report={format:'musteroffice.path-raster-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),adapterSha256:sha(fs.readFileSync('.codex-work/raster-component/index.js')),cases,equivalents,exactPixelAndMetadataEquality:true,standaloneComponentComparison:true,cliPublicationVerified:true,consecutiveWorkerJobs:jobs.length,bridgeFaults:faults,actualAllocationFailure:{heldMiB,failure:JSON.parse(failure.metadata),reuse:JSON.parse(reuse.metadata),replacementVerified:true}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,rendered:cases.filter(c=>c.status==='rendered').length,paragraphs:cases.filter(c=>c.paragraphSource).length,faults:faults.length,heldMiB}));
