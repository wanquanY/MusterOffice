import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import skiaFactory from '../../.codex-work/skia/mo-skia.mjs';
const root='.codex-work/character-spacing';
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const previousPath='docs/reviews/evidence/2026-09-25-paragraph-spacing-verification.json';
assert.equal(entry(previousPath).sha256,'834cfbbbb98b3a064d93159f70e973ef8db86a0fa88d91c63266a3e3fa880ccc');
const previous=JSON.parse(fs.readFileSync(previousPath));
const wasm=createRequire(import.meta.url)(`../../${root}/wasm-node/mo_wasm.js`);
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const raster=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/skia/mo-skia.wasm')));
let textCalls=0,rasterCalls=0,invalidated=false;
const text={...Object.fromEntries(['shapeBatch','measureBatch','outlineBatch'].map(name=>[name,(...a)=>{textCalls++;return shaper[name](...a);}])),invalidate(){invalidated=true;shaper.invalidate();}};
const paint={raster(words){rasterCalls++;return raster.raster(words);},invalidate(){invalidated=true;raster.invalidate();}};
const records=[];
function render(name,q,source,fonts){
 const json=typeof q==='string'?q:JSON.stringify(q),head=Buffer.alloc(12);head.writeUInt32LE(Buffer.byteLength(json));head.writeUInt32LE(source.length,4);head.writeUInt32LE(fonts.length,8);
 const n=spawnSync('target/debug/mo-raster-worker',['--pptx-text-page'],{input:Buffer.concat([head,Buffer.from(json),source,fonts]),env:{},maxBuffer:80*1024*1024,timeout:60000});assert.equal(n.status,0,n.stderr?.toString());
 const meta=n.stdout.readUInt32LE(),pixels=n.stdout.subarray(8+meta),raw=n.stdout.subarray(8,8+meta).toString();assert.equal(n.stdout.readUInt32LE(4),pixels.length);
 textCalls=0;rasterCalls=0;const w=wasm.render_pptx_text_page(json,source,fonts,text,paint);assert.equal(w.metadata,raw,name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,name);
 const response=JSON.parse(raw);if(response.status==='error'){assert.equal(pixels.length,0);assert.equal(rasterCalls,0);}else{assert.equal(rasterCalls,1);assert.equal(sha(pixels),response.info.page.scene.raster.sha256);}
 const prefix=`${root}/${name}`;for(const [suffix,value]of [['request.json',json],['response.json',raw],['pptx',source],['fonts.bin',fonts]])fs.writeFileSync(prefix+'.'+suffix,value);
 const record={name,request:entry(prefix+'.request.json'),response:entry(prefix+'.response.json'),source:entry(prefix+'.pptx'),fonts:entry(prefix+'.fonts.bin'),textCalls,rasterCalls};
 if(pixels.length){fs.writeFileSync(prefix+'.rgba',pixels);record.pixels=entry(prefix+'.rgba');}
 records.push(record);return {response,pixels,record};
}
for(const c of previous.parityEvidence.cases){
 const r=render('prior-'+c.name,read(c.request).toString(),read(c.source),read(c.fonts));
 assert.equal(read(r.record.response).toString(),read(c.response).toString(),c.name);
 if(c.pixels)assert.deepEqual(r.pixels,read(c.pixels));r.record.priorResponse=c.response;
}
const fonts=fs.readFileSync('fixtures/fonts/owned-tracking.ttf');
const manifest=JSON.parse(fs.readFileSync('fixtures/fonts/tracking-manifest.json'));
const newCases=[],outcomes=new Map();
for(const name of fs.readdirSync(root+'/cases').filter(n=>n.endsWith('.page.json')).map(n=>n.slice(0,-10)).sort()){
 const q={profile:'drawingml-solid-text-page-q32-draft-v1',page:JSON.parse(fs.readFileSync(root+'/cases/'+name+'.page.json')),fonts:manifest};
 const source=fs.readFileSync(root+'/cases/'+name+'.pptx');const r=render(name,q,source,fonts);
 if(fs.existsSync(root+'/cases/'+name+'.plan.json')){
  assert.equal(r.response.status,'rendered');assert.deepEqual(r.pixels,fs.readFileSync(root+'/cases/'+name+'.rgba'));
  r.record.libraryPlan=entry(root+'/cases/'+name+'.plan.json');
 }else{
  assert.equal(r.response.status,'error');const e=r.response.error;
  assert.equal(e.error.location.object,42);assert.equal(textCalls,0);
  const expected={lexical:'native tracking lexical bytes',range:'native tracking range','cluster-conflict':'GraphemeStyleConflict'};
  assert.ok(expected[name]);assert.ok(e.error.message.includes(expected[name]),e.error.message);

 }
 newCases.push(r.record);outcomes.set(name,{q,source,...r});
}
assert.equal(newCases.length,18);
const cli=[];
for(const c of newCases){
 const output=root+'/'+c.name+'.cli.rgba';if(fs.existsSync(output))fs.unlinkSync(output);
 const r=spawnSync('target/debug/mo-cli',['render-pptx-text-page',c.request.path,c.source.path,c.fonts.path,output],{timeout:60000,encoding:'utf8',maxBuffer:8*1024*1024});
 assert.equal(r.status,0,r.stderr);assert.equal(r.stdout.trimEnd(),read(c.response).toString());assert.equal(fs.existsSync(output),!!c.pixels);if(c.pixels)assert.deepEqual(fs.readFileSync(output),read(c.pixels));
 cli.push({name:c.name,...(c.pixels?{pixels:entry(output)}:{})});
}
assert.equal(invalidated,false);
const artifacts=['target/debug/mo-cli','target/debug/mo-raster-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`,'.codex-work/harfbuzz/release/mo-hb.wasm','.codex-work/skia/mo-skia.wasm','.codex-work/text-component/index.js','.codex-work/raster-component/index.js'].map(entry);
const counts={priorRequests:106,changedDiagnostics:0,newRequests:newCases.length,cliRequests:cli.length};
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.character-spacing-parity/1',previousEvidence:entry(previousPath),artifacts,counts,cases:records,cli},null,2)+'\n');console.log(JSON.stringify(counts));
