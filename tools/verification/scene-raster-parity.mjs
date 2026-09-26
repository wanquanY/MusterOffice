import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import {fixtures,fromPaths} from './scene-raster-fixtures.mjs';
import {base,solid} from './path-raster-fixtures.mjs';
const root='.codex-work/scene-raster',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
fs.mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex'),componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm'),module=new WebAssembly.Module(componentBytes);
const component=await RasterComponent.create(factory,module),cases=[];let frame,calls=0;
const backend={raster(words){calls++;frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function packet(json){const data=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(data.length);return Buffer.concat([h,data]);}
function parse(bytes){const m=bytes.readUInt32LE(),p=bytes.readUInt32LE(4);assert.equal(bytes.length,8+m+p);return {metadata:bytes.subarray(8,8+m).toString(),pixels:bytes.subarray(8+m)};}
function native(json){const r=spawnSync('target/release/mo-raster-worker',['--scene'],{input:packet(json),timeout:30000,maxBuffer:80*1024*1024,env:{}});assert.equal(r.status,0,r.stderr?.toString());return parse(r.stdout);}
function wasmCall(json,b=backend){const r=wasm.render_scene(json,b);return {metadata:r.metadata,pixels:Buffer.from(r.take_pixels())};}
for(const c of fixtures()){
 const json=typeof c.q==='string'?c.q:JSON.stringify(c.q),n=native(json),before=calls;frame=null;const w=wasmCall(json);
 assert.equal(w.metadata,n.metadata,c.name);assert.deepEqual(w.pixels,n.pixels,c.name);const r=JSON.parse(n.metadata);
 assert.equal(r.status==='error'?r.error.code:r.status,c.expected??'rendered',c.name);
 const record={name:c.name,validRequest:c.validRequest??true,requestPath:`${root}/${c.name}.request.json`,responsePath:`${root}/${c.name}.response.json`,requestSha256:sha(json),responseSha256:sha(n.metadata),status:r.status==='error'?r.error.code:r.status};
 fs.writeFileSync(record.requestPath,json);fs.writeFileSync(record.responsePath,n.metadata);
 if(r.status==='rendered'){
  assert.equal(calls,before+1);assert.equal(r.info.raster.sha256,sha(n.pixels));assert.equal(r.info.raster.frameSha256,sha(frame));
  assert(BigInt(r.info.work.combinedCoordinateErrorBound)<=BigInt(c.q.viewport.coordinateTolerance));
  if(c.attempts)assert.equal(r.info.work.loweringAttempts,c.attempts);
  if(c.referencePixels)assert.deepEqual(n.pixels,fs.readFileSync(c.referencePixels),c.name+' prior pixels');
  if(c.name==='shared-shear-alpha')assert.equal(r.info.work.compiledPaths,1);
  const h=Buffer.alloc(4);h.writeUInt32LE(frame.length/4);
  const probe=spawnSync('.codex-work/skia/mo-skia-probe',[],{input:Buffer.concat([h,frame]),timeout:30000,maxBuffer:80*1024*1024});assert.equal(probe.status,0);assert.equal(probe.stdout.readUInt32LE(),0);assert.deepEqual(probe.stdout.subarray(8),n.pixels);
  Object.assign(record,{framePath:`${root}/${c.name}.frame.bin`,pixelsPath:`${root}/${c.name}.rgba`,frameSha256:sha(frame),pixelsSha256:sha(n.pixels),work:r.info.work});
  fs.writeFileSync(record.framePath,frame);fs.writeFileSync(record.pixelsPath,n.pixels);
  if(c.paragraphSource)record.paragraphSource=c.paragraphSource;
 }else{assert.equal(calls,before);assert.equal(frame,null);assert.equal(n.pixels.length,0);}
 cases.push(record);
}
const request=JSON.stringify(fromPaths(solid())),requestPath=root+'/cli.json',outputPath=root+'/cli.rgba';fs.writeFileSync(requestPath,request);fs.rmSync(outputPath,{force:true});
function cli(){return spawnSync('target/release/mo-cli',['render-scene',requestPath,outputPath],{encoding:'utf8',timeout:30000});}
const c=cli();assert.equal(c.status,0,c.stderr);const n=native(request);assert.equal(c.stdout.trimEnd(),n.metadata);assert.deepEqual(fs.readFileSync(outputPath),n.pixels);assert.notEqual(cli().status,0);assert.deepEqual(fs.readFileSync(outputPath),n.pixels);
let raw;const exhausted=await RasterComponent.create(async options=>{raw=await factory(options);return raw;},module);let heldMiB=0;while(raw._malloc(1024*1024))heldMiB++;assert(heldMiB>100);
const large=fromPaths(base());large.viewport.width=large.viewport.height=4096;
const failed=wasmCall(JSON.stringify(large),exhausted);assert.equal(JSON.parse(failed.metadata).error.code,'COMPONENT_FAILURE');assert.equal(failed.pixels.length,0);assert(exhausted.invalid);
const reused=wasmCall(request,exhausted);assert.equal(JSON.parse(reused.metadata).error.code,'HOST_FAILURE');assert.equal(reused.pixels.length,0);assert.equal(wasmCall(request).metadata,n.metadata);
const report={format:'musteroffice.scene-raster-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),cases,exactPixelAndMetadataEquality:true,standaloneComponentComparison:true,cliExclusivePublication:true,actualAllocationFailure:{heldMiB,failure:JSON.parse(failed.metadata),reuse:JSON.parse(reused.metadata),replacementVerified:true}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,rendered:cases.filter(c=>c.status==='rendered').length,rotatedParagraphs:cases.filter(c=>c.paragraphSource).length}));
