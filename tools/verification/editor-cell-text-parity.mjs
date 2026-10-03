// High-level cell text intents, history and actual editor-page re-preparation.
import assert from 'node:assert/strict';
import {readFileSync,mkdirSync,writeFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const root=resolve(process.argv[2]??'.codex-work/authored-cell-text');
const fixture=resolve('.codex-work/document-inspection/fixtures/author-table.json');
const wasm=createRequire(import.meta.url)(join(root,'wasm-node/mo_wasm.js'));
const {PresentationEditor,PresentationEditorPage,EditorComputationError,EditorPageComputationError}=await import(pathToFileURL(resolve('.codex-work/editor-client/build/editor-client/src/index.js')));
const {ShapingComponent}=await import(pathToFileURL(resolve('.codex-work/caret/text-component/index.js')));
const {RasterComponent}=await import(pathToFileURL(resolve('.codex-work/raster-component/index.js')));
const hbRoot=resolve('.codex-work/caret/harfbuzz'),skiaRoot=resolve('.codex-work/object-picking/skia');
const {default:hb}=await import(pathToFileURL(join(hbRoot,'mo-hb.mjs')));
const {default:skia}=await import(pathToFileURL(join(skiaRoot,'mo-skia.mjs')));
const shaping=await ShapingComponent.create(hb,new WebAssembly.Module(readFileSync(join(hbRoot,'mo-hb.wasm'))));
const raster=await RasterComponent.create(skia,new WebAssembly.Module(readFileSync(join(skiaRoot,'mo-skia.wasm'))));
const editor=new PresentationEditor(wasm),page=new PresentationEditorPage(wasm);
const base=JSON.parse(readFileSync(fixture)).request,material=Buffer.alloc(0),fonts=readFileSync('fixtures/fonts/owned-decorations.ttf');
const fontManifest=JSON.parse(readFileSync('fixtures/fonts/decoration-manifest.json'));
const zeroUnderlineFont=readFileSync('fixtures/fonts/owned-interaction.ttf');
const sha=b=>createHash('sha256').update(b).digest('hex'),records=[],messages=[],frames=[];
let sequence=0;
function native(q){
 const p=spawnSync('target/debug/mo-cli',[],{input:JSON.stringify(q),encoding:'utf8',maxBuffer:32*1024*1024,timeout:60000});
 assert.equal(p.error,undefined);assert.equal(p.status,0,p.stderr);return JSON.parse(p.stdout);
}
function call(q,compute,status){
 const expected=native(q),before=JSON.stringify(q);assert.equal(expected.status,status,JSON.stringify(expected));
 const result=compute();assert.deepEqual(result,status==='textPrepared'?expected.result:expected);
 assert.equal(JSON.stringify(q),before);records.push({requestSha256:sha(before),responseSha256:sha(JSON.stringify(expected)),status});return result;
}
function command(snapshot,cell,action){
 const n=++sequence;
 return {documentId:snapshot.document.id,baseRevision:snapshot.revision,requestId:`cell:request:${n}`,operationId:`cell:op:${n}`,object:'shape:1',cell,action};
}
function edit(snapshot,cell,action){
 const commandValue=command(snapshot,cell,action);
 return call({operation:'prepareText',snapshot,command:commandValue},()=>editor.prepareText(snapshot,commandValue),'textPrepared');
}
function reject(snapshot,cell,action){
 const commandValue=command(snapshot,cell,action),q={operation:'prepareText',snapshot,command:commandValue},expected=native(q),view=page.view;
 assert.equal(expected.status,'error');
 assert.throws(()=>editor.prepareText(snapshot,commandValue),error=>{
  assert.ok(error instanceof EditorComputationError);assert.deepEqual(error.diagnostic,expected.error);return true;
 });
 assert.equal(page.view,view);assert.deepEqual(page.query([]),[]);
 messages.push(envelope({operation:'query',view,queries:[]},material,material));frames.push({metadata:{status:'queried',view,results:[]},pixels:material});
 records.push({requestSha256:sha(JSON.stringify(q)),responseSha256:sha(JSON.stringify(expected)),status:'error'});
}
function envelope(q,m,f){
 const json=Buffer.from(JSON.stringify(q)),head=Buffer.alloc(12);
 head.writeUInt32LE(json.length);head.writeUInt32LE(m.length,4);head.writeUInt32LE(f.length,8);return Buffer.concat([head,json,m,f]);
}
function cellBody(snapshot,cell){
 return snapshot.document.objects['shape:1'].content.table.rows.flatMap(r=>r.cells).find(c=>c.id===cell).text;
}
function textSelection(snapshot,cell,start,end){
 const p=cellBody(snapshot,cell).paragraphs[0].id;
 return {anchor:{paragraph:p,scalarOffset:start,affinity:'after'},focus:{paragraph:p,scalarOffset:end,affinity:'after'}};
}
function render(snapshot,label){
 const input={kind:'author',document:snapshot.document,defaults:base.input.defaults},info=page.inspect(input);
 assert.equal(info.model.semanticDigest,snapshot.semanticDigest);
 messages.push(envelope({operation:'inspect',input},material,material));frames.push({metadata:{status:'inspected',info},pixels:material});
 const request=structuredClone(base);request.input.document=snapshot.document;
 request.fonts=fontManifest;
 request.page.page.expectedSourceSha256=info.sourceSha256;request.page.page.slide=info.slides[0].slide;
 const image=page.prepare(request,{material,fonts,decoder:raster,shaping,raster});
 messages.push(envelope({operation:'prepare',request},material,fonts));frames.push({metadata:{status:'prepared',view:image.view,info:image.info},pixels:Buffer.from(image.pixels)});
 for(const frame of image.info.textFrames){
  assert.equal(frame.objectId,'shape:1');assert.ok(frame.cellId);
  const body=cellBody(snapshot,frame.cellId);
  for(const [i,p] of frame.paragraphs.entries()){
   if(body){assert.equal(p.model.id,body.paragraphs[i].id);assert.deepEqual(p.model.runs.map(r=>r.id),body.paragraphs[i].runs.map(r=>r.id));}
   else assert.equal(p.model,null);
  }
 }
 // Query the actual retained tab geometry through the public page API.
 const queries=[];
 for(const [frame,f] of image.info.textFrames.entries())for(const [paragraph,p] of f.paragraphs.entries()){
  const scalarOffset=[...p.text].indexOf('\t');if(scalarOffset<0)continue;
  const position=offset=>({paragraph,position:{scalarOffset:offset,affinity:'downstream'}});
  queries.push({frame,action:{kind:'selection',anchor:position(scalarOffset),focus:position(scalarOffset+1)}});
  queries.push({frame,action:{kind:'move',position:position(scalarOffset),movement:'nextGrapheme'}});
 }
 if(queries.length){
  const results=page.query(queries);
  for(let i=0;i<results.length;i+=2){
   const selection=results[i];assert.equal(selection.kind,'selection');assert.equal(selection.fragments.length,1);
   const fragment=selection.fragments[0].local.fragment;
   assert.equal(fragment.kind,'tab');assert.ok(BigInt(fragment.bounds.max.x)>BigInt(fragment.bounds.min.x));
   assert.equal(results[i+1].kind,'moved');assert.equal(results[i+1].caret.local.caret.position.scalarOffset,fragment.end.scalarOffset);
  }
  messages.push(envelope({operation:'query',view:image.view,queries},material,material));
  frames.push({metadata:{status:'queried',view:image.view,results},pixels:material});
 }
 records.push({label,view:image.view,pixelsSha256:sha(image.pixels),paragraphs:image.info.textFrames.map(f=>({cellId:f.cellId,paragraphs:f.paragraphs.map(p=>({text:p.text,model:p.model}))}))});
 return image;
}
function history(snapshot,original,result,direction){
 const transaction={documentId:snapshot.document.id,baseRevision:snapshot.revision,requestId:`cell:history:${++sequence}`,direction,
  originalSnapshot:original,originalTransaction:result.transaction};
 return call({operation:'prepareHistory',snapshot,transaction},()=>editor.prepareHistory(snapshot,transaction),'prepared').snapshot;
}
try{
 let snapshot=editor.initialize(base.input.document);
 assert.deepEqual(snapshot,native({operation:'initialize',document:base.input.document}).snapshot);
 const original=snapshot,initial=render(snapshot,'initial');
 const split=edit(snapshot,'cell:0:0',{kind:'replace',selection:textSelection(snapshot,'cell:0:0',0,0),text:'αA\r\nA😀'});
 assert.equal(split.rangeChange.cell,'cell:0:0');assert.equal(split.transaction.operations[0].operation.kind,'editTable');
 snapshot=split.snapshot;const splitImage=render(snapshot,'split');assert.notEqual(sha(splitImage.pixels),sha(initial.pixels));
 const paragraphs=cellBody(snapshot,'cell:0:0').paragraphs;
 const selection={anchor:{paragraph:paragraphs[1].id,scalarOffset:1,affinity:'after'},focus:{paragraph:paragraphs[0].id,scalarOffset:1,affinity:'after'}};
 const styled=edit(snapshot,'cell:0:0',{kind:'setCharacterStyle',selection,patch:{underline:{kind:'value',value:true}}});
 assert.deepEqual(styled.selection,selection);assert.equal(styled.rangeChange,undefined);
 // A font without usable underline metrics must reject the page and keep the
 // prior view. Positive underline probes explicitly supply the owned metric font.
 const styledInput={kind:'author',document:styled.snapshot.document,defaults:base.input.defaults};
 const styledInfo=page.inspect(styledInput),prior=page.view;
 messages.push(envelope({operation:'inspect',input:styledInput},material,material));frames.push({metadata:{status:'inspected',info:styledInfo},pixels:material});
 const bad=structuredClone(base);bad.input.document=styled.snapshot.document;bad.page.page.expectedSourceSha256=styledInfo.sourceSha256;bad.page.page.slide=styledInfo.slides[0].slide;
 assert.throws(()=>page.prepare(bad,{material,fonts:zeroUnderlineFont,decoder:raster,shaping,raster}),error=>{
  assert.ok(error instanceof EditorPageComputationError);assert.equal(error.diagnostic.text.kind,'decorationMetric');
  messages.push(envelope({operation:'prepare',request:bad},material,zeroUnderlineFont));frames.push({metadata:{status:'error',error:error.diagnostic},pixels:material});
  return true;
 });
 assert.equal(page.view,prior);assert.deepEqual(page.query([]),[]);
 snapshot=styled.snapshot;render(snapshot,'style');
 const joined=edit(snapshot,'cell:0:0',{kind:'replace',selection,text:'A'});snapshot=joined.snapshot;
 assert.equal(cellBody(snapshot,'cell:0:0').paragraphs.length,1);render(snapshot,'join');
 const template=cellBody(snapshot,'cell:0:2');
 const setup={style:template.style,insets:template.insets,wrap:template.wrap,overflow:template.overflow,
  paragraphStyle:template.paragraphs[0].style,defaultRunStyle:template.paragraphs[0].defaultRunStyle};
 const beforeInit=snapshot,beforeInitImage=render(snapshot,'before-initialize');
 assert.equal(cellBody(snapshot,'cell:2:2'),null);
 const initialized=edit(snapshot,'cell:2:2',{kind:'initialize',text:'A\u0301😀\nα\tA',setup});
 assert.equal(initialized.rangeChange,undefined);snapshot=initialized.snapshot;
 assert.equal(initialized.selection.focus.paragraph,cellBody(snapshot,'cell:2:2').paragraphs[1].id);
 assert.equal(initialized.selection.focus.scalarOffset,3);
 const initializedImage=render(snapshot,'initialize');assert.notEqual(sha(initializedImage.pixels),sha(beforeInitImage.pixels));
 for(const cell of ['cell:0:1','cell:missing'])reject(snapshot,cell,{kind:'replace',selection:textSelection(snapshot,'cell:0:0',0,0),text:'A'});
 reject(snapshot,undefined,{kind:'replace',selection:textSelection(snapshot,'cell:0:0',0,0),text:'A'});
 reject(snapshot,'cell:0:2',{kind:'replace',selection:textSelection(snapshot,'cell:0:0',0,0),text:'A'});
 reject(snapshot,'cell:2:2',{kind:'replace',selection:textSelection(snapshot,'cell:2:2',1,1),text:'A'});
 reject(snapshot,'cell:2:2',{kind:'initialize',text:'A',setup});
 const undo=history(snapshot,beforeInit,initialized,'undo');assert.deepEqual(undo.document,beforeInit.document);
 assert.deepEqual(render(undo,'undo-initialize').pixels,beforeInitImage.pixels);
 const redo=history(undo,beforeInit,initialized,'redo');assert.deepEqual(redo.document,snapshot.document);
 assert.deepEqual(render(redo,'redo-initialize').pixels,initializedImage.pixels);
 // Ordinary guarded history refuses a changed table declaration atomically.
 const conflict={documentId:snapshot.document.id,baseRevision:snapshot.revision,requestId:'history:overlap',direction:'undo',originalSnapshot:original,originalTransaction:split.transaction};
 assert.equal(native({operation:'prepareHistory',snapshot,transaction:conflict}).error.code,'REFERENCE_CONFLICT');
 assert.throws(()=>editor.prepareHistory(snapshot,conflict),EditorComputationError);
 const processResult=spawnSync('target/release/mo-raster-worker',['--editor-page-session'],{input:Buffer.concat(messages),maxBuffer:128*1024*1024,timeout:60000});
 assert.equal(processResult.error,undefined);assert.equal(processResult.status,0,processResult.stderr?.toString());let offset=0;
 for(const [i,frame] of frames.entries()){
  const ml=processResult.stdout.readUInt32LE(offset),pl=processResult.stdout.readUInt32LE(offset+4);offset+=8;
  assert.deepEqual(JSON.parse(processResult.stdout.subarray(offset,offset+ml)),frame.metadata,`frame ${i}`);offset+=ml;
  assert.deepEqual(processResult.stdout.subarray(offset,offset+pl),frame.pixels,`pixels ${i}`);offset+=pl;
 }
 assert.equal(offset,processResult.stdout.length);
 mkdirSync(root,{recursive:true});
 writeFileSync(join(root,'report.json'),JSON.stringify({format:'musteroffice.editor-cell-text-parity/1',records,pageMessages:frames.length,
  fixtureSha256:sha(readFileSync(fixture)),nativeEditorSha256:sha(readFileSync('target/debug/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-raster-worker')),
  wasmSha256:sha(readFileSync(join(root,'wasm-node/mo_wasm_bg.wasm'))),fontSha256:sha(fonts),fontManifestSha256:sha(JSON.stringify(fontManifest)),zeroUnderlineFontSha256:sha(zeroUnderlineFont),
  skiaWasmSha256:sha(readFileSync(join(skiaRoot,'mo-skia.wasm'))),harfbuzzWasmSha256:sha(readFileSync(join(hbRoot,'mo-hb.wasm')))},null,2)+'\n');
 console.log(`PASS ${records.length} edit/error/render records, ${frames.length} Native/WASM page messages; split/style/join/initialize/undo/redo and rejected targets`);
}finally{page.close();}
