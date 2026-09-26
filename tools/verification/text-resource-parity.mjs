import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import skiaFactory from '../../.codex-work/skia/mo-skia.mjs';
const root='.codex-work/text-resource-diagnostics';
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const read=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const fixtures=JSON.parse(fs.readFileSync(root+'/fixtures.json'));
const previous=JSON.parse(read(fixtures.previousEvidence));
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
// Deliberate additive changes to exactly two prior error replies; everything
// else, including all prior page pixels, must remain byte-identical.
let additions=0;
for(const c of previous.parityEvidence.cases){
 const r=render('prior-'+c.name,read(c.request).toString(),read(c.source),read(c.fonts));
 const expected=read(c.response).toString();
 if(['later-underline','missing-paint'].includes(c.name)){
  assert.ok(r.response.error.paintLocation);const normalized=structuredClone(r.response);delete normalized.error.paintLocation;
  assert.deepEqual(normalized,JSON.parse(expected));additions++;
 }else assert.equal(read(r.record.response).toString(),expected,c.name);
 if(c.pixels)assert.deepEqual(r.pixels,read(c.pixels));
 r.record.priorResponse=c.response;
}
assert.equal(additions,2);
const template=JSON.parse(read(fixtures.base.request)),fonts=read(fixtures.base.fonts),newCases=[];
for(const c of fixtures.cases){
 const source=read(c.source),q=structuredClone(template);q.page.expectedSourceSha256=sha(source);
 if(c.change==='missingFamily')q.fonts.typefaces=[];
 if(c.change==='missingRegular')q.fonts.typefaces[0].regular=null;
 const failed=render(c.name,q,source,fonts);assert.equal(failed.response.status,'error');assert.equal(textCalls,0);assert.equal(invalidated,false);
 const e=failed.response.error;assert.equal(e.stage,'page');assert.equal(e.error.location.object,42);
 const observation={...c,failed:failed.record};
 if(c.paint){
  assert.equal(e.detail.kind,c.paint);assert.equal(e.paintLocation.paragraph,c.paragraph);assert.equal(e.paintLocation.run,c.run);
  if(c.recoverColor)q.page.colorContext.systemColors.window=[32,112,192];
 }else{
  assert.equal(e.error.code,'RESOURCE_REQUIRED');assert.equal(e.detail.kind,'fontSelection');
  const f=e.detail.failure;assert.equal(f.sourceSha256,sha(source));assert.equal(f.paragraph,c.paragraph);assert.equal(f.selection.reason,c.reason);assert.equal(f.selection.fontStyle,c.style);assert.equal(f.selection.typeface,'MusterOffice Synthetic');assert.deepEqual(f.uses.map(u=>u.run),c.runs);
  // The explicit resource catalog supplies verified faces/instances. Use the
  // diagnostic's family/style, never its message or an inferred replacement.
  const required=f.selection,catalog=structuredClone(template.fonts.typefaces[0]);assert.equal(catalog.typeface,required.typeface);
  const slot=required.fontStyle;assert.ok(['regular','bold','italic','boldItalic'].includes(slot));
  // The owned font exposes an actual wght axis. The catalog has no italic
  // instance, so that prerequisite remains unresolved in this probe.
  if(slot==='bold')catalog.bold={face:0,variations:[{tag:'wght',value1616:700*65536}]};
  const existing=q.fonts.typefaces.find(x=>x.typeface===required.typeface);
  if(catalog[slot]){if(existing)existing[slot]=catalog[slot];else q.fonts.typefaces.push(catalog);}
 }
 if((!c.paint&&c.style!=='italic')||c.recoverColor){
  const recovered=render(c.name+'-recovered',q,source,fonts);assert.equal(recovered.response.status,'rendered');assert.ok(textCalls>0);assert.equal(invalidated,false);
  assert.equal(sha(read(c.source)),sha(source));observation.recovered=recovered.record;
  if(['missing-family','missing-regular','system-color'].includes(c.name))assert.deepEqual(recovered.pixels,read(fixtures.base.pixels));
 }
 newCases.push(observation);
}
// Current CLI publication checks cover both actionable failures and recovery.
const cli=[];
for(const c of newCases.flatMap(c=>[c.failed,...(c.recovered?[c.recovered]:[])])){
 const output=`${root}/${c.name}.cli.rgba`;if(fs.existsSync(output))fs.unlinkSync(output);
 const r=spawnSync('target/debug/mo-cli',['render-pptx-text-page',c.request.path,c.source.path,c.fonts.path,output],{timeout:60000,encoding:'utf8',maxBuffer:8*1024*1024});assert.equal(r.status,0,r.stderr);assert.equal(r.stdout.trimEnd(),read(c.response).toString());
 assert.equal(fs.existsSync(output),!!c.pixels);if(c.pixels)assert.deepEqual(fs.readFileSync(output),read(c.pixels));cli.push({name:c.name,published:!!c.pixels});
}
const artifacts=['target/debug/mo-cli','target/debug/mo-raster-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`,'.codex-work/harfbuzz/release/mo-hb.wasm','.codex-work/skia/mo-skia.wasm','.codex-work/text-component/index.js','.codex-work/raster-component/index.js'].map(entry);
const counts={priorRequests:27,additiveDiagnostics:2,newRequests:records.length-27,resourceRecoveries:newCases.filter(c=>c.recovered).length,cliRequests:cli.length};
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.text-resource-diagnostics-parity/1',fixtures:entry(root+'/fixtures.json'),previousEvidence:fixtures.previousEvidence,artifacts,counts,cases:records,newCases,cli},null,2)+'\n');console.log(JSON.stringify(counts));
