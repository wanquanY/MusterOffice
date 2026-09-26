import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/text-shaping';mkdirSync(root,{recursive:true});
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const binary=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm');
const compiled=new WebAssembly.Module(binary),component=await ShapingComponent.create(factory,compiled);
const manifest=JSON.parse(readFileSync('.codex-work/harfbuzz/cases/manifest.json'));
const sourceCases=manifest.cases.slice(0,35),cases=[];
function input(c) {
  const text=c.before+c.text+c.after;
  return {expectedSha256:c.fontSha256,faceIndex:c.face,text,runs:[{
    start:[...c.before].length,end:[...c.before+c.text].length,
    direction:['leftToRight','rightToLeft','topToBottom','bottomToTop'][c.direction-4],
    script:c.script,language:c.language,clusterLevel:['monotoneGraphemes','monotoneCharacters','characters','graphemes'][c.clusterLevel],
    flags:{beginningOfText:!!(c.flags&1),endOfText:!!(c.flags&2),ignorables:c.flags&4?'preserve':c.flags&8?'remove':'default',suppressDottedCircle:!!(c.flags&16),unsafeToConcat:!!(c.flags&64),safeToInsertTatweel:!!(c.flags&128)},
    features:c.features.map(([tag,value,start,end])=>({tag,value,start,end:end===0xffffffff?null:end})),
    variations:c.variations.map(([tag,v])=>({tag,value1616:Math.round(v*65536)})),maxGlyphs: c.name==='output-budget'?1:262144,
  }]};
}
// Serde camelCase preserves numeric underscores as digits: confirm via generated schema.
const requestSchema=JSON.parse(readFileSync('contracts/generated/shape-request.schema.json'));
assert.ok(requestSchema.$defs.ShapeVariation.properties.value1616);
function nativeWorker(request,font) {
  const req=Buffer.from(request),header=Buffer.alloc(8);header.writeUInt32LE(req.length);header.writeUInt32LE(font.length,4);
  const r=spawnSync('target/release/mo-text-worker',[],{input:Buffer.concat([header,req,font]),maxBuffer:80*1024*1024,timeout:30000});
  assert.equal(r.status,0,r.stderr?.toString());const n=r.stdout.readUInt32LE(0);assert.equal(r.stdout.length,n+4);return r.stdout.subarray(4).toString();
}
function run(name,request,fontPath,expected='shaped',validRequest=true) {
  const json=typeof request==='string'?request:JSON.stringify(request),font=readFileSync(fontPath);
  const n=nativeWorker(json,font),w=wasm.shape_text(json,font,component);
  assert.equal(w,n,name);const response=JSON.parse(n);
  assert.equal(response.status==='shaped'?'shaped':response.error.code,expected,name);
  const record={name,request:json,validRequest,fontSha256:sha(font),response,responseSha256:sha(n)};cases.push(record);return record;
}
function golden(c) {
  const r=spawnSync('.codex-work/harfbuzz/release/mo-hb-probe',[c.font,c.request,c.language],{encoding:'utf8',maxBuffer:20*1024*1024});assert.equal(r.status,0,r.stderr);
  return JSON.parse(r.stdout).words;
}
let glyphs=0;
for(const c of sourceCases) {
  const record=run(c.name,input(c),c.font,c.expectedStatus===0?'shaped':c.expectedStatus===4?'LIMIT_EXCEEDED':'INPUT_INVALID');
  if(record.response.status==='shaped') {
    const words=golden(c),out=record.response.text.runs[0].glyphs;assert.equal(out.length,words[4]);
    for(let i=0;i<out.length;i++) {
      const g=out[i],p=8+i*7;
      assert.deepEqual([g.glyphId,g.cluster,(g.unsafeToBreak?1:0)|(g.unsafeToConcat?2:0)|(g.safeToInsertTatweel?4:0),g.xAdvance>>>0,g.yAdvance>>>0,g.xOffset>>>0,g.yOffset>>>0],words.slice(p,p+7));
    }
    glyphs+=out.length;
  }
}
const base=input(sourceCases[0]),font=sourceCases[0].font;
for(const [name,change,expected] of [
  ['digest-conflict',r=>r.expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],
  ['run-range',r=>r.runs[0].end++,'INPUT_INVALID'],
  ['reversed-range',r=>r.runs[0].start=r.runs[0].end+1,'INPUT_INVALID'],
  ['script-syntax',r=>r.runs[0].script='la/n','INPUT_INVALID'],
  ['language-syntax',r=>r.runs[0].language='en/US','INPUT_INVALID'],
  ['feature-range',r=>r.runs[0].features=[{tag:'liga',value:1,start:10,end:1}],'INPUT_INVALID'],
  ['feature-tag',r=>r.runs[0].features=[{tag:'abc',value:1,start:0,end:null}],'INPUT_INVALID'],
  ['axis-too-small',r=>r.runs[0].variations=[{tag:'wght',value1616:99<<16}],'INPUT_INVALID'],
  ['axis-too-large',r=>r.runs[0].variations=[{tag:'wght',value1616:1001<<16}],'INPUT_INVALID'],
  ['duplicate-axis',r=>r.runs[0].variations=[{tag:'wght',value1616:400<<16},{tag:'wght',value1616:400<<16}],'INPUT_INVALID'],
  ['glyph-limit',r=>r.runs[0].maxGlyphs=262145,'LIMIT_EXCEEDED'],
  ['text-limit',r=>r.text='A'.repeat(65537),'LIMIT_EXCEEDED'],
  ['run-limit',r=>r.runs=Array.from({length:257},()=>structuredClone(r.runs[0])),'LIMIT_EXCEEDED'],
]) {const r=structuredClone(base);change(r);run(name,r,font,expected);}
const badEnum=JSON.stringify(base).replace('leftToRight','sideways');run('unknown-enum',badEnum,font,'INPUT_INVALID',false);
run('duplicate-json-key',JSON.stringify(base).replace('"faceIndex":0','"faceIndex":0,"faceIndex":0'),font,'INPUT_INVALID',false);
const unknown={...base,filename:'untrusted.ttf'};run('unknown-field',unknown,font,'INPUT_INVALID',false);
const batch=structuredClone(base);batch.runs.push({...structuredClone(batch.runs[0]),start:7,end:13});
const result=run('batch-context',batch,font).response.text;
for(let i=0;i<batch.runs.length;i++) {
  const single=run('batch-independent-'+i,{...batch,runs:[batch.runs[i]]},font).response.text;
  assert.deepEqual(result.runs[i],single.runs[0]);
}
const failed=structuredClone(batch);failed.runs[1].maxGlyphs=0;
assert.ok(!('text' in run('atomic-second-run-failure',failed,font,'LIMIT_EXCEEDED').response));
run('no-runs',{...base,runs:[]},font);
const fractional=structuredClone(base);fractional.runs[0].variations=[{tag:'wght',value1616:(400<<16)+1}];
const effective=run('explicit-axis-quantization',fractional,font).response.text.runs[0].effectiveVariations[0];
const b=new ArrayBuffer(4),v=new DataView(b);v.setFloat32(0,fractional.runs[0].variations[0].value1616/65536,true);
assert.equal(effective.effectiveF32Bits,v.getUint32(0,true));assert.equal(effective.requested1616,(400<<16)+1);
// Verify the actual CLI starts its sibling worker and yields the same bytes.
writeFileSync(root+'/request.json',JSON.stringify(base));
const cli=spawnSync('target/release/mo-cli',['shape-text',root+'/request.json',font],{encoding:'utf8',maxBuffer:80*1024*1024});
assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trim(),nativeWorker(JSON.stringify(base),readFileSync(font)));
// Tampered component outputs are rejected by the actual Rust WASM decoder.
let invalidations=0;
const corrupt={shapeBatch(){return Uint32Array.of(0,1,8,0,1,1000,64000,0,67,0,0);},invalidate(){invalidations++;}};
assert.equal(JSON.parse(wasm.shape_text(JSON.stringify(base),readFileSync(font),corrupt)).error.code,'COMPONENT_INVALID');assert.equal(invalidations,1);
component.invalidate();assert.throws(()=>component.shapeBatch(new Uint8Array([1]),new Uint32Array()),/unavailable/);
const replacement=await ShapingComponent.create(factory,compiled);
assert.equal(wasm.shape_text(JSON.stringify(base),readFileSync(font),replacement),nativeWorker(JSON.stringify(base),readFileSync(font)));
// Same worker handles independent jobs and a rejected request without losing framing.
const fontBytes=readFileSync(font),jobs=[JSON.stringify(base),badEnum,JSON.stringify(base)];
const frames=jobs.map(job=>{const body=Buffer.from(job),h=Buffer.alloc(8);h.writeUInt32LE(body.length);h.writeUInt32LE(fontBytes.length,4);return Buffer.concat([h,body,fontBytes]);});
const persistent=spawnSync('target/release/mo-text-worker',[],{input:Buffer.concat(frames),maxBuffer:80*1024*1024});assert.equal(persistent.status,0,persistent.stderr.toString());
let cursor=0;
for(const job of jobs){const n=persistent.stdout.readUInt32LE(cursor);cursor+=4;assert.equal(persistent.stdout.subarray(cursor,cursor+n).toString(),nativeWorker(job,fontBytes));cursor+=n;}
assert.equal(cursor,persistent.stdout.length);
for(const header of [Buffer.from([1]),Buffer.from([0,0,0,0,255,255,255,255])]) {
  const invalid=spawnSync('target/release/mo-text-worker',[],{input:header,maxBuffer:1024*1024});assert.notEqual(invalid.status,0);assert.equal(invalid.stdout.length,0);
}
// Actual allocation failures go through C++ -> adapter -> Rust, not a mock backend.
const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default;
const faultCompiled=new WebAssembly.Module(readFileSync('.codex-work/harfbuzz/mo-hb.wasm'));
const faultCases=[];
for(const failAfter of [0,7,16,150]) {
  let raw;
  const instance=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},faultCompiled);
  raw._mo_hb_fail_after(failAfter);
  const response=JSON.parse(wasm.shape_text(JSON.stringify(base),fontBytes,instance));
  assert.equal(response.error.code,'COMPONENT_FAILURE');assert.match(response.error.message,/status 2 at run 0/);assert.equal(instance.invalid,true);
  assert.equal(JSON.parse(wasm.shape_text(JSON.stringify(base),fontBytes,instance)).error.code,'HOST_FAILURE');
  faultCases.push({failAfter,response,reuseRejected:true});
}
assert.equal(wasm.shape_text(JSON.stringify(base),fontBytes,replacement),nativeWorker(JSON.stringify(base),fontBytes));
const report={format:'musteroffice.text-parity/1',nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),nativeCliSha256:sha(readFileSync('target/release/mo-cli')),wasmKernelSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(binary),componentGoldenCases:33,componentGoldenGlyphs:glyphs,cliVerified:true,tamperedOutputRejected:true,invalidatedInstanceRejected:true,replacementInstanceVerified:true,persistentNativeJobs:3,malformedNativeFramesRejected:2,faultCases,cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({cases:cases.length,goldenCases:33,goldenGlyphs:glyphs,cliVerified:true,replacementVerified:true}));
