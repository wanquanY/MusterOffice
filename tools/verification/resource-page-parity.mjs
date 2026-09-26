/** Execute the actual source/resource page API end to end in Native and WASM. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {RasterComponent} from '../../.codex-work/clips/ts-raster/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import skiaFactory from '../../.codex-work/clips/component/mo-skia.mjs';
const root='.codex-work/resource-page',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const wasm=createRequire(import.meta.url)('../../.codex-work/resource-page/wasm-node/mo_wasm.js');
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const skm=new WebAssembly.Module(fs.readFileSync('.codex-work/clips/component/mo-skia.wasm'));
const font=fs.readFileSync('fixtures/fonts/owned.ttf'),manifest=JSON.parse(fs.readFileSync('fixtures/fonts/manifest-paragraph.json')).manifest;
const fixtures=JSON.parse(fs.readFileSync(root+'/fixtures.json'));const records=[];
function request(source){return {profile:'drawingml-resource-page-q32-v1-draft',page:{expectedSourceSha256:sha(source),slide:'/ppt/slides/slide1.xml',profile:'drawingml-static-solid-page-v1-draft',colorContext:{systemColors:{},placeholder:null},viewport:{width:400,height:300,origin:{x:'0',y:'0'},scale:{numerator:1,denominator:4000},coordinateTolerance:'16777216',background:[255,255,255,255]}},imageSource:'embeddedSnapshot',sampling:'nearest',fonts:structuredClone(manifest)};}
function native(json,source,fonts){
 const j=Buffer.from(json),h=Buffer.alloc(12);h.writeUInt32LE(j.length);h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);
 const n=spawnSync('target/debug/mo-raster-worker',['--pptx-resource-page'],{input:Buffer.concat([h,j,source,fonts]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr.toString());const m=n.stdout.readUInt32LE(),p=n.stdout.readUInt32LE(4);assert.equal(n.stdout.length,8+m+p);
 return {metadata:n.stdout.subarray(8,8+m).toString(),pixels:n.stdout.subarray(8+m)};
}
async function run(name,source,q,fonts,expected,prior){
 const json=typeof q==='string'?q:JSON.stringify(q),n=native(json,source,fonts),r=JSON.parse(n.metadata);
 const shaper=await ShapingComponent.create(hbFactory,hbm),component=await RasterComponent.create(skiaFactory,skm);
 let decodeCalls=0,textCalls=0,rasterCalls=0;
 const text={...Object.fromEntries(['shapeBatch','measureBatch','outlineBatch'].map(k=>[k,(...a)=>{textCalls++;return shaper[k](...a);}])) ,invalidate(){shaper.invalidate();}};
 const decoder={decodeImage(b){decodeCalls++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
 const paint={rasterImages(f,b){rasterCalls++;return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
 const w=wasm.render_pptx_resource_page(json,source,fonts,decoder,text,paint);
 assert.equal(w.metadata,n.metadata,name);assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels,name);
 assert.equal(r.status,expected.status,name+': '+n.metadata);
 if(expected.decodeCalls!==undefined)assert.equal(decodeCalls,expected.decodeCalls,name);
 if(r.status==='rendered'){
  assert.equal(rasterCalls,1);assert.equal(textCalls,r.info.textWork.componentCalls);assert.equal(decodeCalls,r.info.decodedImages.length);
  assert.equal(r.info.page.scene.raster.sha256,sha(n.pixels));assert.equal(n.pixels.length,400*300*4);
  if(prior){assert.deepEqual(n.pixels,load(prior.priorPixels));const p=JSON.parse(load(prior.priorPlan));assert.deepEqual(r.info.images,p.imageWork);assert.deepEqual(r.info.page.page,p.page.info);}
 }else{
  assert.equal(n.pixels.length,0);assert.equal(rasterCalls,0);assert.equal(textCalls,0);
  if(expected.code)assert.equal(r.error.error.code,expected.code,name+': '+n.metadata);
  if(expected.imageIssue)assert.equal(r.error.image.kind,expected.imageIssue);
  if(expected.stage)assert.equal(r.error.stage,expected.stage);
 }
 shaper.invalidate();component.invalidate();
 const prefix=out+'/'+name,record={name,request:put(prefix+'.request.json',json),source:put(prefix+'.pptx',source),fonts:put(prefix+'.fonts.bin',fonts),response:put(prefix+'.response.json',n.metadata),status:r.status,decodeCalls,textCalls,rasterCalls};
 if(n.pixels.length)record.pixels=put(prefix+'.rgba',n.pixels);
 records.push(record);return {record,result:r,pixels:n.pixels};
}
for(const c of fixtures.cases){const s=load(c.source),q=request(s);q.imageSource=c.imageSource??q.imageSource;await run(c.name,s,q,font,c,c.priorPlan?c:undefined);}
const source=load(fixtures.cases.find(c=>c.name==='dual-fill').source),textSource=load(fixtures.cases.find(c=>c.name==='image-text').source);
let q=request(source);q.fonts=null;
await run('no-text-context',source,q,Buffer.alloc(0),{status:'rendered',decodeCalls:1});
q=request(textSource);q.fonts=null;
await run('text-context-required',textSource,q,Buffer.alloc(0),{status:'error',code:'RESOURCE_REQUIRED',decodeCalls:0});
q=request(source);q.fonts=null;
await run('unclaimed-fonts',source,q,font,{status:'error',stage:'request',code:'INPUT_INVALID',decodeCalls:0});
q=request(source);q.page.expectedSourceSha256='0'.repeat(64);
await run('source-conflict',source,q,font,{status:'error',stage:'source',code:'SOURCE_CONFLICT',decodeCalls:0});
q=request(source);q.page.viewport.width=0;
await run('invalid-viewport',source,q,font,{status:'error',code:'INPUT_INVALID',decodeCalls:0});
q=request(source);q.imageSource='linkedSource';
await run('no-linked-fallback',source,q,font,{status:'error',code:'RESOURCE_REQUIRED',decodeCalls:0});
q=request(source);
await run('duplicate-json',source,JSON.stringify(q).replace('"sampling":','"sampling":"linear","sampling":'),font,{status:'error',stage:'request',code:'INPUT_INVALID',decodeCalls:0});
await run('unknown-profile',source,{...q,profile:'html'},font,{status:'error',stage:'request',code:'INPUT_INVALID',decodeCalls:0});
q.fonts.fonts[0].expectedSha256='0'.repeat(64);
await run('font-conflict',source,q,font,{status:'error',stage:'fonts',code:'RESOURCE_CONFLICT',decodeCalls:0});
// Native CLI uses its real worker, verifies pixels, and publishes a new file.
const cliDir=fs.mkdtempSync(root+'/cli-'),positive=records.find(c=>c.name==='dual-fill'),negative=records.find(c=>c.name==='stationary');
const output=cliDir+'/page.rgba',failure=cliDir+'/failure.rgba';
function cli(c,p){return spawnSync('target/debug/mo-cli',['render-pptx-resource-page',c.request.path,c.source.path,c.fonts.path,p],{env:{},timeout:60000,maxBuffer:80*1024*1024});}
const c=cli(positive,output);assert.equal(c.status,0,c.stderr.toString());assert.deepEqual(fs.readFileSync(output),load(positive.pixels));
const before=entry(output),again=cli(positive,output);assert.notEqual(again.status,0);assert.deepEqual(entry(output),before);
const bad=cli(negative,failure);assert.equal(bad.status,0,bad.stderr.toString());assert(!fs.existsSync(failure));assert.equal(JSON.parse(bad.stdout).status,'error');
const report={format:'musteroffice.resource-page-parity/1',scope:'Actual Native/WASM public source-resource page operation, including source parsing, resource resolution, decoding, text, geometry, clipping and rasterization.',pairedCalls:records.length,cases:records,fixtures:entry(root+'/fixtures.json'),cli:{pixels:before,createResponse:put(cliDir+'/created.json',c.stdout),overwriteExit:again.status,overwriteError:put(cliDir+'/overwrite.txt',again.stderr),failureResponse:put(cliDir+'/failure.json',bad.stdout),failureOutputAbsent:true}};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({pairedCalls:records.length,successful:records.filter(c=>c.status==='rendered').length,cliVerified:true}));
