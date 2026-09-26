/** Native source timing, decoder/shaper and shared raster all execute for real. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/source-session/oneshot',out=root+'/frames';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/source-session/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
let rasters=0,decodes=0,shapes=0,frame=null;
const raster={raster(f){rasters++;frame=f.slice();return component.raster(f);},rasterImages(f,b){rasters++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
const text={shapeBatch(...args){shapes++;return shaper.shapeBatch(...args);},measureBatch(...args){shapes++;return shaper.measureBatch(...args);},outlineBatch(...args){shapes++;return shaper.outlineBatch(...args);},invalidate(){shaper.invalidate();}};
function native(q,source,fonts,mode){
 const h=Buffer.alloc(12);h.writeUInt32LE(q.length);h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);
 const r=spawnSync('target/debug/mo-raster-worker',[mode],{env:{},input:Buffer.concat([h,q,source,fonts]),timeout:60000,maxBuffer:80<<20});
 assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0);const ml=r.stdout.readUInt32LE(),pl=r.stdout.readUInt32LE(4);assert.equal(r.stdout.length,ml+pl+8);
 return {metadata:r.stdout.subarray(8,8+ml).toString(),pixels:r.stdout.subarray(8+ml)};
}
function pair(name,request,source,fonts,animated=true){
 const q=load(request),s=load(source),f=load(fonts),before=sha(s);const n=native(q,s,f,animated?'--pptx-playback-page':'--pptx-resource-page');
 rasters=0;decodes=0;shapes=0;frame=null;
 const w=(animated?wasm.render_pptx_playback_page:wasm.render_pptx_resource_page)(q.toString(),s,f,decoder,text,raster);
 const metadata=w.metadata,pixels=Buffer.from(w.take_pixels());assert.equal(metadata,n.metadata,name);assert.deepEqual(pixels,n.pixels,name);assert.equal(sha(s),before);assert(!component.invalid);assert(!shaper.invalid);
 const response=JSON.parse(metadata);if(response.status==='rendered'){assert.equal(rasters,1);const r=animated?response.info.page.page.scene.raster:response.info.page.scene.raster;assert.equal(sha(pixels),r.sha256);}else{assert.equal(rasters,0);assert.equal(pixels.length,0);}
 return {metadata,pixels,response,rasters,decodes,shapes,frame};
}
const fixturePath='.codex-work/source-playback/fixtures.json',fixtures=JSON.parse(fs.readFileSync(fixturePath)),records=[];let controls=0;
for(const c of fixtures.cases){
 const r=pair(c.name,c.request,c.source,c.fonts);const p=out+'/'+records.length;
 if(c.error){assert.equal(r.response.status,'error',c.name);const error=r.response.error;assert.equal(error.stage==='timing'?error.error.code:error.error.error.code,c.error,c.name);if(c.noComponentCalls)assert.equal(r.decodes+r.shapes,0,c.name);}
 else{
  assert.equal(r.response.status,'rendered',c.name);const q=JSON.parse(load(c.request));assert.deepEqual(r.response.info.playback.evaluated.state.rotations,c.expectedRotations,c.name+' independent Fraction');assert.deepEqual(r.response.info.playback.objectBindings,c.expectedBindings,c.name+' native identity');assert.deepEqual(r.response.info.playback.evaluated.state.binding,q.sample.binding);assert.deepEqual(r.response.info.playback.evaluated.state.time,q.sample.at);assert.equal(r.response.info.playback.sourceSha256,c.source.sha256);
 }
 const record={name:c.name,request:c.request,source:c.source,fonts:c.fonts,response:put(p+'.response.json',r.metadata),pixels:put(p+'.rgba',r.pixels),rasters:r.rasters,decodes:r.decodes,shapes:r.shapes,...(r.frame?{frame:put(p+'.frame',Buffer.from(r.frame.buffer))}:{})};
 if(c.staticControl){const s=pair(c.name+' static',c.staticControl.request,c.staticControl.source,c.fonts,false);assert.equal(s.response.status,'rendered');assert.deepEqual(s.pixels,r.pixels,c.name+' independently rewritten transform pixels');assert.deepEqual(s.frame,r.frame,c.name+' compiled frame');record.staticControl={...c.staticControl,response:put(p+'.static.response.json',s.metadata),pixelSha256:sha(s.pixels),frameSha256:sha(Buffer.from(s.frame.buffer)),rasters:s.rasters,decodes:s.decodes,shapes:s.shapes};controls++;}
 const frozen=JSON.parse(fs.readFileSync('.codex-work/source-playback/product.json')).cases.find(r=>r.name===c.name);
 assert.equal(r.metadata,load(frozen.response).toString());assert.deepEqual(r.pixels,load(frozen.pixels));
 records.push(record);
}
for(const name of ['image-text','group-image','masters','circle','text']){
 const a=records.find(c=>c.name===name+'-1'),b=records.find(c=>c.name===name+'-6');assert.deepEqual(load(a.pixels),load(b.pixels));assert.equal(load(a.response).toString(),load(b.response).toString());
}
const positive=records.find(c=>c.name==='image-text-1'),negative=records.find(c=>c.name==='click');const dir=fs.mkdtempSync(root+'/cli-'),path=dir+'/frame.rgba';
const invoke=(c,target)=>spawnSync('target/debug/mo-cli',['render-pptx-playback-page',c.request.path,c.source.path,c.fonts.path,target],{env:{},timeout:60000,maxBuffer:80<<20});
let n=invoke(positive,path);assert.equal(n.status,0,n.stderr.toString());assert.deepEqual(fs.readFileSync(path),load(positive.pixels));const published=entry(path);n=invoke(positive,path);assert.notEqual(n.status,0);assert.deepEqual(entry(path),published);
const absent=dir+'/failure.rgba',fail=invoke(negative,absent);assert.equal(fail.status,0);assert.equal(JSON.parse(fail.stdout).status,'error');assert(!fs.existsSync(absent));
const report={format:'musteroffice.source-playback-parity/1',fixtures:entry(fixturePath),cases:records,pairedCalls:records.length+controls,renderedFrames:records.filter(c=>c.rasters===1).length,independentNativeTransformControls:controls,seekRecovery:true,cli:{published,overwriteExit:n.status,failureOutputAbsent:true,response:put(dir+'/failure.json',fail.stdout)},artifacts:[entry('target/debug/mo-cli'),entry('target/debug/mo-raster-worker'),entry('.codex-work/source-session/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({pairedCalls:report.pairedCalls,renderedFrames:report.renderedFrames,independentNativeTransformControls:controls}));
