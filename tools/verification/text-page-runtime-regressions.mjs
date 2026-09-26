// Re-execute old Native requests AND the current Rust WASM build. Historic
// runtime totals remain separate from this explicitly bounded regression set.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';

import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
const componentDirectory=process.argv[3]??'.codex-work/skia';
const adapterPath=process.argv[4]??'.codex-work/raster-component/index.js';
const {RasterComponent}=await import(pathToFileURL(path.resolve(adapterPath)));
const {default:skiaFactory}=await import(pathToFileURL(path.resolve(componentDirectory,'mo-skia.mjs')));
const root=process.argv[2]??'.codex-work/text-page-runtime',out=root+'/regressions';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)(`../../${root}/wasm-node/mo_wasm.js`);
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const previous='docs/reviews/evidence/2026-09-25-source-text-page-library-verification.json';
assert.equal(entry(previous).sha256,'5f39994c5f0e0c7fe53fc7e8114d2d1ec650f9a1732ec07c797bb9208364c0b5');
const old=JSON.parse(fs.readFileSync(previous));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const raster=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync(componentDirectory+'/mo-skia.wasm')));
const cases=[];let countPixels=0;
function save(c,actual){const path=`${out}/${cases.length}.json`;fs.writeFileSync(path,actual);const record={...c,response:entry(path),priorResponse:c.response};cases.push(record);return record;}
for(const c of old.pageRegressionEvidence.cases){
 const source=read(c.source),json=c.request?read(c.request).toString():null,expected=read(c.response).toString().trimEnd();
 const output=`${out}/page-${cases.length}.rgba`;if(fs.existsSync(output))fs.unlinkSync(output);
 const args=c.mode==='source'?['pptx-inspect',c.source.path]:c.mode==='compile'?['compile-pptx-page',c.request.path,c.source.path]:['render-pptx-page',c.request.path,c.source.path,output];
 const n=spawnSync('target/debug/mo-cli',args,{encoding:'utf8',timeout:60000,maxBuffer:80*1024*1024});assert.equal(n.status,0,n.stderr);assert.equal(n.stdout.trimEnd(),expected,c.name);
 let actual,pixels;
 if(c.mode==='source')actual=wasm.inspect_pptx(source);
 else if(c.mode==='compile')actual=wasm.compile_pptx_page(json,source);
 else {const r=wasm.render_pptx_page(json,source,raster);actual=r.metadata;pixels=Buffer.from(r.take_pixels());}
 assert.equal(actual,expected,c.name);const r=save(c,actual);
 if(c.pixels){assert.deepEqual(pixels,read(c.pixels));assert.deepEqual(fs.readFileSync(output),pixels);r.pixels=entry(output);r.priorPixels=c.pixels;countPixels++;}
 else {assert.ok(!pixels||!pixels.length);assert.equal(fs.existsSync(output),false);}
}
const methods={'--paragraph':'shape_paragraph','--lines':'shape_lines','--geometry':'layout_lines','--layout':'layout_paragraph','--paths':'paragraph_paths'};
for(const c of old.textRegressionEvidence.cases){
 const q=read(c.request),bundle=read(c.bundle),expected=read(c.response).toString().trimEnd(),header=Buffer.alloc(8);header.writeUInt32LE(q.length);header.writeUInt32LE(bundle.length,4);
 const n=spawnSync('target/debug/mo-text-worker',[c.mode],{input:Buffer.concat([header,q,bundle]),timeout:60000,maxBuffer:80*1024*1024,env:{}});assert.equal(n.status,0,n.stderr?.toString());assert.equal(n.stdout.length,4+n.stdout.readUInt32LE());assert.equal(n.stdout.subarray(4).toString(),expected,c.name);
 const actual=wasm[methods[c.mode]](q.toString(),bundle,shaper);assert.equal(actual,expected,c.name);save(c,actual);
}
assert.equal(cases.length,307);assert.equal(countPixels,21);
const artifacts=['target/debug/mo-cli','target/debug/mo-text-worker','target/debug/mo-raster-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`].map(entry);
fs.writeFileSync(root+'/regressions.json',JSON.stringify({format:'musteroffice.text-page-runtime-regressions/1',previousEvidence:entry(previous),artifacts,counts:{requests:307,pageRequests:100,textRequests:207,pixelOutputs:21},cases},null,2)+'\n');console.log(JSON.stringify({requests:307,pixelOutputs:21}));
