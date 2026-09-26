/** Per-call native source sampling cost; source resource caches are not retained yet. */
import fs from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/elliptic-source/ts-raster/index.js';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import skiaFactory from '../../.codex-work/gradient-coordinates/component/mo-skia.mjs';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/source-playback',product=JSON.parse(fs.readFileSync(root+'/product.json'));
const wasm=createRequire(import.meta.url)('../../.codex-work/source-playback/wasm-node/mo_wasm.js');
const component=await RasterComponent.create(skiaFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/gradient-coordinates/component/mo-skia.wasm')));
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const sha=b=>createHash('sha256').update(b).digest('hex');
const summary=v=>{const s=[...v].sort((a,b)=>a-b);return {samplesMs:v,medianMs:s[Math.floor(s.length/2)],p95Ms:s[Math.ceil(s.length*.95)-1]};};
const cases=[];
for(const name of ['image-text-3','masters-3','circle-0','text-3']){
 const c=product.cases.find(c=>c.name===name);assert(c.staticControl);
 const input=fs.readFileSync(c.request.path,'utf8'),source=fs.readFileSync(c.source.path),fonts=fs.readFileSync(c.fonts.path),staticInput=fs.readFileSync(c.staticControl.request.path,'utf8'),staticSource=fs.readFileSync(c.staticControl.source.path);
 const raw={animated:[],static:[]};
 for(let i=0;i<25;i++)for(const mode of i%2?['animated','static']:['static','animated']){
  const start=performance.now();const r=mode==='animated'?wasm.render_pptx_playback_page(input,source,fonts,component,shaper,component):wasm.render_pptx_resource_page(staticInput,staticSource,fonts,component,shaper,component);
  const metadata=JSON.parse(r.metadata),pixels=Buffer.from(r.take_pixels()),elapsed=performance.now()-start;
  assert.equal(metadata.status,'rendered');assert.equal(sha(pixels),c.pixels.sha256);assert(!component.invalid);assert(!shaper.invalid);if(i>=5)raw[mode].push(elapsed);
 }
 const q=JSON.parse(input);cases.push({name,request:c.request,source:c.source,fonts:c.fonts,viewport:[q.page.page.viewport.width,q.page.page.viewport.height],pixelSha256:c.pixels.sha256,animated:summary(raw.animated),static:summary(raw.static)});
}
const report={format:'musteroffice.source-playback-cost/1',environment:{platform:os.platform(),arch:os.arch(),release:os.release(),cpu:os.cpus()[0].model,node:process.version},conditions:{warmup:5,samples:20,order:'alternating paired order',modules:'warm',taskBuildsOrTestsConcurrent:false,otherSystemLoad:'uncontrolled',clock:'performance.now',includes:'Per-call source parse/index, font preparation, timing parse/evaluation (animated), placements, geometry, image decode, text metrics/shaping/outlines, certified rendering, metadata and pixel transfer',static:'Independently rewritten integral native transforms, timing removed. Exact same pixels verified.',excludes:'Cold start, RSS, complete dependency/font/media closure, retained source resource session, display scheduling, target applications and product host overhead'},cases};
fs.writeFileSync(root+'/benchmark.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(cases.map(c=>({name:c.name,animated:c.animated.medianMs,static:c.static.medianMs}))));
