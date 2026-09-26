// Focused warm in-memory WASM batch measurement, not paragraph/desktop latency.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {performance} from 'node:perf_hooks';
import os from 'node:os';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const record=JSON.parse(readFileSync('fixtures/fonts/upstream.json')).find(r=>r.family==='notosanssc'&&r.name.endsWith('.ttf'));
const font=readFileSync(`.codex-work/font-corpus/${record.family}/${record.name}`);assert.equal(sha(font),record.sha256);
const cpp=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm'),compiled=new WebAssembly.Module(cpp);
const component=await ShapingComponent.create(factory,compiled);
const text='中文排版字体资源';
const items=[...text].map((_,start)=>({start,end:start+1,direction:'leftToRight',script:'Hani',language:'zh',features:[],beginningOfText:start===0,endOfText:start===[...text].length-1,suppressDottedCircle:false,maxGlyphs:32,candidates:[{font:0,variations:[]}]}));
const fonts=[{expectedSha256:record.sha256,faceIndex:0,offset:'0',byteLength:String(font.length)}];
const cascadeRequest=JSON.stringify({text,fonts,items});
const singles=items.map(item=>JSON.stringify({text,expectedSha256:record.sha256,faceIndex:0,runs:[{
  start:item.start,end:item.end,direction:item.direction,script:item.script,language:item.language,features:[],variations:[],maxGlyphs:32,clusterLevel:'monotoneGraphemes',
  flags:{beginningOfText:item.beginningOfText,endOfText:item.endOfText,ignorables:'default',suppressDottedCircle:false,unsafeToConcat:true,safeToInsertTatweel:false}
}]}));
const baseline=()=>singles.map(json=>wasm.shape_text(json,font,component));
const reused=()=>wasm.shape_cascade(cascadeRequest,font,component);
function compare(a,b) {
  const result=JSON.parse(b);assert.equal(result.status,'evaluated');assert.equal(result.result.verifiedFaces,1);
  for(let i=0;i<a.length;i++) {const r=JSON.parse(a[i]);assert.equal(r.status,'shaped');assert.equal(result.result.items[i].status,'selected');assert.deepEqual(r.text,result.result.items[i].shaped);}
}
compare(baseline(),reused()); // Warm modules and allocator; not a font-result cache.
const measurements={singleCalls:[],cascadeCall:[]};
for(let round=0;round<9;round++) {
  let a,b;
  for(const mode of round%2?['cascadeCall','singleCalls']:['singleCalls','cascadeCall']) {
    const start=performance.now(),value=mode==='singleCalls'?baseline():reused(),ms=performance.now()-start;
    measurements[mode].push(ms);if(mode==='singleCalls')a=value;else b=value;
  }
  compare(a,b);
}
component.invalidate();
const summary=v=>{const a=[...v].sort((x,y)=>x-y);return {medianMs:a[Math.floor(a.length/2)],minimumMs:a[0],maximumMs:a.at(-1),samplesMs:v};};
const result={format:'musteroffice.font-cascade-benchmark/1',environment:{platform:os.platform(),release:os.release(),architecture:os.arch(),cpu:os.cpus()[0].model,node:process.version},
  scope:'Warm, in-memory Node WASM API comparison. Eight individually itemized CJK scalars, identical glyph results. No file I/O, worker scheduling, paragraph itemization/layout, rasterization, cold start or desktop package included.',
  inputs:{fontSha256:record.sha256,fontByteLength:font.length,text,items:items.length,scalarContextPerCall:[...text].length},
  measuredWork:{singleCalls:'Eight font uploads, digest/directory/metadata/cmap checks and independent shape JSON calls per sample.',cascadeCall:'One font upload and verification, eight shape attempts with the same full context; ordered choice and diagnostics included.',shared:'C++ shaping still constructs a face/font per item; no production HB face cache or cross-request Rust font cache.'},
  warmupPairs:1,counterbalancedPairs:9,exactGlyphsVerified:true,singleCalls:summary(measurements.singleCalls),cascadeCall:summary(measurements.cascadeCall),
  artifacts:{rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cppWasmSha256:sha(cpp),adapterSha256:sha(readFileSync('.codex-work/text-component/index.js'))}};
mkdirSync('.codex-work/font-cascade',{recursive:true});writeFileSync('.codex-work/font-cascade/benchmark.json',JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({singleMedianMs:result.singleCalls.medianMs,cascadeMedianMs:result.cascadeCall.medianMs,scope:result.scope}));
