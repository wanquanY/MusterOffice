// Public inspect -> edit -> inspect -> prepare, through actual Native/WASM/TS.
import assert from 'node:assert/strict';
import {readFileSync,readdirSync,mkdirSync,writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const root=resolve(process.argv[2]??'.codex-work/document-inspection');
const fixtures=resolve(process.argv[3]??'.codex-work/text-identity/fixtures');
const wasm=createRequire(import.meta.url)(join(root,'wasm-node/mo_wasm.js'));
const hbRoot=resolve('.codex-work/caret/harfbuzz'),skiaRoot=resolve('.codex-work/object-picking/skia');
const {ShapingComponent}=await import(pathToFileURL(resolve('.codex-work/caret/text-component/index.js')));
const {RasterComponent}=await import(pathToFileURL(resolve('.codex-work/raster-component/index.js')));
const {default:hb}=await import(pathToFileURL(join(hbRoot,'mo-hb.mjs')));
const {default:skia}=await import(pathToFileURL(join(skiaRoot,'mo-skia.mjs')));
const shaping=await ShapingComponent.create(hb,new WebAssembly.Module(readFileSync(join(hbRoot,'mo-hb.wasm'))));
const raster=await RasterComponent.create(skia,new WebAssembly.Module(readFileSync(join(skiaRoot,'mo-skia.wasm'))));
const {PresentationEditorPage,PresentationEditor}=await import(pathToFileURL(resolve('.codex-work/editor-client/build/editor-client/src/index.js')));
const editor=new PresentationEditor(wasm),fonts=readFileSync('fixtures/fonts/owned-interaction.ttf');
const empty=Buffer.alloc(0),sha=b=>createHash('sha256').update(b).digest('hex');
const nativeCli=resolve('target/debug/mo-cli');
function nativeEdit(q){
 const p=spawnSync(nativeCli,[],{input:JSON.stringify(q),encoding:'utf8',maxBuffer:32*1024*1024,timeout:60000});
 assert.equal(p.error,undefined);assert.equal(p.status,0,p.stderr);return JSON.parse(p.stdout);
}
function inputOf(i){
 return i.kind==='author'?{kind:'author',document:i.document,defaults:i.defaults}:i;
}
function envelope(q,material,font){
 const json=Buffer.from(JSON.stringify(q)),head=Buffer.alloc(12);
 head.writeUInt32LE(json.length);head.writeUInt32LE(material.length,4);head.writeUInt32LE(font.length,8);
 return Buffer.concat([head,json,material,font]);
}
function metadata(info,input){
 assert.match(info.sourceSha256,/^[0-9a-f]{64}$/);
 assert.equal(new Set(info.slides.map(s=>s.slide)).size,info.slides.length);
 if(input.kind==='pptx'){
  assert.equal(info.model,null);assert.ok(info.slides.every(s=>s.slideId===null));return;
 }
 assert.equal(info.model.id,input.document.id);
 assert.equal(info.model.semanticDigest,editor.initialize(input.document).semanticDigest);
 assert.deepEqual(info.slides.map(s=>s.slideId),input.document.slideOrder);
 assert.deepEqual(info.pageSize,input.document.pageSize);
 for(const s of info.slides){
  const slide=input.document.slides[s.slideId];assert.equal(s.name??'',slide.name);
  assert.equal(s.hidden,slide.hidden);
  if(input.kind==='retained')assert.equal(s.slide,input.document.sourceBindings.slides[s.slideId]);
 }
}
const cases=[],coverage={authoredText:0,authoredTable:0,retainedText:0,titleOnly:0,reorders:0,negativeChannels:0};
for(const name of readdirSync(fixtures).filter(n=>n.endsWith('.json')).sort()){
 const base=JSON.parse(readFileSync(join(fixtures,name))).request;
 const material=readFileSync(join(fixtures,name.replace('.json','.pptx')));
 const owner=new wasm.EditorPageSession(),client=new PresentationEditorPage(wasm),messages=[],expected=[];
 function run(q,status,bytes=empty,font=empty){
  messages.push(envelope(q,bytes,font));let result;
  if(q.operation==='prepare'||font.length){
   const frame=owner.prepare(JSON.stringify(q),bytes,font,raster,shaping,raster);
   let consumed=false;
   try {const metadata=frame.metadata;consumed=true;result={metadata,pixels:Buffer.from(frame.take_pixels())};}
   finally{if(!consumed)frame.free();}
  }else result={metadata:q.operation==='inspect'?owner.inspect(JSON.stringify(q),bytes):owner.command(JSON.stringify(q)),pixels:empty};
  const reply=JSON.parse(result.metadata);assert.equal(reply.status,status,`${name}: ${result.metadata}`);
  if(status!=='prepared')assert.equal(result.pixels.length,0);expected.push(result);return reply;
 }
 const query=view=>({operation:'query',view,queries:[]});
 const inspectInput=inputOf(base.input),inspectionMaterial=base.input.kind==='author'?empty:material;
 try{
  const initial=run({operation:'inspect',input:inspectInput},'inspected',inspectionMaterial).info;
  metadata(initial,inspectInput);assert.equal(initial.sourceSha256,base.page.page.expectedSourceSha256);
  assert.deepEqual(client.inspect(inspectInput,inspectionMaterial),initial);assert.equal(client.view,null);
  const selected=initial.slides.find(s=>s.slide===base.page.page.slide);assert.ok(selected);
  const request=structuredClone(base);
  request.page.page.expectedSourceSha256=initial.sourceSha256;request.page.page.slide=selected.slide;
  const first=run({operation:'prepare',request},'prepared',material,fonts);
  const inputs={material,fonts,decoder:raster,shaping,raster};
  assert.deepEqual(client.prepare(request,inputs).info,first.info);
  assert.deepEqual(client.inspect(inspectInput,inspectionMaterial),initial);assert.equal(client.view,first.view);
  run(query(first.view),'queried');
  run({operation:'inspect',input:inspectInput},'error',inspectionMaterial,Buffer.from([1]));coverage.negativeChannels++;
  if(inspectInput.kind==='author'){
   run({operation:'inspect',input:inspectInput},'error',Buffer.from([1]));coverage.negativeChannels++;
  }else{
   run({operation:'inspect',input:inspectInput},'error',Buffer.from('not an OPC package'));coverage.negativeChannels++;
  }
  run(query(first.view),'queried');
  let edited=false,reordered=false,changedInfo=null;
  if(inspectInput.kind!=='pptx'&&inspectInput.document.sourceBindings?.profile!=='presentationml-retained-fields-v1-draft'){
   const original=editor.initialize(inspectInput.document);
   assert.deepEqual(original,nativeEdit({operation:'initialize',document:inspectInput.document}).snapshot);
   let snapshot,chosen;
   for(const frame of first.info.textFrames){
    const object=original.document.objects[frame.objectId];
    for(const p of frame.paragraphs){
     if(!p.model)continue;
     const run=p.model.runs.find(r=>r.scalarStart===0&&r.scalarEnd>r.scalarStart&&
      (inspectInput.kind!=='retained'||original.document.sourceBindings.objects[frame.objectId].runs[r.id].constraint===null));
     if(run){chosen={frame,p,run,object};break;}
    }
    if(chosen)break;
   }
   if(chosen){
    // The owned font covers alpha. Unlike another identical A prefix, this
    // also changes visible pixels in the narrow clipped-text fixture.
    const {frame,p,run,object}=chosen;
    if(object.content.kind==='shape'){
     const caret={paragraph:p.model.id,scalarOffset:0,affinity:'after'};
     const command={documentId:original.document.id,baseRevision:original.revision,requestId:'editor:inspect:text',operationId:'inspect:text',
      object:frame.objectId,action:{kind:'replace',selection:{anchor:caret,focus:caret},text:'αA'}};
     const result=editor.prepareText(original,command);
     assert.deepEqual(result,nativeEdit({operation:'prepareText',snapshot:original,command}).result);snapshot=result.snapshot;coverage.authoredText++;
    }else{
     const transaction={documentId:original.document.id,baseRevision:original.revision,requestId:'editor:inspect:splice',
      operations:[{operationId:'inspect:splice',operation:{kind:'spliceText',object:frame.objectId,paragraph:p.model.id,run:run.id,start:0,delete:0,insert:'αA'}}]};
     const result=editor.prepare(original,transaction);
     assert.deepEqual(result,nativeEdit({operation:'prepare',snapshot:original,transaction}));snapshot=result.snapshot;
     if(object.content.kind==='table')coverage.authoredTable++;else coverage.retainedText++;
    }
    edited=true;
   }else{
    const transaction={documentId:original.document.id,baseRevision:original.revision,requestId:'editor:inspect:title',
     operations:[{operationId:'inspect:title',operation:{kind:'setTitle',title:`${original.document.title} updated`}}]};
    const result=editor.prepare(original,transaction);assert.deepEqual(result,nativeEdit({operation:'prepare',snapshot:original,transaction}));
    snapshot=result.snapshot;coverage.titleOnly++;
   }
   if(inspectInput.kind==='author'&&snapshot.document.slideOrder.length>1){
    const transaction={documentId:snapshot.document.id,baseRevision:snapshot.revision,requestId:'editor:inspect:move',operations:[
     {operationId:'inspect:move',operation:{kind:'moveSlide',slide:selected.slideId,index:selected.slideId===snapshot.document.slideOrder[0]?1:0}},
     {operationId:'inspect:name',operation:{kind:'setSlideName',slide:selected.slideId,name:'Renamed by editor'}},
    ]};
    const result=editor.prepare(snapshot,transaction);assert.deepEqual(result,nativeEdit({operation:'prepare',snapshot,transaction}));
    snapshot=result.snapshot;coverage.reorders++;reordered=true;
   }
   const changedInput={...inspectInput,document:snapshot.document};
   changedInfo=run({operation:'inspect',input:changedInput},'inspected',inspectionMaterial).info;
   metadata(changedInfo,changedInput);assert.notEqual(changedInfo.sourceSha256,initial.sourceSha256);
   assert.equal(changedInfo.model.semanticDigest,snapshot.semanticDigest);
   assert.deepEqual(client.inspect(changedInput,inspectionMaterial),changedInfo);assert.equal(client.view,first.view);
   run(query(first.view),'queried');
   const next=structuredClone(request);next.input.document=snapshot.document;
   assert.equal(run({operation:'prepare',request:next},'error',material,fonts).error.error.code,'SOURCE_CONFLICT');
   run(query(first.view),'queried');
   const target=changedInfo.slides.find(s=>s.slideId===selected.slideId);assert.ok(target);
   if(reordered)assert.notEqual(target.slide,selected.slide);
   next.page.page.expectedSourceSha256=changedInfo.sourceSha256;next.page.page.slide=target.slide;
   const updated=run({operation:'prepare',request:next},'prepared',material,fonts);
   assert.notEqual(updated.view,first.view);assert.equal(updated.info.page.slide,target.slide);
   if(edited){
    const p=updated.info.textFrames.find(f=>f.objectId===chosen.frame.objectId&&JSON.stringify(f.cell)===JSON.stringify(chosen.frame.cell))
     .paragraphs.find(p=>p.model?.id===chosen.p.model.id);
    assert.ok(p);assert.equal(p.text,'αA'+chosen.p.text);
    assert.notEqual(sha(expected.at(-1).pixels),sha(expected.find(r=>r.pixels.length).pixels),'actual edit changes raster');
   }
   assert.deepEqual(client.prepare(next,inputs).info,updated.info);
   assert.equal(run(query(first.view),'error').error.error.code,'SOURCE_CONFLICT');
   run(query(updated.view),'queried');
   const resized=structuredClone(next),v=resized.page.page.viewport;v.scale.denominator*=2;v.width=Math.ceil(v.width/2);v.height=Math.ceil(v.height/2);
   const small=run({operation:'prepare',request:resized},'prepared',material,fonts);assert.notEqual(small.view,updated.view);
   assert.equal(small.info.page.sourceSha256,changedInfo.sourceSha256);
   assert.equal(run(query(updated.view),'error').error.error.code,'SOURCE_CONFLICT');
   run(query(small.view),'queried');
  }
  const native=spawnSync('target/release/mo-raster-worker',['--editor-page-session'],{input:Buffer.concat(messages),maxBuffer:128*1024*1024,timeout:60000});
  assert.equal(native.error,undefined);assert.equal(native.status,0,native.stderr?.toString());let offset=0;
  for(const [i,result] of expected.entries()){
   const ml=native.stdout.readUInt32LE(offset),pl=native.stdout.readUInt32LE(offset+4);offset+=8;
   assert.equal(native.stdout.subarray(offset,offset+ml).toString(),result.metadata,`${name} message ${i}`);offset+=ml;
   assert.deepEqual(native.stdout.subarray(offset,offset+pl),result.pixels,`${name} pixels ${i}`);offset+=pl;
  }
  assert.equal(offset,native.stdout.length);
  cases.push({name,kind:inspectInput.kind,slides:initial.slides.length,edited,reordered,messages:expected.length,
   materialSha256:sha(material),requestSha256:sha(JSON.stringify(base)),initialPlan:initial.sourceSha256,changedPlan:changedInfo?.sourceSha256??null,
   responseSha256:sha(expected.map(r=>r.metadata).join('\n')),pixelsSha256:sha(Buffer.concat(expected.map(r=>r.pixels)))});
  console.log(`PASS ${name} (${expected.length} messages; text edit ${edited}; reorder ${reordered})`);
 }finally{owner.free();client.close();}
}
for(const key of Object.keys(coverage))assert.ok(coverage[key]>0,`missing ${key}`);
assert.equal(cases.length,27);
mkdirSync(root,{recursive:true});
writeFileSync(join(root,'report.json'),JSON.stringify({format:'musteroffice.editor-document-parity/1',cases,coverage,
 nativeWorkerSha256:sha(readFileSync('target/release/mo-raster-worker')),nativeEditorSha256:sha(readFileSync(nativeCli)),
 wasmSha256:sha(readFileSync(join(root,'wasm-node/mo_wasm_bg.wasm'))),fontSha256:sha(fonts),
 skiaWasmSha256:sha(readFileSync(join(skiaRoot,'mo-skia.wasm'))),harfbuzzWasmSha256:sha(readFileSync(join(hbRoot,'mo-hb.wasm')))},null,2)+'\n');
console.log(`PASS ${cases.length} fixtures; ${cases.reduce((n,c)=>n+c.messages,0)} Native/WASM messages`,coverage);
