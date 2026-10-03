// Consume only the assembled public entry in an actual host Worker.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {Worker,isMainThread,parentPort,workerData} from 'node:worker_threads';
const sha=b=>createHash('sha256').update(b).digest('hex');
if(isMainThread){
 const root=resolve(process.argv[2]??'.codex-work/editor-runtime');
 const worker=new Worker(new URL(import.meta.url),{workerData:{root}});
 const messages=[];
 await new Promise((done,fail)=>{
  worker.on('error',fail);
  worker.on('message',m=>messages.push(m));
  worker.on('exit',code=>code===0?done():fail(new Error(`Editor Worker exited ${code}`)));
 });
 assert.equal(messages.length,2);
 const {report,pixels}=messages[0];
 assert.ok(pixels instanceof Uint8Array);
 assert.equal(sha(pixels),report.changedPixelsSha256);
 assert.deepEqual(messages[1],{transferred:true});
 writeFileSync(join(root,'bundle-report.json'),JSON.stringify(report,null,2)+'\n');
 console.log('PASS public editor bundle: restored revision, independent page owners, edit/reprepare, capabilities, history and real Worker transfer');
}else{
 const {root}=workerData,bundle=join(root,'playback-sdk');
 const api=await import(pathToFileURL(join(bundle,'index.mjs')));
 const code={};
 for(const [name,file] of [['kernel','mo_wasm_bg.wasm'],['raster','mo-skia.wasm'],['text','mo-hb.wasm']]){
  code[name]=await WebAssembly.compile(readFileSync(join(bundle,'runtime',file)));
 }
 const runtime=await api.createPlaybackRuntime(code);
 await assert.rejects(api.createPlaybackRuntime(code),/already initialized/);
 const base=JSON.parse(readFileSync('.codex-work/document-inspection/fixtures/author-table.json')).request;
 const fonts=readFileSync('fixtures/fonts/owned-interaction.ttf');
 const inputs={material:new Uint8Array(),fonts,decoder:runtime.raster,shaping:runtime.shaping,raster:runtime.raster};
 const original=runtime.editor.initialize(base.input.document);
 const first=runtime.createEditorPage(),second=runtime.createEditorPage();
 assert.notEqual(first,second);
 let third;
 try{
  const initial=first.prepare(base,inputs);
  const independent=second.prepare(base,inputs);
  assert.deepEqual(initial.info,independent.info);
  assert.deepEqual(initial.pixels,independent.pixels);
  const frame=initial.info.textFrames.find(f=>f.cellId&&f.paragraphs.some(p=>p.model));
  assert.ok(frame);
  const paragraph=frame.paragraphs.find(p=>p.model).model;
  const anchor={paragraph:paragraph.id,scalarOffset:0,affinity:'after'};
  const selected={anchor,focus:anchor};
  const query={object:frame.objectId,cell:frame.cellId,selection:selected};
  const caps=runtime.editor.textCapabilities(original,query);
  assert.equal(caps.replace.kind,'available');
  assert.equal(caps.revision,original.revision);
  const command={documentId:original.document.id,baseRevision:original.revision,requestId:'bundle:edit',operationId:'bundle:edit',
   object:frame.objectId,cell:frame.cellId,action:{kind:'replace',selection:selected,text:'αA'}};
  const edit=runtime.editor.prepareText(original,command);
  const restored=runtime.editor.restore(edit.snapshot);
  assert.deepEqual(restored,edit.snapshot);
  assert.notEqual(restored.revision,runtime.editor.initialize(restored.document).revision);
  const tampered=structuredClone(restored);tampered.document.title+=' forged';
  assert.throws(()=>runtime.editor.restore(tampered),api.EditorComputationError);
  assert.deepEqual(runtime.editor.restore(restored),restored);
  const input={kind:'author',defaults:base.input.defaults,document:restored.document};
  const info=first.inspect(input);
  assert.equal(info.model.semanticDigest,restored.semanticDigest);
  assert.equal(first.view,initial.view);
  const invalid=structuredClone(base);invalid.input.document=restored.document;
  assert.throws(()=>first.prepare(invalid,inputs),api.EditorPageComputationError);
  assert.equal(first.view,initial.view);assert.deepEqual(first.query([]),[]);
  const next={...invalid,page:{...invalid.page,page:{...invalid.page.page,expectedSourceSha256:info.sourceSha256,slide:info.slides[0].slide}}};
  const changed=first.prepare(next,inputs);
  assert.notEqual(sha(changed.pixels),sha(initial.pixels));
  assert.equal(second.view,independent.view);assert.deepEqual(second.query([]),[]);
  const changedFrame=changed.info.textFrames.find(f=>f.cellId===frame.cellId);
  const changedParagraph=changedFrame.paragraphs.find(p=>p.model?.id===paragraph.id);
  assert.ok(changedParagraph.text.startsWith('αA'));
  const queries=[{frame:changed.info.textFrames.indexOf(changedFrame),action:{kind:'move',position:{paragraph:changedFrame.paragraphs.indexOf(changedParagraph),position:{scalarOffset:0,affinity:'downstream'}},movement:'nextGrapheme'}}];
  assert.equal(first.query(queries)[0].caret.local.caret.position.scalarOffset,1);
  first.close();assert.equal(first.closed,true);assert.deepEqual(second.query([]),[]);
  third=runtime.createEditorPage();
  assert.equal(third.view,null);assert.deepEqual(third.inspect(input),info);
  const history={documentId:restored.document.id,baseRevision:restored.revision,requestId:'bundle:undo',direction:'undo',originalSnapshot:original,originalTransaction:edit.transaction};
  const undone=runtime.editor.prepareHistory(restored,history);
  assert.deepEqual(undone.snapshot.document,original.document);
  const report={format:'musteroffice.editor-runtime-bundle/1',worker:'node:worker_threads',
   bundleManifestSha256:sha(readFileSync(join(bundle,'bundle-manifest.json'))),
   wasmSha256:sha(readFileSync(join(bundle,'runtime/mo_wasm_bg.wasm'))),fontSha256:sha(fonts),
   originalRevision:original.revision,restoredRevision:restored.revision,undoRevision:undone.snapshot.revision,
   originalPixelsSha256:sha(initial.pixels),changedPixelsSha256:sha(changed.pixels),
   checks:['public entry only','one initialized runtime','independent page owners and close','saved revision restored','tamper rejection remains usable','snapshot-bound cell capability','high-level edit and changed raster','typed stale-plan failure retains prior view','native grapheme movement','new page after another closes','normal guarded undo','pixels transferred from real Worker']};
  parentPort.postMessage({report,pixels:changed.pixels},[changed.pixels.buffer]);
  parentPort.postMessage({transferred:changed.pixels.byteLength===0});
 }finally{first.close();second.close();third?.close();}
}
