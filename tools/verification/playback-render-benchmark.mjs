/** Warm public WASM sampling cost; this is not a retained playback benchmark. */
import fs from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
const root='.codex-work/playback-render',report=JSON.parse(fs.readFileSync(root+'/product.json'));
const wasm=createRequire(import.meta.url)('../../.codex-work/playback-render/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');let rasterMs=0;
const backend={raster(f){const start=performance.now();const r=component.raster(f);rasterMs+=performance.now()-start;return r;},invalidate(){component.invalidate();}};
const summary=v=>{const s=[...v].sort((a,b)=>a-b);return {samplesMs:v,medianMs:s[Math.floor(s.length/2)],p95Ms:s[Math.ceil(s.length*.95)-1]};};
function measure(fn,expected){
 const total=[],raster=[],remaining=[];
 for(let i=0;i<30;i++){
  rasterMs=0;const start=performance.now();const result=fn(),meta=JSON.parse(result.metadata),pixels=Buffer.from(result.take_pixels()),elapsed=performance.now()-start;assert.equal(meta.status,'rendered');assert.equal(sha(pixels),expected);assert(!component.invalid);
  if(i>=5){total.push(elapsed);raster.push(rasterMs);remaining.push(elapsed-rasterMs);}
 }
 return {total:summary(total),raster:summary(raster),remaining:summary(remaining),pixelSha256:expected};
}
const records=[];
for(const name of ['rigid-1','grouped-4','grouped-1','nested-2']){
 const c=report.cases.find(c=>c.name===name),q=fs.readFileSync(c.request.path,'utf8');const record={name,request:c.request,animated:measure(()=>wasm.render_playback_page(q,backend),c.pixels.sha256)};
 if(c.integerStaticReference){const staticQ=JSON.parse(fs.readFileSync(c.integerStaticReference.path));delete staticQ.page.document.timelines;const staticJson=JSON.stringify(staticQ);record.staticControl=measure(()=>wasm.render_page(staticJson,backend),c.pixels.sha256);}
 records.push(record);
}
const result={format:'musteroffice.playback-public-cost/1',environment:{platform:os.platform(),arch:os.arch(),release:os.release(),cpu:os.cpus()[0].model,node:process.version},conditions:{viewport:'320x240',warmup:5,samples:25,modules:'warm',taskBuildsOrTestsConcurrent:false,otherSystemLoad:'uncontrolled',clock:'performance.now',includes:'JSON parse, snapshot validation, plan creation, timeline/placement/path/device computation, raster callback, result metadata and pixel transfer',staticControl:'Same integral sampled placement baked into author transforms with timeline removed. Equal pixels verified.',excludes:'Cold start, complete dependency/font/media closure, RSS, real retained compositor, display scheduling and target applications'},cases:records};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(records.map(c=>({name:c.name,animatedMedianMs:c.animated.total.medianMs,staticMedianMs:c.staticControl?.total.medianMs}))));
