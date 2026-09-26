import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import {fixtures,base} from './page-render-fixtures.mjs';
const root='.codex-work/page-render',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
fs.mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex'),componentBytes=fs.readFileSync('.codex-work/skia/mo-skia.wasm'),module=new WebAssembly.Module(componentBytes);
const component=await RasterComponent.create(factory,module),cases=[];let frame,calls=0;
const backend={raster(words){calls++;frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function packet(json){const data=Buffer.from(json),h=Buffer.alloc(4);h.writeUInt32LE(data.length);return Buffer.concat([h,data]);}
function native(json){const r=spawnSync('target/release/mo-raster-worker',['--page'],{input:packet(json),timeout:30000,maxBuffer:80*1024*1024,env:{}});assert.equal(r.status,0,r.stderr?.toString());const b=r.stdout,m=b.readUInt32LE(),p=b.readUInt32LE(4);assert.equal(b.length,8+m+p);return {metadata:b.subarray(8,8+m).toString(),pixels:b.subarray(8+m)};}
function call(json,b=backend){const r=wasm.render_page(json,b);return {metadata:r.metadata,pixels:Buffer.from(r.take_pixels())};}
function pixel(bytes,width,x,y){return [...bytes.subarray(4*(y*width+x),4*(y*width+x+1))];}
for(const c of fixtures()){
 const json=typeof c.q==='string'?c.q:JSON.stringify(c.q),requestPath=`${root}/${c.name}.request.json`;fs.writeFileSync(requestPath,json);
 const planResult=spawnSync('target/release/mo-cli',['compile-page',requestPath],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(planResult.status,0,planResult.stderr);
 const compiled=wasm.compile_page(json);assert.equal(compiled,planResult.stdout.trimEnd(),c.name+' plan');const planResponse=JSON.parse(compiled);
 const n=native(json),before=calls;frame=null;const w=call(json);
 assert.equal(w.metadata,n.metadata,c.name);assert.deepEqual(w.pixels,n.pixels,c.name);const r=JSON.parse(n.metadata);
 assert.equal(r.status==='error'?r.error.code:r.status,c.expected,c.name);
 const record={name:c.name,validRequest:c.validRequest,requestPath,requestSha256:sha(json),responsePath:`${root}/${c.name}.response.json`,responseSha256:sha(n.metadata),planPath:`${root}/${c.name}.plan.json`,planSha256:sha(compiled),status:c.expected};
 fs.writeFileSync(record.responsePath,n.metadata);fs.writeFileSync(record.planPath,compiled);
 if(r.status==='rendered'){
  assert.equal(planResponse.status,'compiled');const plan=planResponse.plan;assert.equal(calls,before+1);
  assert.deepEqual(plan.info,r.info.page);assert.deepEqual(plan.deviceWork,r.info.scene.work);assert.equal(plan.combinedCoordinateErrorBound,r.info.combinedCoordinateErrorBound);
  assert(BigInt(r.info.combinedCoordinateErrorBound)<=BigInt(c.q.viewport.coordinateTolerance));
  assert.equal(r.info.scene.raster.sha256,sha(n.pixels));assert.equal(r.info.scene.raster.frameSha256,sha(frame));
  if(c.priorPixels)assert.deepEqual(n.pixels,fs.readFileSync(c.priorPixels),c.name+' unchanged prior pixels');
  if(c.name==='rectangle'||c.name==='repeated-8192'){
   assert.deepEqual(pixel(n.pixels,800,250,200),[180,40,70,255]);assert.deepEqual(pixel(n.pixels,800,99,120),[0,0,0,0]);
   assert.equal(plan.deviceWork.sourcePaths,1);assert.equal(plan.deviceWork.compiledPaths,1);assert.equal(plan.deviceWork.transforms,1);
  }
  if(c.name==='theme-layering'){
   assert.deepEqual(plan.paintSources.map(s=>s.object),[null,'master-shape','layout-shape','shape:1']);
   assert.deepEqual(pixel(n.pixels,800,700,500),[12,23,34,255]);assert.deepEqual(pixel(n.pixels,800,250,200),[20,180,90,255]);
   assert.deepEqual(pixel(n.pixels,800,75,150),[127,0,128,255]);
  }
  if(c.name==='alpha-composition')assert.deepEqual(pixel(n.pixels,800,250,200),[255,127,127,255]);
  if(c.name==='none-fill'||c.name==='transparent-fill'||c.name==='empty-page')assert(n.pixels.every(v=>v===0));
  if(c.name==='nonzero-holes')assert.deepEqual(pixel(n.pixels,800,250,200),[0,0,0,0]);
  Object.assign(record,{framePath:`${root}/${c.name}.frame.bin`,frameSha256:sha(frame),pixelsPath:`${root}/${c.name}.rgba`,pixelsSha256:sha(n.pixels),info:r.info});
  fs.writeFileSync(record.framePath,frame);fs.writeFileSync(record.pixelsPath,n.pixels);
 }else{assert.deepEqual(planResponse,r,c.name+' rejection');assert.equal(calls,before);assert.equal(frame,null);assert.equal(n.pixels.length,0);}
 cases.push(record);
}
const q=JSON.stringify(base()),requestPath=root+'/rectangle.request.json',outputPath=root+'/cli.rgba';fs.rmSync(outputPath,{force:true});
const cli=()=>spawnSync('target/release/mo-cli',['render-page',requestPath,outputPath],{encoding:'utf8',timeout:30000});
const result=cli(),n=native(q);assert.equal(result.status,0,result.stderr);assert.equal(result.stdout.trimEnd(),n.metadata);assert.deepEqual(fs.readFileSync(outputPath),n.pixels);assert.notEqual(cli().status,0);assert.deepEqual(fs.readFileSync(outputPath),n.pixels);
const failedOutput=root+'/failed-cli.rgba';fs.rmSync(failedOutput,{force:true});const rejected=spawnSync('target/release/mo-cli',['render-page',root+'/shape-text.request.json',failedOutput],{encoding:'utf8',timeout:30000});assert.equal(rejected.status,0);assert.equal(JSON.parse(rejected.stdout).error.code,'MAPPING_NOT_IMPLEMENTED');assert(!fs.existsSync(failedOutput));
const faults=[];
for(const [name,behavior,code]of [['throw',()=>{throw Error('owned injected bridge failure')},'HOST_FAILURE'],['length',()=>({status:0,pixels:new Uint8Array(1)}),'COMPONENT_INVALID'],['component',()=>({status:2,pixels:new Uint8Array()}),'COMPONENT_FAILURE']]){
 let invalidated=0,invoked=0;const r=call(q,{raster(){invoked++;return behavior();},invalidate(){invalidated++;}});assert.equal(JSON.parse(r.metadata).error.code,code);assert.equal(r.pixels.length,0);assert.equal(invalidated,1);assert.equal(invoked,1);faults.push({name,response:JSON.parse(r.metadata),invalidated,invoked});
}
const report={format:'musteroffice.page-render-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),cases,exactPixelAndMetadataEquality:true,exactPlanEquality:true,priorGroupPixelsUnchanged:3,cliExclusivePublication:true,failedPageHasNoArtifact:true,faults};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,rendered:cases.filter(c=>c.status==='rendered').length,rejected:cases.filter(c=>c.status!=='rendered').length}));
