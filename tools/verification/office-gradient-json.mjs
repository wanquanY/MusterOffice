/** Preserve a real pre-fix WASM parser and replay the same decimal inputs. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {RasterComponent} from '../../.codex-work/office-gradient/ts-raster/index.js';
import factory from '../../.codex-work/office-gradient/component/mo-skia.mjs';
const root='.codex-work/office-gradient',out=root+'/json-regression';fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const load=createRequire(import.meta.url),before=load('../../.codex-work/office-gradient/wasm-before-roundtrip/mo_wasm.js'),after=load('../../.codex-work/office-gradient/wasm-node/mo_wasm.js');
const module=new WebAssembly.Module(fs.readFileSync(root+'/component/mo-skia.wasm'));
const cases=[];
for(const name of ['near-position','near-color']){
 const q=JSON.parse(fs.readFileSync(root+'/runtime/linear-clamp-symmetric.json'));
 const g=q.draws[0].brush.gradient;g.stops[0].srgb[0]=g.stops[2].srgb[0]=1;
 if(name==='near-position')g.stops[2].position=.9999999999999999;else g.stops[2].srgb[0]=.9999999999999999;
 const json=JSON.stringify(q);assert(json.includes('0.9999999999999999'));const record={name,request:put(out+'/'+name+'.json',json)};
 for(const [label,runtime] of [['before',before],['after',after]]){
  const component=await RasterComponent.create(factory,module);let calls=0;
  const r=runtime.render_paths(json,{raster(frame){calls++;return component.raster(frame);},invalidate(){component.invalidate();}});
  const text=r.metadata,metadata=JSON.parse(text),pixels=Buffer.from(r.take_pixels());
  assert.equal(metadata.status,label==='before'?'rendered':'error');assert.equal(calls,label==='before'?1:0);
  if(label==='after')assert.equal(pixels.length,0);
  record[label]={calls,response:put(out+'/'+name+'.'+label+'.json',text),pixels:put(out+'/'+name+'.'+label+'.rgba',pixels)};component.invalidate();
 }
 cases.push(record);
}
const report={format:'musteroffice.office-gradient-json-regression/1',beforeWasm:entry(root+'/wasm-before-roundtrip/mo_wasm_bg.wasm'),beforeGlue:entry(root+'/wasm-before-roundtrip/mo_wasm.js'),afterWasm:entry(root+'/wasm-node/mo_wasm_bg.wasm'),afterGlue:entry(root+'/wasm-node/mo_wasm.js'),cases};
fs.writeFileSync(root+'/json-regression.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({reproducedBefore:cases.length,rejectedAfter:cases.length}));
