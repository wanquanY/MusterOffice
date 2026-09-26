// Actual signed cluster spacing and prior baseline/line-spacing execution.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import hbFactory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const root='.codex-work/character-spacing';
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
function run(name,q,success,lateFailure=false){
 const json=typeof q==='string'?q:JSON.stringify(q),head=Buffer.alloc(8);head.writeUInt32LE(Buffer.byteLength(json));head.writeUInt32LE(font.length,4);
 const n=spawnSync('target/debug/mo-text-worker',['--geometry'],{input:Buffer.concat([head,Buffer.from(json),font]),env:{},timeout:60000,maxBuffer:8*1024*1024});
 assert.equal(n.status,0,n.stderr?.toString());assert.equal(n.stdout.length,n.stdout.readUInt32LE()+4);
 const raw=n.stdout.subarray(4).toString();componentCalls=0;assert.equal(wasm.layout_lines(json,font,component),raw);
 const r=JSON.parse(raw);assert.equal(r.status!=='error',success,name);
 if(success){assert.equal(r.result.issues.length,0);assert.ok(r.result.layout);}
 else {
  assert.equal(r.error.code,lateFailure?'LIMIT_EXCEEDED':'INPUT_INVALID');
  if(lateFailure){assert.ok(componentCalls>0);assert.match(r.error.message,/line geometry numeric range/);}
  else assert.equal(componentCalls,0);
 }
 const prefix=root+'/geometry-'+name;fs.writeFileSync(prefix+'.request.json',json);fs.writeFileSync(prefix+'.response.json',raw);
 cases.push({name,request:entry(prefix+'.request.json'),response:entry(prefix+'.response.json'),bundle:entry(fontPath),success,componentCalls});return raw;
}
const previousPath='docs/reviews/evidence/2026-09-25-paragraph-spacing-verification.json';
assert.equal(entry(previousPath).sha256,'834cfbbbb98b3a064d93159f70e973ef8db86a0fa88d91c63266a3e3fa880ccc');
const previous=JSON.parse(fs.readFileSync(previousPath));
for(const c of previous.geometryEvidence.cases){
 assert.deepEqual(entry(c.request.path),c.request);assert.deepEqual(entry(c.response.path),c.response);
 const raw=run('prior-'+c.name,fs.readFileSync(c.request.path).toString(),c.success);
 assert.equal(raw,fs.readFileSync(c.response.path).toString());cases.at(-1).priorResponse=c.response;
}
const raw=n=>(BigInt(n)*4294967296n).toString();
const tracked=value=>{const q=structuredClone(base);q.styles.forEach(s=>s.clusterSpacing=value);return q;};
run('positive',tracked(raw(10000)),true);
run('negative',tracked(raw(-10000)),true);
run('fractional',tracked('42949672960001'),true);
run('reversed-advance',tracked(raw(-500000)),true);
const mixed=tracked(raw(10000));mixed.styles[1].clusterSpacing=raw(-10000);run('mixed',mixed,true);
const combining=tracked(raw(10000));combining.shaping.paragraph.text='A\u0301A';combining.shaping.paragraph.spans=[{end:3,style:0}];combining.shaping.lineEnds=[3];run('combining',combining,true);
const rtl=tracked(raw(10000));rtl.shaping.paragraph.text='אב';run('rtl',rtl,true);
const empty=tracked(raw(10000));empty.shaping.paragraph.text='';empty.shaping.paragraph.spans=[];empty.shaping.lineEnds=[0];run('empty',empty,true);
const legacy=run('legacy',base,true);assert.equal(run('zero',tracked('0'),true),legacy);
for(const [name,value] of [['number',1],['null',null],['noncanonical','01'],['range','170141183460469231731687303715884105728']])run(name,tracked(value),false);
run('duplicate',JSON.stringify(tracked('1')).replace('"clusterSpacing":','"clusterSpacing":"0","clusterSpacing":'),false);
run('overflow',tracked('170141183460469231731687303715884105727'),false,true);
const recovered=JSON.parse(run('recovered',tracked(raw(10000)),true));assert.equal(recovered.result.shaping.items.length,1);
assert.equal(cases.length,41);
const report={format:'musteroffice.character-spacing-geometry/1',previousEvidence:entry(previousPath),counts:{priorRequests:24,newRequests:17,requests:41,success:25,errors:16},artifacts:['target/debug/mo-text-worker',`${root}/wasm-node/mo_wasm.js`,`${root}/wasm-node/mo_wasm_bg.wasm`].map(entry),cases};
fs.writeFileSync(root+'/geometry.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report.counts));
