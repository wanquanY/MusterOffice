// Real shared Native/WASM editor owners, immutable queries and pixel parity.
import assert from 'node:assert/strict';
import {readFileSync, readdirSync, mkdirSync, writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {resolve, join} from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const root=resolve(process.argv[2]??'.codex-work/page-interaction');
const wasm=createRequire(import.meta.url)(join(root,'wasm-node/mo_wasm.js'));
const {ShapingComponent}=await import(pathToFileURL(resolve('.codex-work/caret/text-component/index.js')));
const {RasterComponent}=await import(pathToFileURL(resolve('.codex-work/raster-component/index.js')));
const hbRoot=resolve('.codex-work/caret/harfbuzz');
const skiaRoot=resolve('.codex-work/font-fallback-20261003/playback/runtime');
const {default:hb}=await import(pathToFileURL(join(hbRoot,'mo-hb.mjs')));
const {default:skia}=await import(pathToFileURL(join(skiaRoot,'mo-skia.mjs')));
const shaping=await ShapingComponent.create(hb,new WebAssembly.Module(readFileSync(join(hbRoot,'mo-hb.wasm'))));
const raster=await RasterComponent.create(skia,new WebAssembly.Module(readFileSync(join(skiaRoot,'mo-skia.wasm'))));
const {PresentationEditorPage,EditorPageComputationError}=await import(pathToFileURL(resolve('.codex-work/editor-client/build/editor-client/src/index.js')));
const fonts=readFileSync('fixtures/fonts/owned-interaction.ttf');
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const cases=[];
const position=(paragraph,scalarOffset,affinity='downstream')=>({paragraph,position:{scalarOffset,affinity}});
let componentCalls=0;
const counted=component=>new Proxy(component,{get(target,key){
 const value=Reflect.get(target,key,target);
 return typeof value==='function'?(...args)=>{componentCalls++;return value.apply(target,args);}:value;
}});
const text=counted(shaping),pixels=counted(raster);
function envelope(request,material=Buffer.alloc(0),font=Buffer.alloc(0)) {
 const json=Buffer.from(typeof request==='string'?request:JSON.stringify(request));
 const head=Buffer.alloc(12);head.writeUInt32LE(json.length);head.writeUInt32LE(material.length,4);head.writeUInt32LE(font.length,8);
 return Buffer.concat([head,json,material,font]);
}
function take(frame) {
 let consumed=false;
 try {const metadata=frame.metadata;consumed=true;return {metadata,pixels:Buffer.from(frame.take_pixels())};}
 finally {if(!consumed)frame.free();}
}
for(const name of readdirSync(join(root,'fixtures')).filter(n=>n.endsWith('.json')).sort()) {
 const request=JSON.parse(readFileSync(join(root,'fixtures',name)));
 const material=readFileSync(join(root,'fixtures',name.replace('.json','.pptx')));
 const owner=new wasm.EditorPageSession(),messages=[],expected=[];
 function run(q,bytes=Buffer.alloc(0),font=Buffer.alloc(0),status) {
  messages.push(envelope(q,bytes,font));
  const json=typeof q==='string'?q:JSON.stringify(q);
  const result=q.operation==='prepare'||bytes.length||font.length?
   take(owner.prepare(json,bytes,font,pixels,text,pixels)):
   {metadata:owner.command(json),pixels:Buffer.alloc(0)};
  const parsed=JSON.parse(result.metadata);
  assert.equal(parsed.status,status,`${name}: ${result.metadata}`);
  if(status==='error')assert.equal(result.pixels.length,0);
  expected.push(result);return parsed;
 }
 try {
  const prepared=run(request,material,fonts,'prepared'),view=prepared.view;
  const query=(queries,hash=view)=>({operation:'query',view:hash,queries});
  const queries=prepared.info.textFrames.flatMap(f=>f.paragraphs.flatMap((p,i)=>{
   const end=p.boundaries.at(-1).scalarOffset;
   return [
    {frame:f.frame,action:{kind:'caret',position:position(i,0)}},
    {frame:f.frame,action:{kind:'caret',position:position(i,end,'upstream')}},
    {frame:f.frame,action:{kind:'selection',anchor:position(i,end),focus:position(i,0)}},
   ];
  }));
  assert.ok(queries.length>0 && queries.length<=64);
  const before=componentCalls;
  const answered=run(query(queries),undefined,undefined,'queried');
  const hits=answered.results.filter(r=>r.kind==='caret').map(r=>({frame:r.frame,action:{kind:'hit',point:{
   x:((BigInt(r.caret.edge[0].x)+BigInt(r.caret.edge[1].x))/2n).toString(),
   y:((BigInt(r.caret.edge[0].y)+BigInt(r.caret.edge[1].y))/2n).toString(),
  }}}));
  run(query(hits),undefined,undefined,'queried');
  const cross=prepared.info.textFrames.filter(f=>f.paragraphs.length>1).map(f=>({frame:f.frame,action:{kind:'selection',
   anchor:position(f.paragraphs.length-1,f.paragraphs.at(-1).boundaries.at(-1).scalarOffset),focus:position(0,0)}}));
  if(cross.length)run(query(cross),undefined,undefined,'queried');
  for(let i=0;i<10;i++)assert.deepEqual(run(query(queries),undefined,undefined,'queried'),answered);
  run(query(queries,'0'.repeat(64)),undefined,undefined,'error');
  run(query([{frame:0,action:{kind:'caret',position:position(999,0)}}]),undefined,undefined,'error');
  run(query(Array(65).fill(queries[0])),undefined,undefined,'error');
  run(query(queries),Buffer.from([1]),undefined,'error');
  run('{"operation":"query","operation":"clear"}',undefined,undefined,'error');
  assert.equal(componentCalls,before,'queries never re-enter font, decoder or raster components');
  const wrong=structuredClone(request);wrong.request.page.page.expectedSourceSha256='0'.repeat(64);
  run(wrong,material,fonts,'error');
  assert.deepEqual(run(query(queries),undefined,undefined,'queried'),answered,'rejected preparation preserves owner');
  const fontFault=structuredClone(request);fontFault.request.fonts.fonts[0].expectedSha256='0'.repeat(64);
  run(fontFault,material,fonts,'error');
  assert.deepEqual(run(query(queries),undefined,undefined,'queried'),answered);
  run({operation:'clear',view:'0'.repeat(64)},undefined,undefined,'error');
  assert.deepEqual(run(query(queries),undefined,undefined,'queried'),answered);
  const resized=structuredClone(request),v=resized.request.page.page.viewport;
  v.scale.denominator*=2;v.width=Math.ceil(v.width/2);v.height=Math.ceil(v.height/2);
  const replacement=run(resized,material,fonts,'prepared');assert.notEqual(replacement.view,view);
  run(query(queries),undefined,undefined,'error');
  const newResults=run(query(queries,replacement.view),undefined,undefined,'queried');
  // Precision certificates round outward in device pixels, so their page-EMU
  // bounds can change with pixel scale even when every coordinate is identical.
  const geometry=(results,viewport)=>JSON.parse(JSON.stringify(results,(key,value)=>{
   if(key!=='coordinateErrorBound')return value;
   assert.ok(BigInt(value)>=0n && BigInt(value)*BigInt(viewport.scale.numerator)<=
    BigInt(viewport.coordinateTolerance)*BigInt(viewport.scale.denominator));
   return undefined;
  }));
  assert.deepEqual(geometry(newResults.results,v),geometry(answered.results,request.request.page.page.viewport),
   'page EMU queries survive viewport changes');
  run({operation:'clear',view:replacement.view},undefined,undefined,'cleared');
  run(query(queries),undefined,undefined,'error');
  run({operation:'clear',view:replacement.view},undefined,undefined,'error');
  const native=spawnSync('target/release/mo-raster-worker',['--editor-page-session'],{
   input:Buffer.concat(messages),maxBuffer:128*1024*1024,timeout:60000,
  });
  assert.equal(native.error,undefined);assert.equal(native.status,0,native.stderr?.toString());
  let offset=0;
  for(const [i,w] of expected.entries()) {
   const ml=native.stdout.readUInt32LE(offset),pl=native.stdout.readUInt32LE(offset+4);offset+=8;
   assert.equal(native.stdout.subarray(offset,offset+ml).toString(),w.metadata,`${name} message ${i} metadata`);offset+=ml;
   assert.deepEqual(native.stdout.subarray(offset,offset+pl),w.pixels,`${name} message ${i} pixels`);offset+=pl;
  }
  assert.equal(offset,native.stdout.length);
  const client=new PresentationEditorPage(wasm);
  try {
   const frame=client.prepare(request.request,{material,fonts,decoder:pixels,shaping:text,raster:pixels});
   assert.equal(frame.view,view);assert.deepEqual(frame.info,prepared.info);assert.deepEqual(Buffer.from(frame.pixels),expected[0].pixels);
   assert.deepEqual(client.query(queries),answered.results);
   assert.throws(()=>client.prepare(wrong.request,{material,fonts,decoder:pixels,shaping:text,raster:pixels}),EditorPageComputationError);
   assert.equal(client.closed,false);assert.deepEqual(client.query(queries),answered.results);
   client.clear();assert.equal(client.view,null);assert.throws(()=>client.query(queries));
  } finally {client.close();client.close();assert.equal(client.closed,true);}
  if(request.request.input.kind!=='pptx')assert.ok(prepared.info.textFrames.every(f=>typeof f.objectId==='string'));
  cases.push({name,input:request.request.input.kind,materialSha256:sha(material),requestSha256:sha(JSON.stringify(request)),view,
   nativeWasmMessages:expected.length,frames:prepared.info.textFrames.length,pixelsSha256:sha(expected[0].pixels),
   responseSha256:sha(expected.map(r=>r.metadata).join('\n'))});
  console.log(`PASS ${name} (${expected.length} messages, ${prepared.info.textFrames.length} frames)`);
 } finally {owner.free();}
}
mkdirSync(join(root,'parity'),{recursive:true});
writeFileSync(join(root,'parity/report.json'),JSON.stringify({format:'musteroffice.editor-page-parity/1',cases,
 fontSha256:sha(fonts),nativeWorkerSha256:sha(readFileSync('target/release/mo-raster-worker')),
 wasmSha256:sha(readFileSync(join(root,'wasm-node/mo_wasm_bg.wasm')))},null,2)+'\n');
console.log(`PASS ${cases.length} fixtures; ${cases.reduce((n,c)=>n+c.nativeWasmMessages,0)} Native/WASM messages`);
