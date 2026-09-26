// Exercise per-style spacing through actual Rust/HB and replay precise baseline
// inputs. Explicit bidi controls and zero-width text marks remain distinct.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/paragraph-spacing';
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
const previousPath='docs/reviews/evidence/2026-09-25-native-baseline-verification.json';
assert.equal(entry(previousPath).sha256,'41cd690774f4a0d3335a6e981bbffadceda43e692442b1ff2e3708537b9c833a');
const previous=JSON.parse(fs.readFileSync(previousPath));
for(const c of previous.geometryEvidence.cases){
 assert.deepEqual(entry(c.request.path),c.request);assert.deepEqual(entry(c.response.path),c.response);
 const raw=run('prior-'+c.name,fs.readFileSync(c.request.path).toString(),c.success);
 assert.equal(raw,fs.readFileSync(c.response.path).toString());cases.at(-1).priorResponse=c.response;
}
const raw=n=>(BigInt(n)*4294967296n).toString();
const baseHeight=structuredClone(base);baseHeight.spacing={kind:'styleMaximum',heights:[raw(100000),raw(200000)]};
run('maximum',baseHeight,true);
const two=structuredClone(baseHeight);two.shaping.lineEnds=[1,2];two.spacing.heights=['545466503921664','1288535284187648'];run('per-line',two,true);
const zero=structuredClone(baseHeight);zero.spacing.heights=['0','0'];run('zero',zero,true);
const empty=structuredClone(baseHeight);empty.shaping.paragraph.text='';empty.shaping.paragraph.spans=[];empty.shaping.lineEnds=[0];empty.strutStyle=1;run('empty',empty,true);
const mark=structuredClone(empty);mark.shaping.paragraph.text='\u200f';mark.shaping.paragraph.spans=[{end:1,style:0}];mark.shaping.lineEnds=[1];
const markResult=JSON.parse(run('zero-width-mark',mark,true)).result;
assert.equal(markResult.shaping.items[0].kind,'text');assert.equal(markResult.layout.height,'100000');
const control=structuredClone(mark);control.shaping.paragraph.text='\u202c';
const controlResult=JSON.parse(run('control',control,true)).result;
assert.equal(controlResult.shaping.items[0].kind,'bidiControl');assert.equal(controlResult.layout.height,'200000');
for(const [name,heights] of [['missing',[]],['extra',['0','0','0']],['negative',['0','-1']],['number',[0,1]]]){
 const q=structuredClone(baseHeight);q.spacing.heights=heights;run(name,q,false);
}
run('duplicate',JSON.stringify(baseHeight).replace('"heights":','"heights":[],"heights":'),false);
const equivalent=structuredClone(baseHeight);equivalent.spacing.heights=[raw(100000),raw(100000)];
const result=JSON.parse(run('equivalent',equivalent,true));assert.equal(result.result.shaping.items.length,1);
assert.equal(cases.length,24);
const report={format:'musteroffice.paragraph-spacing-geometry/1',previousEvidence:entry(previousPath),counts:{priorRequests:12,newRequests:12,requests:24,success:14,errors:10},artifacts:['target/debug/mo-text-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`].map(entry),cases};
fs.writeFileSync(root+'/geometry.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report.counts));
