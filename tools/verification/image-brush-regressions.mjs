// Current Native/Rust WASM with the new shared Skia component, old expectations.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {RasterComponent} from '../../.codex-work/image-brush/ts-raster/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import skiaFactory from '../../.codex-work/image-brush/component/mo-skia.mjs';
const root='.codex-work/image-brush',out=root+'/recent-regressions';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const previous='docs/reviews/evidence/2026-09-25-character-spacing-verification.json';
assert.equal(entry(previous).sha256,'3d8ef124acce831347b8bc967901a0b66c9e7e78ca74d26ab3f833c371e38220');
const old=JSON.parse(fs.readFileSync(previous)),cases=[];
const wasm=createRequire(import.meta.url)('../../.codex-work/image-brush/wasm-node/mo_wasm.js');
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const raster=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm')));
for(const c of old.parityEvidence.cases){
 const json=read(c.request),source=read(c.source),fonts=read(c.fonts),expected=read(c.response).toString(),header=Buffer.alloc(12);
 header.writeUInt32LE(json.length);header.writeUInt32LE(source.length,4);header.writeUInt32LE(fonts.length,8);
 const n=spawnSync('target/debug/mo-raster-worker',['--pptx-text-page'],{input:Buffer.concat([header,json,source,fonts]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());const length=n.stdout.readUInt32LE(),actual=n.stdout.subarray(8,8+length).toString(),pixels=n.stdout.subarray(8+length);
 assert.equal(actual,expected,c.name);assert.equal(pixels.length,n.stdout.readUInt32LE(4));
 const w=wasm.render_pptx_text_page(json.toString(),source,fonts,shaper,raster);
 assert.equal(w.metadata,expected,c.name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels);
 const file=out+'/'+cases.length+'.json';fs.writeFileSync(file,actual);
 const record={name:c.name,request:c.request,source:c.source,fonts:c.fonts,priorResponse:c.response,response:entry(file)};
 if(c.pixels){assert.deepEqual(pixels,read(c.pixels));const file=out+'/'+cases.length+'.rgba';fs.writeFileSync(file,pixels);record.pixels=entry(file);record.priorPixels=c.pixels;}
 else assert.equal(pixels.length,0);
 cases.push(record);
}
for(const c of old.geometryEvidence.cases){
 const json=read(c.request),fonts=read(c.bundle),expected=read(c.response).toString(),header=Buffer.alloc(8);
 header.writeUInt32LE(json.length);header.writeUInt32LE(fonts.length,4);
 const n=spawnSync('target/debug/mo-text-worker',['--geometry'],{input:Buffer.concat([header,json,fonts]),env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());assert.equal(n.stdout.readUInt32LE(),n.stdout.length-4);
 const actual=n.stdout.subarray(4).toString();assert.equal(actual,expected,c.name);
 assert.equal(wasm.layout_lines(json.toString(),fonts,shaper),actual);
 const file=out+'/'+cases.length+'.json';fs.writeFileSync(file,actual);
 cases.push({name:c.name,request:c.request,bundle:c.bundle,priorResponse:c.response,response:entry(file)});
}
assert.equal(cases.length,165);assert(!raster.invalid);
fs.writeFileSync(root+'/recent-regressions.json',JSON.stringify({format:'musteroffice.image-recent-regressions/1',previousEvidence:entry(previous),
 counts:{requests:165,textPages:124,geometry:41},cases},null,2));
console.log(JSON.stringify({requests:165,textPages:124,geometry:41}));
