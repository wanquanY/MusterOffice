// Preserve prior source text pages, text geometry and raw image rendering.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {pathToFileURL} from 'node:url';
const componentDirectory=process.argv[3]??'.codex-work/image-brush/component';
const adapterPath=process.argv[4]??'.codex-work/image-brush/ts-raster/index.js';
const {RasterComponent}=await import(pathToFileURL(path.resolve(adapterPath)));
const {default:skiaFactory}=await import(pathToFileURL(path.resolve(componentDirectory,'mo-skia.mjs')));
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root=process.argv[2]??'.codex-work/source-images',out=root+'/recent-regressions';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const previous='docs/reviews/evidence/2026-09-25-image-brush-completed-verification.json';
assert.equal(entry(previous).sha256,'4f407ae1696efeb49337e113f0a399576f33f44f5994bc621733883f0c86c31a');
const old=JSON.parse(fs.readFileSync(previous)),cases=[];
const wasm=createRequire(import.meta.url)(path.resolve(root,'wasm-node/mo_wasm.js'));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const raster=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync(componentDirectory+'/mo-skia.wasm')));
function native(binary,mode,buffers,pixelOutput){
 const header=Buffer.alloc(buffers.length*4);buffers.forEach((b,i)=>header.writeUInt32LE(b.length,4*i));
 const n=spawnSync('target/debug/'+binary,[mode],{input:Buffer.concat([header,...buffers]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());const length=n.stdout.readUInt32LE(),offset=pixelOutput?8:4;
 const metadata=n.stdout.subarray(offset,offset+length).toString(),pixels=n.stdout.subarray(offset+length);
 if(pixelOutput)assert.equal(pixels.length,n.stdout.readUInt32LE(4));else assert.equal(pixels.length,0);
 return {metadata,pixels};
}
function save(c,metadata,pixels,expected){
 assert.equal(metadata,read(expected).toString(),c.name);
 const file=out+'/'+cases.length+'.json';fs.writeFileSync(file,metadata);
 const r={...c,response:entry(file),priorResponse:expected};
 if(c.pixels){assert.deepEqual(pixels,read(c.pixels));const file=out+'/'+cases.length+'.rgba';fs.writeFileSync(file,pixels);r.pixels=entry(file);r.priorPixels=c.pixels;}
 else assert.equal(pixels.length,0);
 cases.push(r);
}
for(const c of old.reports['recent-regressions'].evidence.cases){
 const q=read(c.request);
 if(c.fonts){
  const source=read(c.source),fonts=read(c.fonts),n=native('mo-raster-worker','--pptx-text-page',[q,source,fonts],true);
  const w=wasm.render_pptx_text_page(q.toString(),source,fonts,shaper,raster);assert.equal(w.metadata,n.metadata);assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels);
  save(c,n.metadata,n.pixels,c.response);
 }else{
  const fonts=read(c.bundle),n=native('mo-text-worker','--geometry',[q,fonts],false);
  assert.equal(wasm.layout_lines(q.toString(),fonts,shaper),n.metadata);save(c,n.metadata,n.pixels,c.response);
 }
}
for(const c of old.reports.parity.evidence.cases){
 const q=read(c.request),images=read(c.images),n=native('mo-raster-worker','--images',[q,images],true);
 const w=wasm.render_image_paths(q.toString(),images,raster);assert.equal(w.metadata,n.metadata);assert.deepEqual(Buffer.from(w.take_pixels()),n.pixels);
 save(c,n.metadata,n.pixels,c.runtime);
}
const c=old.reports.parity.evidence.cases.find(c=>c.name==='premultiplied-nearest-clamp'),good=JSON.parse(read(c.request)),images=read(c.images);
const invalid=[q=>q.extra=true,q=>q.images[0].extra=true,q=>q.raster.draws[0].brush.image.extra=true,q=>q.raster.draws[0].brush.image.sampling='cubic',q=>q.raster.draws[0].brush.image.origin.x=0,q=>delete q.images[0].sha256];
const rejections=[];
for(let i=0;i<7;i++){
 const q=structuredClone(good);if(i<6)invalid[i](q);
 const text=i<6?JSON.stringify(q):'{"images":[],"images":[]}',n=native('mo-raster-worker','--images',[Buffer.from(text),images],true);
 const w=wasm.render_image_paths(text,images,raster);assert.equal(w.metadata,n.metadata);assert.equal(w.take_pixels().length,0);assert.equal(n.pixels.length,0);assert.equal(JSON.parse(n.metadata).error.code,'INPUT_INVALID');
 const prefix=out+'/invalid-'+i;fs.writeFileSync(prefix+'.request.json',text);fs.writeFileSync(prefix+'.response.json',n.metadata);
 rejections.push({request:entry(prefix+'.request.json'),images:c.images,response:entry(prefix+'.response.json')});
}
assert.equal(cases.length,204);assert(!raster.invalid);
const counts={requests:211,priorStableResponses:204,textPages:124,textGeometry:41,imageRequests:39,jsonRejections:7};
fs.writeFileSync(root+'/recent-regressions.json',JSON.stringify({format:'musteroffice.source-image-regressions/1',previousEvidence:entry(previous),counts,cases,rejections},null,2)+'\n');
console.log(JSON.stringify(counts));
