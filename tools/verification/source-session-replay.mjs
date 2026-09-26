/** Replay the most recent frozen static rendering outputs through the new Rust build. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/source-session',out=root+'/replay';fs.mkdirSync(out,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const wasm=createRequire(import.meta.url)('../../.codex-work/source-session/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
let frame=null,rasters=0,decodes=0;
const raster={raster(f){rasters++;frame=f.slice();return component.raster(f);},rasterImages(f,b){rasters++;frame=f.slice();return component.rasterImages(f,b);},invalidate(){component.invalidate();}};
const decoder={decodeImage(b){decodes++;return component.decodeImage(b);},invalidate(){component.invalidate();}};
const previousPath='.codex-work/playback-session/replay.json',previous=JSON.parse(fs.readFileSync(previousPath)),records=[];
for(const c of previous.cases){
 const json=load(c.request),source=c.source?load(c.source):null,fonts=c.fonts?load(c.fonts):null;
 const isScene=c.kind==='runtime'&&(c.name.startsWith('scene-')||c.name.startsWith('prior-scene-'));
 const h=Buffer.alloc(source?12:4);h.writeUInt32LE(json.length);if(source){h.writeUInt32LE(source.length,4);h.writeUInt32LE(fonts.length,8);}
 const args=source?['--pptx-resource-page']:isScene?['--scene']:[];
 const n=spawnSync('target/debug/mo-raster-worker',args,{input:Buffer.concat(source?[h,json,source,fonts]:[h,json]),env:{},timeout:60000,maxBuffer:80*1024*1024});assert.equal(n.status,0,c.name+': '+n.stderr.toString());assert.equal(n.stderr.length,0);
 const ml=n.stdout.readUInt32LE(),pl=n.stdout.readUInt32LE(4);assert.equal(n.stdout.length,8+ml+pl);
 const metadata=n.stdout.subarray(8,8+ml).toString(),pixels=n.stdout.subarray(8+ml);
 frame=null;rasters=0;decodes=0;
 const w=source?wasm.render_pptx_resource_page(json.toString(),source,fonts,decoder,shaper,raster):isScene?wasm.render_scene(json.toString(),raster):wasm.render_paths(json.toString(),raster);
 assert.equal(w.metadata,metadata,c.name);assert.deepEqual(Buffer.from(w.take_pixels()),pixels,c.name);assert(!component.invalid);
 assert.equal(metadata,load(c.response).toString(),c.name+' frozen response');assert.deepEqual(pixels,c.pixels?load(c.pixels):Buffer.alloc(0),c.name+' frozen pixels');
 if(c.frame)assert.deepEqual(Buffer.from(frame.buffer),load(c.frame));else assert.equal(frame,null);
 assert.equal(rasters,c.rasters);assert.equal(decodes,c.decodes);
 records.push({name:c.name,kind:c.kind,request:c.request,...(source?{source:c.source,fonts:c.fonts}:{}),response:put(out+'/'+records.length+'.json',metadata),pixelsSha256:sha(pixels),...(c.pixels?{pixels:c.pixels}:{}),...(c.frame?{frame:c.frame}:{}),rasters,decodes});
}
assert.equal(records.length,355);
const timingPath='.codex-work/timeline/product.json',timing=JSON.parse(fs.readFileSync(timingPath)),timingCases=[];
for(const c of timing.cases){
 const json=load(c.request).toString(),source=c.source?load(c.source):null;
 const args=c.operation==='dispatch'?[]:c.operation==='evaluate'?['evaluate-timeline',c.request.path]:['pptx-timing',c.request.path,c.source.path];
 const n=spawnSync('target/debug/mo-cli',args,{input:c.operation==='dispatch'?json:undefined,env:{},timeout:60000,maxBuffer:40<<20});assert.equal(n.status,0,n.stderr.toString());assert.equal(n.stderr.length,0);
 const response=n.stdout.toString().trimEnd(),w=c.operation==='dispatch'?wasm.dispatch_json(json):c.operation==='evaluate'?wasm.evaluate_timeline(json):wasm.inspect_pptx_timing(json,source);
 assert.equal(response,w,c.name);assert.equal(response,load(c.response).toString(),c.name+' frozen timeline response');timingCases.push(c);
}
assert.equal(timingCases.length,246);

fs.writeFileSync(root+'/replay.json',JSON.stringify({format:'musteroffice.playback-static-replay/1',previous:entry(previousPath),pairedCalls:records.length,cases:records,timelineCalls:timingCases.length,timelineCases:timingCases,timelinePrevious:entry(timingPath),artifacts:[entry('target/debug/mo-raster-worker'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]},null,2)+'\n');
console.log(JSON.stringify({pairedCalls:records.length,timelineCalls:timingCases.length,pixelsAndMetadataUnchanged:true}));
