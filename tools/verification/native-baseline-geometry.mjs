// Exercise the additive precise baseline wire form through actual Rust/HB,
// not merely through library structs or a schema validator.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/native-baseline';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const wasm=createRequire(import.meta.url)(`../../${root}/wasm-node/mo_wasm.js`);
const shaper=await ShapingComponent.create(hbFactory,new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
let componentCalls=0;
const component={...Object.fromEntries(['shapeBatch','measureBatch','outlineBatch'].map(name=>[name,(...args)=>{componentCalls++;return shaper[name](...args);}])),invalidate(){assert.fail('invalid wire input must not invalidate the component');}};
const fontPath='fixtures/fonts/owned-decorations.ttf',font=fs.readFileSync(fontPath);
const manifest=JSON.parse(fs.readFileSync('fixtures/fonts/decoration-manifest.json'));
const style={language:'en',features:[],candidates:[{font:0,variations:[]}],suppressDottedCircle:false,maxGlyphs:262144};
const base={shaping:{paragraph:{text:'AA',direction:'leftToRight',spans:[{end:1,style:0},{end:2,style:1}],styles:[style,style],fonts:manifest.fonts},lineEnds:[2]},styles:[{fontSize:'381000',baselineShift:'0'},{fontSize:'381000',baselineShift:'0'}],strutStyle:0,spacing:{kind:'natural'}};
const cases=[];
function run(name,q,success){
 const json=typeof q==='string'?q:JSON.stringify(q),head=Buffer.alloc(8);head.writeUInt32LE(Buffer.byteLength(json));head.writeUInt32LE(font.length,4);
 const n=spawnSync('target/debug/mo-text-worker',['--geometry'],{input:Buffer.concat([head,Buffer.from(json),font]),env:{},timeout:60000,maxBuffer:8*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());assert.equal(n.stdout.length,n.stdout.readUInt32LE()+4);
 const raw=n.stdout.subarray(4).toString();componentCalls=0;assert.equal(wasm.layout_lines(json,font,component),raw);
 const r=JSON.parse(raw);assert.equal(r.status!=='error',success,name);
 if(success){assert.equal(r.result.issues.length,0);assert.ok(r.result.layout);}
 else {assert.equal(r.error.code,'INPUT_INVALID');assert.equal(componentCalls,0);}
 const prefix=root+'/geometry-'+name;fs.writeFileSync(prefix+'.request.json',json);fs.writeFileSync(prefix+'.response.json',raw);
 cases.push({name,request:entry(prefix+'.request.json'),response:entry(prefix+'.response.json'),bundle:entry(fontPath),success,componentCalls});return raw;
}
for(const [name,offset,spacing] of [
 ['half',{q32:'2147483648'},{kind:'natural'}],
 ['negative',{q32:'-6442450944'},{kind:'natural'}],
 ['at-least',{q32:'2147483648'},{kind:'atLeast',height:'500000'}],
 ['exact',{q32:'2147483648'},{kind:'exact',height:'100000'}],
]){const q=structuredClone(base);q.styles[1].baselineShift=offset;q.spacing=spacing;run(name,q,true);}
const legacy=structuredClone(base);for(const s of legacy.styles)s.baselineShift='1';
const old=run('legacy-integer',legacy,true);
const equivalent=structuredClone(legacy);equivalent.styles[1].baselineShift={q32:'4294967296'};
assert.equal(run('equivalent-integer',equivalent,true),old);
const zero=structuredClone(base);zero.styles[1].baselineShift={q32:'0'};
for(const [name,offset] of [
 ['unknown',{q32:'0',extra:0}],['range',{q32:'170141183460469231731687303715884105728'}],
 ['noncanonical',{q32:'01'}],['number',{q32:0.5}],
]){const q=structuredClone(base);q.styles[1].baselineShift=offset;run(name,q,false);}
run('duplicate',JSON.stringify(zero).replace('"q32":"0"','"q32":"0","q32":"1"'),false);
// Same live instance must remain usable after every invalid input above.
run('equivalent-zero',zero,true);
assert.equal(cases.length,12);
const report={format:'musteroffice.native-baseline-geometry/1',counts:{requests:12,success:7,errors:5},artifacts:['target/debug/mo-text-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`].map(entry),cases};
fs.writeFileSync(root+'/geometry.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report.counts));
