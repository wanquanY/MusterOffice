import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root=process.argv[2];assert.ok(root,'usage: font-carets-safety.mjs evidence-directory');
const sha=b=>createHash('sha256').update(b).digest('hex');
const wasm=createRequire(import.meta.url)(path.resolve(root,'wasm-node/mo_wasm.js'));
const {ShapingComponent}=await import(pathToFileURL(path.resolve(root,'text-component/index.js')));
const {default:factory}=await import(pathToFileURL(path.resolve(root,'harfbuzz/mo-hb.mjs')));
const {default:faultFactory}=await import(pathToFileURL(path.resolve(root,'fault/mo-hb.mjs')));
const binary=readFileSync(path.join(root,'harfbuzz/mo-hb.wasm')),faultBinary=readFileSync(path.join(root,'fault/mo-hb.wasm'));
const regular=await factory({wasmBinary:binary});assert.equal(typeof regular._mo_hb_fail_after,'undefined');
const points=process.argv[3]==='points';
const fontPath=points?'fixtures/fonts/owned-carets-points.ttf':'fixtures/fonts/owned-carets.ttf',font=readFileSync(fontPath);
const out=path.join(root,points?'safety-points':'safety');mkdirSync(out,{recursive:true});
const tag=s=>Buffer.from(s).readUInt32BE(),float=n=>{const b=Buffer.alloc(4);b.writeFloatLE(n);return b.readUInt32LE();};
const ids=points?[2,3]:[0,1,2,3,4,5,6];
const good=[0x4d4f4354,1,0,1,points?6:4,ids.length,64,0,tag('wght'),float(650),...ids];
function raw(m,words,operation='carets') {
 const pointers=[];let output=0;
 const alloc=n=>{const p=m._malloc(Math.max(1,n));assert.ok(p);pointers.push(p);return p;};
 try {
  const f=alloc(font.length),r=alloc(words.length*4),slots=alloc(8),lang=alloc(3);
  m.HEAPU8.set(font,f);m.HEAPU32.set(words,r/4);m.HEAPU32.fill(0,slots/4,slots/4+2);m.HEAPU8.set(Buffer.from('und'),lang);
  const status=operation==='shape'?m._mo_hb_shape(f,font.length,r,words.length,lang,3,slots,slots+4):m._mo_hb_caret_font(f,font.length,r,words.length,slots,slots+4);
  output=m.HEAPU32[slots/4];const n=m.HEAPU32[slots/4+1];assert.equal(Boolean(output),n!==0);
  if(status!==0)assert.equal(output,0);
  return {status,words:output?Array.from(m.HEAPU32.subarray(output/4,output/4+n)):[]};
 } finally {if(output)m._mo_hb_free(output);for(const p of pointers)m._free(p);}
}
function native(words,label,failAfter,sanitized=false) {
 const file=path.join(out,label+'.bin'),buffer=Buffer.alloc(words.length*4);words.forEach((w,i)=>buffer.writeUInt32LE(w>>>0,i*4));writeFileSync(file,buffer);
 const args=[fontPath,file,'--carets'];if(failAfter!==undefined)args.push(String(failAfter));
 const r=spawnSync(path.join(root,sanitized?'asan':'fault','mo-hb-probe'),args,{encoding:'utf8',timeout:30000,env:{...process.env,ASAN_OPTIONS:'halt_on_error=1',UBSAN_OPTIONS:'halt_on_error=1:print_stacktrace=1'}});
 assert.equal(r.status,0,r.stderr);assert.equal(r.stderr,'');return JSON.parse(r.stdout);
}
const inputs=[['valid',good,0],['truncated',good.slice(0,7),1],['trailing',[...good,0],1]];
for(const [name,index,value,status=1] of [
 ['magic',0,0],['version',1,2],['bad-face',2,99,5],['axes',3,65],['direction-low',4,3],['direction-high',4,8],
 ['glyph-count',5,257],['caret-count',6,65],['reserved',7,1],['unknown-axis',8,tag('xxxx')],['nan-axis',9,0x7fc00000],
 ['inf-axis',9,0x7f800000],['axis-range',9,float(901)],['glyph-range',10,7],['duplicate-glyph',11,ids[0]],
]) {const words=[...good];words[index]=value;inputs.push([name,words,status]);}
const duplicateAxis=[...good.slice(0,10),tag('wght'),float(650),...ids];duplicateAxis[3]=2;inputs.push(['duplicate-axis',duplicateAxis,1]);
const cases=[];
for(const [name,words,status] of inputs) {
 const n=native(words,name),s=native(words,name,undefined,true),w=raw(regular,words);
 assert.equal(n.status,status,name);assert.deepEqual(w,{status,words:n.words},name);assert.deepEqual(s.words,n.words);assert.equal(s.status,status);
 cases.push({name,status,requestSha256:sha(readFileSync(path.join(out,name+'.bin'))),responseSha256:sha(JSON.stringify(w))});
}
const baseline=raw(regular,good),shape=[0x4d4f4842,0,4,tag('Latn'),3,0,1,0,1,0,0,16,1,65],faults=[];
assert.equal(baseline.status,0);assert.equal(raw(regular,shape,'shape').status,0);
for(let at=0;at<300;at++) {
 const n=native(good,'fault',at),s=native(good,'fault',at,true),m=await faultFactory({wasmBinary:faultBinary});
 m._mo_hb_fail_after(at);const w=raw(m,good);m._mo_hb_fail_after(0xffffffff);
 const reuse=raw(m,good),cross=raw(m,shape,'shape');
 for(const r of [n,s]) {
  assert.ok([0,2].includes(r.status),JSON.stringify({at,r}));
  if(r.status===2){assert.deepEqual(r.words,[]);assert.equal(r.recoveryStatus,6);assert.deepEqual(r.recoveryWords,[]);}
  else{assert.deepEqual(r.words,baseline.words);assert.deepEqual(r.recoveryWords,baseline.words);}
 }
 assert.equal(n.status,s.status);assert.ok([0,2].includes(w.status));
 if(w.status===2){assert.deepEqual(w.words,[]);assert.equal(reuse.status,6);assert.equal(cross.status,6);}
 else {assert.deepEqual(w,baseline);assert.deepEqual(reuse,baseline);assert.equal(cross.status,0);}
 faults.push({at,nativeStatus:n.status,wasmStatus:w.status,sanitizedStatus:s.status,retainedBytes:n.retainedBytes,wasmShapeReuseStatus:cross.status});
 if(at%10===0)global.gc?.();
}
assert.ok(faults.some(f=>f.nativeStatus===2));assert.ok(faults.some(f=>f.nativeStatus===0));assert.ok(faults.some(f=>f.wasmStatus===2));assert.ok(faults.some(f=>f.wasmStatus===0));
const reverse=await faultFactory({wasmBinary:faultBinary});reverse._mo_hb_fail_after(0);assert.equal(raw(reverse,shape,'shape').status,2);reverse._mo_hb_fail_after(0xffffffff);assert.equal(raw(reverse,good).status,6);
let calls=0;
const adapter=await ShapingComponent.create(async options=>{const m=await faultFactory(options),original=m._mo_hb_caret_font;m._mo_hb_caret_font=(...args)=>{if(++calls===2)m._mo_hb_fail_after(0);return original(...args);};return m;},new WebAssembly.Module(faultBinary));
const instance={variations:[],direction:'leftToRight',glyphIds:points?[2,3]:[3,5,6]},q=JSON.stringify({expectedSha256:sha(font),faceIndex:0,instances:[instance,instance]});
const failed=JSON.parse(wasm.font_carets(q,font,adapter));assert.equal(failed.error.code,'COMPONENT_FAILURE');assert.equal(failed.carets,undefined);assert.equal(adapter.invalid,true);assert.equal(calls,2);
assert.equal(JSON.parse(wasm.font_carets(q,font,adapter)).error.code,'HOST_FAILURE');
assert.throws(()=>adapter.shapeBatch(font,Uint32Array.from([0x4d4f5342,1,0x0e0500,0])));
const replacement=await ShapingComponent.create(factory,new WebAssembly.Module(binary));assert.equal(JSON.parse(wasm.font_carets(q,font,replacement)).status,'queried');
const report={format:'musteroffice.font-carets-safety/1',profile:points?'composite-gvar-vertical':'gdef-horizontal',nativeProbeSha256:sha(readFileSync(path.join(root,'fault/mo-hb-probe'))),sanitizedProbeSha256:sha(readFileSync(path.join(root,'asan/mo-hb-probe'))),
 releaseWasmSha256:sha(binary),faultWasmSha256:sha(faultBinary),fontSha256:sha(font),addressSanitizer:true,undefinedBehaviorSanitizer:true,cases,faults,
 lateBatchFailureAtomic:true,bidirectionalOperationQuarantine:true,replacementVerified:true};
writeFileSync(path.join(root,points?'safety-points.json':'safety.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({invalidFrames:cases.length-1,faultPositions:faults.length,nativeFailures:faults.filter(f=>f.nativeStatus===2).length,wasmFailures:faults.filter(f=>f.wasmStatus===2).length}));
