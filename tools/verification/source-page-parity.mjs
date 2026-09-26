import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/skia/mo-skia.mjs';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const root='.codex-work/source-page',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'), cases=[];
const component=await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
let frame=null,calls=0;
const backend={raster(words){calls++;frame=Buffer.alloc(words.length*4);words.forEach((w,i)=>frame.writeUInt32LE(w,i*4));return component.raster(words);},invalidate(){component.invalidate();}};
function store(c,kind,q,raw,validRequest=true) {
 const r={name:c.name,kind,sourcePath:c.path,sourceSha256:c.sha256,responsePath:`${root}/${c.name}.${kind}.response.json`,responseSha256:sha(raw),validRequest};fs.writeFileSync(r.responsePath,raw);
 if(q!==null){const text=typeof q==='string'?q:JSON.stringify(q);r.requestPath=`${root}/${c.name}.${kind}.request.json`;r.requestSha256=sha(text);fs.writeFileSync(r.requestPath,text);}
 cases.push(r);return r;
}
function request(c){return {expectedSourceSha256:c.sha256,slide:'/ppt/slides/slide1.xml',profile:'drawingml-static-solid-page-v1-draft',colorContext:{systemColors:{},placeholder:null},viewport:{width:800,height:450,origin:{x:'0',y:'0'},scale:{numerator:800,denominator:12192000},coordinateTolerance:'16777216',background:[255,255,255,255]}};}
function compile(c,q,valid=true){
 const raw=typeof q==='string'?q:JSON.stringify(q),path=root+'/query.json';fs.writeFileSync(path,raw);
 const n=spawnSync('target/release/mo-cli',['compile-pptx-page',path,c.path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);
 const w=wasm.compile_pptx_page(raw,fs.readFileSync(c.path));assert.equal(n.stdout.trimEnd(),w,c.name);store(c,'compile',q,w,valid);return JSON.parse(w);
}
function render(c,q,valid=true){
 const raw=typeof q==='string'?q:JSON.stringify(q),source=fs.readFileSync(c.path),h=Buffer.alloc(8);h.writeUInt32LE(Buffer.byteLength(raw));h.writeUInt32LE(source.length,4);
 const n=spawnSync('target/release/mo-raster-worker',['--pptx-page'],{input:Buffer.concat([h,Buffer.from(raw),source]),timeout:30000,maxBuffer:80*1024*1024,env:{}});assert.equal(n.status,0,n.stderr?.toString());
 const m=n.stdout.readUInt32LE(),meta=n.stdout.subarray(8,8+m).toString(),pixels=n.stdout.subarray(8+m);assert.equal(n.stdout.readUInt32LE(4),pixels.length);
 frame=null;calls=0;const w=wasm.render_pptx_page(raw,source,backend);assert.equal(w.metadata,meta,c.name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,c.name);
 const result=JSON.parse(meta),r=store(c,'raster',q,meta,valid);
 if(result.status==='rendered') {
  assert.equal(calls,1);assert.equal(pixels.length,800*450*4);
  Object.assign(r,{pixelsPath:`${root}/${c.name}.rgba`,pixelsSha256:sha(pixels),framePath:`${root}/${c.name}.frame.bin`,frameSha256:sha(frame)});fs.writeFileSync(r.pixelsPath,pixels);fs.writeFileSync(r.framePath,frame);
 } else {assert.equal(calls,0);assert.equal(pixels.length,0);assert.equal(frame,null);}
 return result;
}
const manifest=JSON.parse(fs.readFileSync(root+'/manifest.json'));
for(const c of manifest.cases){
 const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);
 const n=spawnSync('target/release/mo-cli',['pptx-inspect',c.path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);
 const s=wasm.inspect_pptx(bytes);assert.equal(n.stdout.trimEnd(),s,c.name);assert.equal(JSON.parse(s).status,'inspected');store(c,'source',null,s);
 const q=request(c),p=compile(c,q),r=render(c,q);
 if(c.issue){assert.equal(p.status,'error',c.name);assert.equal(p.error.code,'MAPPING_NOT_IMPLEMENTED',c.name);assert.equal(p.error.issue.kind,c.issue,c.name);assert.deepEqual(p,r);}
 else {assert.equal(p.status,'compiled',c.name+JSON.stringify(p));assert.equal(r.status,'rendered');assert.deepEqual(p.plan.info,r.info.page);assert.equal(p.plan.downstreamCoordinateErrorBound,r.info.downstreamCoordinateErrorBound);}
}
const base=manifest.cases[0];
for(const [name,change,code,valid]of [
 ['digest-conflict',q=>q.expectedSourceSha256='0'.repeat(64),'SOURCE_CONFLICT',true],
 ['missing-slide',q=>q.slide='/ppt/no.xml','INPUT_INVALID',true],
 ['layout-as-page',q=>q.slide='/ppt/slideLayouts/slideLayout2.xml','INPUT_INVALID',true],
 ['bad-viewport',q=>q.viewport.width=801,'INPUT_INVALID',true],
 ['unknown-profile',q=>q.profile='html','INPUT_INVALID',false],
 ['unknown-field',q=>q.script='unknown','INPUT_INVALID',false],
 ['zero-scale',q=>q.viewport.scale.numerator=0,'INPUT_INVALID',true],
 ]) {
 const c={...base,name},q=request(c);change(q);const p=compile(c,q,valid);assert.equal(p.error.code,code,name);assert.deepEqual(render(c,q,valid),p,name);
}
const duplicate=JSON.stringify(request(base)).replace('"slide":','"slide":"ignored","slide":');
assert.equal(compile({...base,name:'duplicate-field'},duplicate,false).error.code,'INPUT_INVALID');
assert.equal(render({...base,name:'duplicate-field'},duplicate,false).error.code,'INPUT_INVALID');
// Worker framing is bounded before allocating source bytes; incomplete requests
// never become raster replies or successful CLI publications.
const malformed=[];
for(const [name,input]of [['source-limit',Buffer.from([0,0,0,0,1,0,0,8])],['truncated-source',Buffer.from([0,0,0,0,4,0,0,0,1])]]){
 const r=spawnSync('target/release/mo-raster-worker',['--pptx-page'],{input,timeout:30000,env:{}});assert.notEqual(r.status,0);assert.equal(r.stdout.length,0);malformed.push({name,exitStatus:r.status,stderr:r.stderr.toString().trim(),stdoutBytes:0});
}
const native=fs.readFileSync('target/release/mo-cli'),rust=fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm');
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.source-page-parity/1',nativeCliSha256:sha(native),rustWasmSha256:sha(rust),nativeRasterWorkerSha256:sha(fs.readFileSync('target/release/mo-raster-worker')),exactNativeWasmResponsesAndPixels:true,cases,malformed},null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,rendered:cases.filter(c=>c.pixelsPath).length,workerRejections:malformed.length}));
