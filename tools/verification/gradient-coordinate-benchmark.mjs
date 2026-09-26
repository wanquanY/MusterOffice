/** Public PPTX-to-pixels API, same frozen Rust/TS host, old/current C++ WASM. */
import fs from 'node:fs';
import {compareRenderedMetadata} from './gradient_coordinate_delta.mjs';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/gradient-coordinates';
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const rust='.codex-work/transform-edit/wasm-node/mo_wasm';
const wasm=createRequire(import.meta.url)(path.resolve(rust+'.js'));
const hbm=new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'));
const variants=[];
for(const [name,dir] of [['before','.codex-work/elliptic-fast/component'],['after',root+'/component']]){
 const factory=(await import(pathToFileURL(path.resolve(dir+'/mo-skia.mjs')))).default;
 variants.push({name,component:await RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(dir+'/mo-skia.wasm'))),
  shaper:await ShapingComponent.create(hbFactory,hbm),artifacts:[entry(dir+'/mo-skia.mjs'),entry(dir+'/mo-skia.wasm')]});
}
const old=JSON.parse(fs.readFileSync('.codex-work/elliptic-source/source-parity.json'));
const names=['center-point','focus-area','focus-line','off-center','equal-width-offset','rotated-ellipse','tiled','expanded-focus'];
const results=[],summary=s=>({samplesMs:s,medianMs:s.toSorted((a,b)=>a-b)[12],p95Ms:s.toSorted((a,b)=>a-b)[23]});
for(const name of names){
 const c=old.cases.find(c=>c.name==='new/'+name);assert(c);
 const source=load(c.source),json=load(c.request).toString(),fonts=load(c.fonts),pixels=load(c.pixels),metadata=load(c.response).toString();
 const result={name,source:c.source,request:c.request,fonts:c.fonts,pixels:c.pixels,response:c.response};
 for(const v of variants){
  const total=[],raster=[],other=[];let firstPixels=null;
  let rasterMs=0,rasters=0,decodes=0;
  const decoder={decodeImage(b){decodes++;return v.component.decodeImage(b);},invalidate(){v.component.invalidate();}};
  const backend={rasterImages(f,b){const start=process.hrtime.bigint();const r=v.component.rasterImages(f,b);rasterMs+=Number(process.hrtime.bigint()-start)/1e6;rasters++;return r;},invalidate(){v.component.invalidate();}};
  for(let i=0;i<28;++i){
   rasterMs=0;rasters=0;decodes=0;
   const start=process.hrtime.bigint(),w=wasm.render_pptx_resource_page(json,source,fonts,decoder,v.shaper,backend);
   const actualMetadata=w.metadata,actual=Buffer.from(w.take_pixels()),elapsed=Number(process.hrtime.bigint()-start)/1e6;
   compareRenderedMetadata(actualMetadata,metadata,actual,pixels);if(v.name==='before')assert.deepEqual(actual,pixels,name);if(firstPixels===null)firstPixels=actual;else assert.deepEqual(actual,firstPixels,name);assert.equal(rasters,1);assert.equal(decodes,0);
   if(i>=3){total.push(elapsed);raster.push(rasterMs);other.push(elapsed-rasterMs);}
  }
  result[v.name]={pixelSha256:sha(firstPixels),total:summary(total),rasterCallback:summary(raster),remaining:summary(other)};
 }
 result.speedup=result.before.total.medianMs/result.after.total.medianMs;results.push(result);
 console.log(JSON.stringify({name,beforeMs:result.before.total.medianMs,afterMs:result.after.total.medianMs,speedup:result.speedup}));
}
fs.writeFileSync(root+'/pages-benchmark.json',JSON.stringify({format:'musteroffice.gradient-coordinate-pages-benchmark/1',
 environment:{platform:os.platform(),release:os.release(),arch:os.arch(),cpu:os.cpus()[0].model,logicalCpus:os.cpus().length,memoryBytes:os.totalmem(),node:process.version},
 policy:{warmups:3,samples:25,viewport:'400 x 300',cache:'Warm modules and hosts; input bytes loaded before timing. Each call reparses PPTX, prepares and renders page. No document/frame/pixel cache; no GC forcing.',
  scope:'Synchronous public Rust WASM PPTX-to-pixels call including returned pixel copy; validation assertions after timing. Raster callback is a nested interval, remaining is elapsed minus callback. Not Native product, browser worker, cold startup, RSS, media or installer.',
  fonts:'Empty explicit manifest; selected gradient-only source pages have no text.',images:'No encoded images or image decode calls.',
  order:'Sequential cases, before then after. Other system load uncontrolled.'},
 variants:variants.map(({name,artifacts})=>({name,artifacts})),sharedArtifacts:[entry(rust+'.js'),entry(rust+'_bg.wasm'),entry('.codex-work/elliptic-source/ts-raster/index.js'),entry('.codex-work/text-component/index.js'),entry('.codex-work/harfbuzz/release/mo-hb.wasm')],results},null,2)+'\n');
