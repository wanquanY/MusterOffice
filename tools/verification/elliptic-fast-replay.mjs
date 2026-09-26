/** Re-execute frozen public requests with current Native and C++ WASM runtimes. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/elliptic-fast/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/elliptic-fast',out=root+'/replay';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const wasm=createRequire(import.meta.url)('../../.codex-work/elliptic-source/wasm-node/mo_wasm.js');
const skm=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const component=await RasterComponent.create(skiaFactory,skm),shaper=await ShapingComponent.create(hbFactory,hbm);
let frame=null,rasters=0,decodes=0;
const raster={raster(f){rasters++;frame=f.slice();return component.raster(f);},
 rasterImages(f,b){rasters++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
const records=[];
for(const kind of ['runtime','source-parity']){
 const old=JSON.parse(fs.readFileSync('.codex-work/elliptic-source/'+kind+'.json'));
 for(const c of old.cases){
  const json=load(c.request),source=c.source?load(c.source):null,fonts=c.fonts?load(c.fonts):null;
  const isScene=kind==='runtime'&&(c.name.startsWith('scene-')||c.name.startsWith('prior-scene-'));
  const h=Buffer.alloc(source?12:4);h.writeUInt32LE(json.length);
  if(source){h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);}
  const args=source?['--pptx-resource-page']:isScene?['--scene']:[];
  const n=spawnSync('target/debug/mo-raster-worker',args,{input:Buffer.concat(source?[h,json,source,fonts]:[h,json]),env:{},timeout:60000,maxBuffer:80*1024*1024});
  assert.equal(n.status,0,c.name+': '+n.stderr.toString());assert.equal(n.stderr.length,0);
  const ml=n.stdout.readUInt32LE(),pl=n.stdout.readUInt32LE(4);assert.equal(n.stdout.length,8+ml+pl);
  const metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml);
  frame=null;rasters=0;decodes=0;
  const w=source?wasm.render_pptx_resource_page(json.toString(),source,fonts,decoder,shaper,raster):
   isScene?wasm.render_scene(json.toString(),raster):wasm.render_paths(json.toString(),raster);
  assert.equal(w.metadata,metadata,c.name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,c.name);assert(!component.invalid);
  assert.equal(metadata,load(c.response).toString(),c.name+' old/current response');
  assert.deepEqual(pixels,c.pixels?load(c.pixels):Buffer.alloc(0),c.name+' old/current pixels');
  if(c.frame)assert.deepEqual(Buffer.from(frame.buffer),load(c.frame),c.name+' old/current frame');else assert.equal(frame,null);
  if(kind==='runtime')assert.equal(rasters,c.calls);else{assert.equal(rasters,c.rasters);assert.equal(decodes,c.decodes);}
  const p=out+'/'+records.length;
  records.push({kind,name:c.name,request:c.request,...(source?{source:c.source,fonts:c.fonts}:{}),
   response:put(p+'.response.json',metadata),...(c.pixels?{pixels:put(p+'.rgba',pixels)}:{}),...(c.frame?{frame:c.frame}:{}),
   status:c.status,rasters,decodes,previousResponse:c.response});
 }
 console.log(JSON.stringify({kind,paired:old.cases.length}));
}
assert.equal(records.length,355);
const triples=[];
// Actual V12 frame corpus, including the two late-budget failures in both modes.
const frames=JSON.parse(fs.readFileSync('.codex-work/elliptic-source/components.json'));
for(const c of frames.cases){
 const b=load(c.frame),f=new Uint32Array(b.buffer.slice(b.byteOffset,b.byteOffset+b.length));
 const w=c.images?component.rasterImages(f,new Uint8Array()):component.raster(f);
 assert.equal(w.status,c.status);assert(!component.invalid);
 const pixels=c.pixels?load(c.pixels):Buffer.alloc(0);assert.deepEqual(Buffer.from(w.pixels),pixels,c.name);
 for(const suffix of ['', '-asan']){
  const h=Buffer.alloc(4);h.writeUInt32LE(f.length);
  const data=Buffer.concat(c.images?[h,b,Buffer.alloc(4)]:[h,b]);
  const n=spawnSync(root+'/component/mo-skia-probe'+suffix,c.images?['--images']:[],
   {input:data,env:{ASAN_OPTIONS:'detect_leaks=0',UBSAN_OPTIONS:'halt_on_error=1'},timeout:60000,maxBuffer:1<<27});
  assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);assert.equal(n.stdout.readUInt32LE(),c.status);
  assert.equal(n.stdout.length,n.stdout.readUInt32LE(4)+8);assert.deepEqual(n.stdout.subarray(8),pixels,c.name);
 }
 triples.push(c);
}
const positive=records.find(c=>c.name==='new/center-point'),negative=records.find(c=>c.name.endsWith('/over-limit'));
const cliDir=fs.mkdtempSync(root+'/cli-'),output=cliDir+'/page.rgba',failure=cliDir+'/failure.rgba';
const cli=(c,p)=>spawnSync('target/debug/mo-cli',['render-pptx-resource-page',c.request.path,c.source.path,c.fonts.path,p],{env:{},timeout:60000,maxBuffer:80*1024*1024});
const created=cli(positive,output);assert.equal(created.status,0,created.stderr.toString());assert.deepEqual(fs.readFileSync(output),load(positive.pixels));
const before=entry(output),again=cli(positive,output);assert.notEqual(again.status,0);assert.deepEqual(entry(output),before);
const bad=cli(negative,failure);assert.equal(bad.status,0,bad.stderr.toString());assert(!fs.existsSync(failure));assert.equal(JSON.parse(bad.stdout).status,'error');
fs.writeFileSync(root+'/replay.json',JSON.stringify({format:'musteroffice.elliptic-fast-replay/1',pairedCalls:records.length,
 cases:records,componentTriples:triples.length,frames:triples,
 previousReports:['runtime','source-parity','components'].map(n=>entry('.codex-work/elliptic-source/'+n+'.json')),
 currentArtifacts:[entry('target/debug/mo-raster-worker'),entry('target/debug/mo-cli'),entry(root+'/component/mo-skia-probe'),entry(root+'/component/mo-skia-probe-asan'),entry(root+'/component/mo-skia.wasm')],
 cli:{pixels:before,createResponse:put(cliDir+'/created.json',created.stdout),overwriteExit:again.status,overwriteError:put(cliDir+'/overwrite.txt',again.stderr),failureResponse:put(cliDir+'/failure.json',bad.stdout),failureOutputAbsent:true}},null,2)+'\n');
console.log(JSON.stringify({paired:records.length,componentTriples:triples.length,cliVerified:true}));
