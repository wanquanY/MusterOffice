// Exercise real C++ WASM font registration, ownership and unchanged batch replies.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
const [adapter,hbDirectory,fontPath,output]=process.argv.slice(2);
assert.ok(output,'adapter.js hbDirectory ownedFont.ttf new-report.json');
const {ShapingComponent}=await import(pathToFileURL(path.resolve(adapter)));
const {default:factory}=await import(pathToFileURL(path.resolve(hbDirectory,'mo-hb.mjs')));
const compiled=new WebAssembly.Module(await fs.readFile(path.join(hbDirectory,'mo-hb.wasm')));
const font=new Uint8Array(await fs.readFile(fontPath));
const tag=s=>Buffer.from(s).readUInt32BE();
const shape=Uint32Array.from([0x4d4f5342,1,0x0e0500,1,3,14,117,110,100,0x4d4f4842,0,4,tag('Latn'),3,0,1,0,1,0,0,16,1,65]);
const metric=Uint32Array.from([0x4d4f4d42,1,0x0e0500,1,7,0x4d4f4d54,1,0,0,1,0,tag('hasc')]);
const outlines=Uint32Array.from([0x4d4f4f42,1,0x0e0500,1,9,0x4d4f4f54,1,0,0,1,4096,1048576,0,1]);
const carets=Uint32Array.from([0x4d4f4342,1,0x0e0500,1,9,0x4d4f4354,1,0,0,4,1,64,0,1]);
const frames=[['shapeBatch','shapeRegistered',shape],['measureBatch','measureRegistered',metric],['outlineBatch','outlineRegistered',outlines],['caretBatch','caretRegistered',carets]];
let allocations=[],frees=[];
const component=await ShapingComponent.create(async options=>{
  const m=await factory(options),malloc=m._malloc,free=m._free;
  m._malloc=n=>{const p=malloc(n);allocations.push({n,p});return p;};
  m._free=p=>{frees.push(p);free(p);};return m;
},compiled);
const reference=frames.map(([method,,frame])=>component[method](font,frame));
for(const response of reference)assert.equal(response[0],0);
allocations=[];frees=[];
const handle=component.registerFont(font),fontPointer=allocations[0].p;
assert.equal(allocations[0].n,font.length);
const saved=font.slice();font.fill(0); // Ownership is a copy, never a mutable caller view.
for(let repeat=0;repeat<3;repeat++)for(let i=0;i<frames.length;i++){
 const [,method,frame]=frames[i];assert.deepEqual(component[method](handle,frame),reference[i]);
}
assert.equal(allocations.filter(a=>a.n===font.length).length,1);
assert.equal(frees.filter(p=>p===fontPointer).length,0);
component.unregisterFont(handle);assert.equal(frees.filter(p=>p===fontPointer).length,1);
const second=component.registerFont(saved);assert.notEqual(second,handle);component.unregisterFont(second);
assert.throws(()=>component.shapeRegistered(handle,shape));assert.equal(component.invalid,true);
const bounded=await ShapingComponent.create(factory,compiled);
for(let i=0;i<32;i++)bounded.registerFont(saved);
assert.throws(()=>bounded.registerFont(saved));assert.equal(bounded.invalid,true);
const empty=await ShapingComponent.create(factory,compiled);assert.throws(()=>empty.registerFont(new Uint8Array()));assert.equal(empty.invalid,true);
await fs.writeFile(output,JSON.stringify({profile:'real-wasm-font-residency/1',exactBatchComparisons:12,fontUploadsForTwelveBatches:1,
  callerMutationIsolated:true,releaseVerified:true,staleHandleRejected:true,entryBudgetEnforced:true,emptyFontRejected:true},null,2)+'\n',{flag:'wx'});
console.log('Verified twelve identical real shaping/metrics/outline/caret replies with one font upload and scoped release.');
