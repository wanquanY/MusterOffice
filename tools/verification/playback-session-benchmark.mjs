/** Same current module and same pixels: per-call preparation vs retained plan. */
import fs from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
const root='.codex-work/playback-session',prior=JSON.parse(fs.readFileSync('.codex-work/playback-render/product.json'));
const wasm=createRequire(import.meta.url)('../../.codex-work/playback-session/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');let rasterMs=0;
const backend={raster(f){const start=performance.now();const r=component.raster(f);rasterMs+=performance.now()-start;return r;},invalidate(){component.invalidate();}};
const summary=v=>{const sorted=[...v].sort((a,b)=>a-b);return {samplesMs:v,medianMs:sorted[Math.floor(sorted.length/2)],p95Ms:sorted[Math.ceil(sorted.length*.95)-1]};};
const cases=[];
for(const name of ['rigid-1','grouped-4','grouped-1','nested-2']){
 const c=prior.cases.find(c=>c.name===name),input=fs.readFileSync(c.request.path,'utf8'),q=JSON.parse(input),p=q.playback;
 const prepare=JSON.stringify({operation:'prepare',request:{snapshot:p.snapshot,slide:p.slide,binding:p.binding,viewport:q.viewport,defaults:q.defaults}});
 const sample=JSON.stringify({operation:'render',sample:{binding:p.binding,at:p.at,history:p.history}});
 const owner=new wasm.PlaybackSession(),start=performance.now(),prepared=JSON.parse(owner.command(prepare)),prepareMs=performance.now()-start;assert.equal(prepared.status,'prepared');
 const raw={perCall:{total:[],raster:[],remaining:[]},retained:{total:[],raster:[],remaining:[]}};
 for(let i=0;i<40;i++)for(const mode of i%2?['retained','perCall']:['perCall','retained']){
  rasterMs=0;const start=performance.now();const r=mode==='retained'?owner.render(sample,backend):wasm.render_playback_page(input,backend);
  const meta=JSON.parse(r.metadata),pixels=Buffer.from(r.take_pixels()),elapsed=performance.now()-start;
  assert.equal(meta.status,'rendered');assert.equal(sha(pixels),c.pixels.sha256);assert(!component.invalid);
  if(i>=10){raw[mode].total.push(elapsed);raw[mode].raster.push(rasterMs);raw[mode].remaining.push(elapsed-rasterMs);}
 }
 assert.equal(JSON.parse(owner.command(JSON.stringify({operation:'dispose',binding:p.binding}))).status,'disposed');owner.free();
 cases.push({name,request:c.request,pixelSha256:c.pixels.sha256,requestBytes:Buffer.byteLength(input),sampleBytes:Buffer.byteLength(sample),prepareBytes:Buffer.byteLength(prepare),prepareMs,...Object.fromEntries(Object.entries(raw).map(([name,parts])=>[name,Object.fromEntries(Object.entries(parts).map(([name,v])=>[name,summary(v)]))]))});
}
const result={format:'musteroffice.playback-session-cost/1',environment:{platform:os.platform(),arch:os.arch(),release:os.release(),cpu:os.cpus()[0].model,node:process.version},conditions:{viewport:'320x240',warmup:10,samples:30,order:'alternating paired order',modules:'warm',taskBuildsOrTestsConcurrent:false,otherSystemLoad:'uncontrolled',clock:'performance.now',includes:'Sample JSON parse, timeline/placement/path/device computation, raster callback, result metadata and pixel transfer. perCall also includes snapshot parse/validation and plan creation.',prepare:'One observed preparation outside sampling timing; not a stable startup statistic.',excludes:'Cold start, JS request serialization, complete font/media/dependency closure, RSS, retained geometry/compositor, display scheduling, target applications, product host overhead'},cases};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(cases.map(c=>({name:c.name,perCall:c.perCall.total.medianMs,retained:c.retained.total.medianMs,ratio:c.perCall.total.medianMs/c.retained.total.medianMs,sampleBytes:c.sampleBytes,requestBytes:c.requestBytes}))));
