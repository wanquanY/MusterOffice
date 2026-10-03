/** Replay actual Native geometry results through the real WASM component.
 * The explicit baseline component also checks the refactored renderer's pixels.
 */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';
const [root,baseline,sanitizedProbe] = process.argv.slice(2);
assert(root && baseline,'usage: raster-picking-parity.mjs evidence-directory baseline-component-directory');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,bytes:b.length,sha256:sha(b)};};
let sanitizedCalls=0;
function nativeCheck(frame,reply) {
  if(!sanitizedProbe)return;
  const input=Buffer.alloc(4+frame.length*4);input.writeUInt32LE(frame.length);
  frame.forEach((word,i)=>input.writeUInt32LE(word,4+i*4));
  const r=spawnSync(sanitizedProbe,['--pick'],{input,timeout:30000,maxBuffer:2*1024*1024,
    env:{...process.env,ASAN_OPTIONS:'detect_leaks=0:halt_on_error=1',UBSAN_OPTIONS:'halt_on_error=1:print_stacktrace=1'}});
  assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0,r.stderr.toString());
  assert.equal(r.stdout.readUInt32LE(),reply.status);assert.equal(r.stdout.readUInt32LE(4),r.stdout.length-8);
  const words=Array.from({length:(r.stdout.length-8)/4},(_,i)=>r.stdout.readUInt32LE(8+i*4));
  assert.deepEqual(words,[...reply.words]);sanitizedCalls++;
}
async function load(directory) {
  const {default:factory}=await import(pathToFileURL(path.resolve(directory,'mo-skia.mjs')));
  return RasterComponent.create(factory,new WebAssembly.Module(fs.readFileSync(path.join(directory,'mo-skia.wasm'))));
}
const component=await load(path.join(root,'skia')),old=await load(baseline);
assert.equal(component.supportsPicking,true);
assert.equal(old.supportsPicking,false);
const cases=[]; let queries=0,rendered=0;
for(const name of fs.readdirSync(path.join(root,'fixtures')).filter(n=>n.endsWith('.json')).sort()) {
  const source=path.join(root,'fixtures',name),f=JSON.parse(fs.readFileSync(source));
  const frame=Uint32Array.from(f.frame),result=component.pick(frame);
  assert.equal(result.status,f.status??0,name); assert.deepEqual([...result.words],f.reply,name);
  nativeCheck(frame,result);
  assert.equal(component.invalid,false); queries+=frame[9];
  let pixels;
  if(f.rasterFrame) {
    const r=Uint32Array.from(f.rasterFrame),now=component.raster(r),before=old.raster(r);
    assert.equal(now.status,0,name); assert.equal(before.status,0,name);
    assert.deepEqual(now.pixels,before.pixels,name);
    assert.deepEqual([...now.pixels],f.pixels,name); rendered++;
    pixels=sha(now.pixels);
  }
  cases.push({name,status:result.status,input:entry(source),replySha256:sha(Buffer.from(result.words.buffer)),pixelsSha256:pixels});
}
assert(cases.length>0 && rendered>0);
assert.throws(()=>old.pick(Uint32Array.of(0x4d4f504b,1,1,1,0,0,0,0,0,0)),/extension unavailable/);
assert.equal(old.invalid,false);
// Raw grammar refusal must not lose ownership or poison later valid calls.
const base=JSON.parse(fs.readFileSync(path.join(root,'fixtures','fill-hole-clip.json'))).frame;
let rejected=0;
for(let end=0;end<base.length;end++) {
  const result=component.pick(Uint32Array.from(base.slice(0,end)));
  assert.notEqual(result.status,0); assert.equal(result.words.length,0); assert.equal(component.invalid,false);
  nativeCheck(Uint32Array.from(base.slice(0,end)),result);rejected++;
}
for(const index of [0,1,4,5,6,7,8,9,10,11]) {
  const bad=Uint32Array.from(base);bad[index]=0xffffffff;
  const result=component.pick(bad);assert.notEqual(result.status,0);assert.equal(result.words.length,0);assert.equal(component.invalid,false);rejected++;
  nativeCheck(bad,result);
}
let clips=10;for(let i=0;i<base[4];i++)clips+=2+base[clips+1]*7;clips+=base[6]*4;
const draws=clips+base[7]*4,query=draws+base[8]*5;
for(const [index,value] of [[12,6],[13,0x7fc00000],[18,1],[clips,1],[clips+1,base[4]],[clips+2,0x7f800000],
  [draws,base[4]],[draws+3,1],[draws+4,base[7]+1],[query,0x7fc00000],[query+2,0xbf800000]]) {
  const bad=Uint32Array.from(base);bad[index]=value;
  const result=component.pick(bad);assert.notEqual(result.status,0,`invalid word ${index}`);assert.equal(result.words.length,0);
  assert.equal(component.invalid,false);nativeCheck(bad,result);rejected++;
}
assert.equal(component.pick(Uint32Array.from(base)).status,0);
const report={format:'musteroffice.raster-picking-parity/1',cases,queries,rendered,invalidFrames:rejected,
  component:entry(path.join(root,'skia/mo-skia.wasm')),baseline:entry(path.join(baseline,'mo-skia.wasm')),
  adapter:entry('.codex-work/raster-component/index.js'),pickingAdapter:entry('.codex-work/raster-component/picking.js')};
if(sanitizedProbe)report.sanitizer={probe:entry(sanitizedProbe),calls:sanitizedCalls,
  scope:'adapter and probe instrumented with ASan/UBSan; reused Skia and codec archives uninstrumented',leakSanitizer:false};
fs.writeFileSync(path.join(root,'parity.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({cases:cases.length,queries,rendered,invalidFrames:rejected,sanitizedCalls}));
