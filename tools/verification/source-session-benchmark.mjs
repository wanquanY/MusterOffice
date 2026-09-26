/** Paired current-build one-shot vs prepared source owner, with pixel parity. */
import fs from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/source-session',product=JSON.parse(fs.readFileSync('.codex-work/source-playback/product.json'));
const wasm=createRequire(import.meta.url)('../../.codex-work/source-session/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const summary=v=>{const s=[...v].sort((a,b)=>a-b);return {samplesMs:v,medianMs:s[Math.floor(s.length/2)],p95Ms:s[Math.ceil(s.length*.95)-1]};};
const cases=[];
for(const name of ['image-text-3','masters-3','circle-0','text-3']){
 const c=product.cases.find(c=>c.name===name),input=fs.readFileSync(c.request.path,'utf8'),q=JSON.parse(input),source=fs.readFileSync(c.source.path),fonts=fs.readFileSync(c.fonts.path);
 const prepare=JSON.stringify({operation:'prepare',request:{page:q.page,binding:q.sample.binding}}),sample=JSON.stringify({operation:'render',sample:q.sample});
 const prep=[];
 for(let i=0;i<6;i++){const s=new wasm.PptxPlaybackSession(),start=performance.now();const p=JSON.parse(s.prepare(prepare,source,fonts,component,shaper));const elapsed=performance.now()-start;assert.equal(p.status,'prepared');s.free();if(i)prep.push(elapsed);}
 const owner=new wasm.PptxPlaybackSession(),p=JSON.parse(owner.prepare(prepare,source,fonts,component,shaper));assert.equal(p.status,'prepared');
 const raw={oneShot:[],retained:[]};
 for(let i=0;i<25;i++)for(const mode of i%2?['oneShot','retained']:['retained','oneShot']){
  const start=performance.now();const r=mode==='oneShot'?wasm.render_pptx_playback_page(input,source,fonts,component,shaper,component):owner.render(sample,component);
  const metadata=JSON.parse(r.metadata),pixels=Buffer.from(r.take_pixels()),elapsed=performance.now()-start;
  assert.equal(metadata.status,'rendered');assert.equal(sha(pixels),c.pixels.sha256);assert(!component.invalid);assert(!shaper.invalid);if(i>=5)raw[mode].push(elapsed);
 }
 owner.free();cases.push({name,request:c.request,source:c.source,fonts:c.fonts,viewport:[q.page.page.viewport.width,q.page.page.viewport.height],pixelSha256:c.pixels.sha256,preparation:p.info.preparation,prepare:summary(prep),oneShot:summary(raw.oneShot),retained:summary(raw.retained)});
}
const report={format:'musteroffice.source-session-cost/1',environment:{platform:os.platform(),arch:os.arch(),release:os.release(),cpu:os.cpus()[0].model,node:process.version},conditions:{warmup:5,samples:20,prepareWarmup:1,prepareSamples:5,order:'alternating paired order',modules:'warm',taskBuildsOrTestsConcurrent:false,otherSystemLoad:'uncontrolled',clock:'performance.now',includes:'Current build JSON entry, exact timeline sampling, per-frame native placements/path/paint/precision/scene compilation, real raster, metadata serialization and pixel transfer. One-shot also reopens/indexes source, parses timing, prepares fonts/decoded images/text.',excludes:'Cold startup, RSS, display scheduling, production host or IPC, target application acceptance, full installer/dependency closure'},cases};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(cases.map(c=>({name:c.name,prepare:c.prepare.medianMs,oneShot:c.oneShot.medianMs,retained:c.retained.medianMs,reductionPercent:100*(1-c.retained.medianMs/c.oneShot.medianMs)}))));
