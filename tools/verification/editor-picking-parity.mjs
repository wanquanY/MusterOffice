// Public page identities and picking through real Native/WASM/TS owners.
import assert from 'node:assert/strict';
import {readFileSync,readdirSync,mkdirSync,writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const root=resolve(process.argv[2]??'.codex-work/page-picking');
const wasm=createRequire(import.meta.url)(join(root,'wasm-node/mo_wasm.js'));
const hbRoot=resolve(process.env.MO_EDITOR_PAGE_HB??'.codex-work/caret/harfbuzz');
const skiaRoot=resolve(process.env.MO_EDITOR_PAGE_SKIA??'.codex-work/object-picking/skia');
const {ShapingComponent}=await import(pathToFileURL(resolve('.codex-work/caret/text-component/index.js')));
const {RasterComponent}=await import(pathToFileURL(resolve('.codex-work/raster-component/index.js')));
const {default:hb}=await import(pathToFileURL(join(hbRoot,'mo-hb.mjs')));
const {default:skia}=await import(pathToFileURL(join(skiaRoot,'mo-skia.mjs')));
const shaping=await ShapingComponent.create(hb,new WebAssembly.Module(readFileSync(join(hbRoot,'mo-hb.wasm'))));
const raster=await RasterComponent.create(skia,new WebAssembly.Module(readFileSync(join(skiaRoot,'mo-skia.wasm'))));
const {PresentationEditorPage,EditorPageComputationError}=await import(pathToFileURL(resolve('.codex-work/editor-client/build/editor-client/src/index.js')));
const fonts=readFileSync('fixtures/fonts/owned-interaction.ttf');
const sha=b=>createHash('sha256').update(b).digest('hex');
const fixed=n=>(BigInt(n)*4294967296n).toString();
const identity=o=>JSON.stringify([o.part,o.nativeId]);
const calls={};
const counted=(component,label)=>new Proxy(component,{get(target,key){
 const value=Reflect.get(target,key,target);
 return typeof value==='function'?(...args)=>{const name=`${label}.${String(key)}`;calls[name]=(calls[name]??0)+1;return value.apply(target,args);}:value;
}});
const text=counted(shaping,'text'),pixels=counted(raster,'raster');
function envelope(request,material,fonts){
 const json=Buffer.from(typeof request==='string'?request:JSON.stringify(request));
 const head=Buffer.alloc(12);head.writeUInt32LE(json.length);head.writeUInt32LE(material.length,4);head.writeUInt32LE(fonts.length,8);
 return Buffer.concat([head,json,material,fonts]);
}
function take(frame){
 let consumed=false;
 try{const metadata=frame.metadata;consumed=true;return {metadata,pixels:Buffer.from(frame.take_pixels())};}
 finally{if(!consumed)frame.free();}
}
function catalog(info,input){
 const seen=new Set();
 for(const [i,o] of info.objects.entries()){
  assert.equal(seen.has(identity(o.object)),false);seen.add(identity(o.object));
  const layer=info.page.layers.find(l=>l.part===o.object.part);assert.ok(layer);assert.equal(o.surface,layer.kind);
  if(o.parent!==null){
   assert.ok(Number.isInteger(o.parent)&&o.parent>=0&&o.parent<i);
   const parent=info.objects[o.parent];assert.equal(parent.kind,'group');assert.equal(parent.object.part,o.object.part);
  }
  if(input.kind==='pptx')assert.equal(o.objectId,null);
  else{
   assert.equal(typeof o.objectId,'string');assert.ok(input.document.objects[o.objectId]);
   if(input.kind==='retained'){
    const binding=input.document.sourceBindings.objects[o.objectId];
    assert.equal(identity(binding),identity(o.object));
   }
  }
 }
 for(const [i,f] of info.textFrames.entries()){
  assert.equal(f.frame,i);const o=info.objects.find(o=>identity(o.object)===identity(f.object));
  assert.ok(o);assert.equal(f.objectId,o.objectId);
 }
}
function checkHits(results,queries,info){
 assert.equal(results.length,queries.length);
 for(const [i,r] of results.entries()){
  assert.equal(typeof r.truncated,'boolean');assert.ok(r.hits.length<=queries[i].maxHits);
  if(r.truncated)assert.equal(r.hits.length,queries[i].maxHits);
  const seen=new Set();
  for(const hit of r.hits){
   assert.ok(Number.isInteger(hit.object)&&hit.object>=0&&hit.object<info.objects.length);
   assert.equal(seen.has(hit.object),false);seen.add(hit.object);assert.ok(['exact','nearby'].includes(hit.kind));
   if(hit.textFrame!==null){
    const f=info.textFrames[hit.textFrame];assert.ok(f);assert.equal(identity(f.object),identity(info.objects[hit.object].object));
   }
  }
 }
}
function textIdentities(info,input){
 const seen=new Set();let paragraphs=0,runs=0,unprojected=0,cells=0;
 for(const frame of info.textFrames){
  const content=input.document?.objects[frame.objectId]?.content;
  const cell=content?.kind==='table'?content.table.rows[frame.cell.row].cells[frame.cell.column]:null;
  assert.equal(frame.cellId,cell?.id??null);if(cell)cells++;
  const opaque=input.kind==='pptx'||(frame.cell&&input.kind==='retained'&&
   ['presentationml-retained-fields-v1-draft','presentationml-retained-fields-v2-draft'].includes(input.document.sourceBindings.profile))||
   (cell&&!cell.text);
  const authored=content?.kind==='shape'?content.text:cell?.text;
  for(const [i,p] of frame.paragraphs.entries()){
   if(opaque){assert.equal(p.model,null);unprojected++;continue;}
   assert.ok(p.model);assert.equal(seen.has(p.model.id),false);seen.add(p.model.id);paragraphs++;
   const model=authored?authored.paragraphs[i]:content.paragraphs.find(p2=>p2.id===p.model.id);
   assert.ok(model);assert.equal(model.id,p.model.id);assert.equal(model.runs.length,p.model.runs.length);
   let offset=0,plain='';
   for(const [j,r] of model.runs.entries()){
    const value=authored?(r.content.kind==='break'?'\u2028':r.content.kind==='tab'?'\t':r.content.text):(r.kind==='break'?'\u2028':r.text);
    const end=offset+[...value].length;
    assert.deepEqual(p.model.runs[j],{id:r.id,scalarStart:offset,scalarEnd:end});
    offset=end;plain+=value;runs++;
   }
   assert.equal(plain,p.text);assert.equal(p.boundaries.at(-1).scalarOffset,offset);
  }
 }
 return {paragraphs,runs,unprojected,cells};
}
const cases=[];const coverage={kinds:new Set(),surfaces:new Set(),groupParents:0,textHits:0,nearbyHits:0,truncated:0,emptyTextPages:0};
for(const name of readdirSync(join(root,'fixtures')).filter(n=>n.endsWith('.json')).sort()){
 const request=JSON.parse(readFileSync(join(root,'fixtures',name)));
 const material=readFileSync(join(root,'fixtures',name.replace('.json','.pptx')));
 const owner=new wasm.EditorPageSession(),messages=[],expected=[];
 function run(q,status='picked',bytes=Buffer.alloc(0),font=Buffer.alloc(0)){
  messages.push(envelope(q,bytes,font));const json=typeof q==='string'?q:JSON.stringify(q);
  const result=q.operation==='prepare'||bytes.length||font.length?take(owner.prepare(json,bytes,font,pixels,text,pixels)):
   {metadata:q.operation==='pick'?owner.pick(json,pixels):owner.command(json),pixels:Buffer.alloc(0)};
  const parsed=JSON.parse(result.metadata);assert.equal(parsed.status,status,`${name}: ${result.metadata}`);
  if(status!=='prepared')assert.equal(result.pixels.length,0);expected.push(result);return parsed;
 }
 try{
  const prepared=run(request,'prepared',material,fonts),{view,info}=prepared;catalog(info,request.request.input);
  const identities=textIdentities(info,request.request.input);
  for(const o of info.objects){coverage.kinds.add(o.kind);coverage.surfaces.add(o.surface);if(o.parent!==null)coverage.groupParents++;}
  if(!info.textFrames.length)coverage.emptyTextPages++;
  const pick=(queries,hash=view)=>({operation:'pick',view:hash,queries});
  const {width,height}=info.viewport;
  const queries=[];
  for(let y=0;y<8;y++)for(let x=0;x<8;x++)queries.push({device:{point:{x:fixed(Math.floor(width*(x+0.5)/8)),y:fixed(Math.floor(height*(y+0.5)/8))},radius:fixed(3)},maxHits:256});
  // Native caret positions supply extra sample points; the host only maps the
  // public page-EMU coordinates through the admitted viewport, never layouts.
  const carets=info.textFrames.flatMap(f=>f.paragraphs.map((_,paragraph)=>({frame:f.frame,action:{kind:'caret',position:{paragraph,position:{scalarOffset:0,affinity:'downstream'}}}})));
  if(carets.length){
   const answer=run({operation:'query',view,queries:carets.slice(0,64)},'queried');
   for(const r of answer.results){
    const axis=k=>(((BigInt(r.caret.edge[0][k])+BigInt(r.caret.edge[1][k]))/2n-BigInt(info.viewport.origin[k]))*BigInt(info.viewport.scale.numerator)/BigInt(info.viewport.scale.denominator)).toString();
    queries.push({device:{point:{x:axis('x'),y:axis('y')},radius:fixed(3)},maxHits:256});
   }
  }
  const before={...calls},results=[];
  for(let i=0;i<queries.length;i+=64){const q=queries.slice(i,i+64),r=run(pick(q));checkHits(r.results,q,info);results.push(...r.results);}
  const limited=queries.slice(0,64).map(q=>({...q,maxHits:1}));
  const short=run(pick(limited));checkHits(short.results,limited,info);
  for(const [i,r] of short.results.entries()){assert.deepEqual(r.hits,results[i].hits.slice(0,1));assert.equal(r.truncated,results[i].hits.length>1);if(r.truncated)coverage.truncated++;}
  for(const r of results)for(const h of r.hits){if(h.textFrame!==null)coverage.textHits++;if(h.kind==='nearby')coverage.nearbyHits++;}
  const first=run(pick(queries.slice(0,64)));assert.deepEqual(first.results,results.slice(0,64));
  assert.deepEqual(run(pick([])).results,[]);
  assert.equal(run(pick(queries.slice(0,1),'0'.repeat(64)),'error').error.error.code,'SOURCE_CONFLICT');
  for(const maxHits of [0,257])run(pick([{...queries[0],maxHits}]),'error');
  for(const radius of [fixed(-1),fixed(33)])run(pick([{...queries[0],device:{...queries[0].device,radius}}]),'error');
  run(pick(Array(65).fill(queries[0])),'error');
  run(pick(queries.slice(0,1)),'error',Buffer.from([1]));
  run(pick(queries.slice(0,1)),'error',undefined,Buffer.from([1]));
  run('{"operation":"pick","operation":"clear"}','error');
  for(const key of new Set([...Object.keys(before),...Object.keys(calls)])){
   if(key!=='raster.pick')assert.equal(calls[key],before[key],`picking must not call ${key}`);
  }
  assert.ok(calls['raster.pick']> (before['raster.pick']??0));
  assert.equal(JSON.parse(owner.command(JSON.stringify(pick([queries[0]])))).error.error.code,'RESOURCE_REQUIRED');
  assert.equal(JSON.parse(owner.pick(JSON.stringify(request),pixels)).error.error.code,'RESOURCE_REQUIRED');
  // A bad component is quarantined; the immutable page itself is still valid
  // and can be queried with a fresh healthy component.
  if(cases.length===0){
   let invalidations=0;
   const bad={pick(){return {status:0,words:new Uint32Array()};},invalidate(){invalidations++;}};
   const fault=JSON.parse(owner.pick(JSON.stringify(pick([queries[0]])),bad));
   assert.equal(fault.status,'error');assert.equal(fault.error.error.code,'COMPONENT_INVALID');assert.equal(invalidations,1);
   assert.deepEqual(JSON.parse(owner.pick(JSON.stringify(pick(queries.slice(0,64))),pixels)),first);
  }
  const wrong=structuredClone(request);wrong.request.page.page.expectedSourceSha256='0'.repeat(64);
  run(wrong,'error',material,fonts);assert.deepEqual(run(pick(queries.slice(0,64))),first);
  run({operation:'clear',view:'0'.repeat(64)},'error');assert.deepEqual(run(pick(queries.slice(0,64))),first);
  const resized=structuredClone(request),v=resized.request.page.page.viewport;
  v.scale.denominator*=2;v.width=Math.ceil(v.width/2);v.height=Math.ceil(v.height/2);
  const replacement=run(resized,'prepared',material,fonts);assert.notEqual(replacement.view,view);catalog(replacement.info,request.request.input);
  assert.deepEqual(replacement.info.textFrames,info.textFrames,'viewport changes preserve native/model paragraph, run and cell identities');
  assert.equal(run(pick([queries[0]]),'error').error.error.code,'SOURCE_CONFLICT');
  run(pick([queries[0]],replacement.view));
  run({operation:'clear',view:replacement.view},'cleared');run(pick([queries[0]],replacement.view),'error');
  run({operation:'clear',view:replacement.view},'error');
  assert.deepEqual(run(request,'prepared',material,fonts),prepared);assert.deepEqual(run(pick(queries.slice(0,64))),first);
  const native=spawnSync('target/release/mo-raster-worker',['--editor-page-session'],{input:Buffer.concat(messages),maxBuffer:128*1024*1024,timeout:60000});
  assert.equal(native.error,undefined);assert.equal(native.status,0,native.stderr?.toString());let offset=0;
  for(const [i,w] of expected.entries()){
   const ml=native.stdout.readUInt32LE(offset),pl=native.stdout.readUInt32LE(offset+4);offset+=8;
   assert.equal(native.stdout.subarray(offset,offset+ml).toString(),w.metadata,`${name} message ${i} metadata`);offset+=ml;
   assert.deepEqual(native.stdout.subarray(offset,offset+pl),w.pixels,`${name} message ${i} pixels`);offset+=pl;
  }
  assert.equal(offset,native.stdout.length);
  const client=new PresentationEditorPage(wasm);
  try{
   const inputs={material,fonts,decoder:pixels,shaping:text,raster:pixels};
   const frame=client.prepare(request.request,inputs);assert.deepEqual(frame.info,info);
   assert.deepEqual(client.pick(queries.slice(0,64),pixels),first.results);
   assert.throws(()=>client.pick([{...queries[0],maxHits:0}],pixels),EditorPageComputationError);
   assert.equal(client.closed,false);assert.deepEqual(client.pick(queries.slice(0,64),pixels),first.results);
   client.clear();assert.throws(()=>client.pick([queries[0]],pixels),/not been prepared/);
   if(cases.length===0){
    client.prepare(request.request,inputs);
    const reentrant={pick(words){assert.throws(()=>client.pick([queries[0]],pixels),/closed or busy/);return pixels.pick(words);},invalidate(){pixels.invalidate();}};
    assert.deepEqual(client.pick(queries.slice(0,64),reentrant),first.results);
    const closing={pick(words){client.close();return pixels.pick(words);},invalidate(){pixels.invalidate();}};
    assert.throws(()=>client.pick([queries[0]],closing),/closed during calculation/);
   }
  }finally{client.close();client.close();assert.equal(client.closed,true);}
  cases.push({name,input:request.request.input.kind,materialSha256:sha(material),requestSha256:sha(JSON.stringify(request)),view,
   objects:info.objects.length,frames:info.textFrames.length,textIdentities:identities,pickingQueries:queries.length,nativeWasmMessages:expected.length,
   pixelsSha256:sha(expected[0].pixels),responseSha256:sha(expected.map(r=>r.metadata).join('\n'))});
  console.log(`PASS ${name} (${info.objects.length} objects, ${queries.length} pick samples, ${expected.length} messages)`);
 }finally{owner.free();}
}
for(const kind of ['shape','picture','group','graphicFrame'])assert.ok(coverage.kinds.has(kind),`missing ${kind}`);
assert.ok(coverage.groupParents&&coverage.textHits&&coverage.nearbyHits&&coverage.truncated&&coverage.emptyTextPages);
coverage.kinds=[...coverage.kinds].sort();coverage.surfaces=[...coverage.surfaces].sort();
mkdirSync(join(root,'parity'),{recursive:true});
writeFileSync(join(root,'parity/report.json'),JSON.stringify({format:'musteroffice.editor-picking-parity/1',cases,coverage,componentCalls:calls,
 fontSha256:sha(fonts),nativeWorkerSha256:sha(readFileSync('target/release/mo-raster-worker')),
 wasmSha256:sha(readFileSync(join(root,'wasm-node/mo_wasm_bg.wasm'))),skiaWasmSha256:sha(readFileSync(join(skiaRoot,'mo-skia.wasm')))},null,2)+'\n');
console.log(`PASS ${cases.length} fixtures; ${cases.reduce((n,c)=>n+c.nativeWasmMessages,0)} Native/WASM messages`);
