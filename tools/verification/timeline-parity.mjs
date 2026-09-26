/** Actual Native/WASM timeline, atomic-edit and editable-PPTX boundaries. */
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const root='.codex-work/timeline',out=root+'/product';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/timeline/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:sha(b)};};
const put=(path,b)=>{fs.writeFileSync(path,typeof b==='string'||Buffer.isBuffer(b)?b:JSON.stringify(b));return entry(path);};
const records=[];
function cli(args,input){const r=spawnSync('target/debug/mo-cli',args,{input,env:{},timeout:60000,maxBuffer:40*1024*1024});assert.equal(r.status,0,r.stderr.toString());assert.equal(r.stderr.length,0);return r.stdout.toString().trimEnd();}
function pair(name,operation,request,source=null,expected=null){
 const stem=out+'/'+records.length, json=JSON.stringify(request), req=put(stem+'.request.json',json);
 let n,w;
 if(operation==='dispatch'){n=cli([],json);w=wasm.dispatch_json(json);}
 else if(operation==='evaluate'){n=cli(['evaluate-timeline',req.path]);w=wasm.evaluate_timeline(json);}
 else{n=cli(['pptx-timing',req.path,source]);w=wasm.inspect_pptx_timing(json,fs.readFileSync(source));}
 assert.equal(n,w,name);const result=JSON.parse(n);if(expected)assert.equal(result.status==='error'?result.error.code:result.status,expected,name);
 records.push({name,operation,request:req,...(source?{source:entry(source)}:{}),response:put(stem+'.response.json',n)});return result;
}
const corpus=JSON.parse(fs.readFileSync(root+'/corpus.json'));
for(const c of corpus.cases){
 const init=pair(c.name+'/initialize','dispatch',{operation:'initialize',document:c.document},null,'initialized');
 const binding={session:'parity',revision:init.snapshot.revision,generation:'7'};
 for(const at of c.samples)pair(c.name+'/sample-'+at.ticks+'-'+at.timescale,'evaluate',{snapshot:init.snapshot,slide:c.slide,binding,at,history:{binding,through:{ticks:'100',timescale:1},events:c.events}},null,'evaluated');
}
const exp=JSON.parse(fs.readFileSync(root+'/export-request.json')),slide=exp.document.slideOrder[0];
const init=pair('export/initialize','dispatch',{operation:'initialize',document:exp.document},null,'initialized');
const binding={session:'export',revision:init.snapshot.revision,generation:'7'};
const q={snapshot:init.snapshot,slide,binding,at:{ticks:'1',timescale:1},history:{binding:{...binding},through:{ticks:'10',timescale:1},events:[]}};
for(const [name,mutate,code] of [
 ['history-required',v=>{v.history=null;},'EVENT_HISTORY_REQUIRED'],
 ['incomplete-history',v=>{v.history.through.ticks='0';},'EVENT_HISTORY_REQUIRED'],
 ['stale-history',v=>{v.history.binding.generation='8';},'EVENT_HISTORY_INVALID'],
 ['wrong-revision',v=>{v.binding.revision='0'.repeat(64);},'REVISION_CONFLICT'],
 ['tampered-snapshot',v=>{v.snapshot.document.title='tampered';},'INPUT_INVALID'],
 ['negative-time',v=>{v.at.ticks='-1';},'INPUT_INVALID'],
 ['unknown-slide',v=>{v.slide='missing';},'INPUT_INVALID'],
 ['unknown-field',v=>{v.hiddenOverride=true;},'INPUT_INVALID'],
 ['invalid-generation',v=>{v.binding.generation='18446744073709551616';},'INPUT_INVALID'],
 ['bad-sequence',v=>{v.history.events=[{generation:'7',sequence:2,at:{ticks:'0',timescale:1},event:{kind:'click',target:null}}];},'EVENT_HISTORY_INVALID'],
]){const v=structuredClone(q);mutate(v);pair(name,'evaluate',v,null,code);}
const generation=structuredClone(q);generation.binding.generation='18446744073709551615';generation.history.binding.generation=generation.binding.generation;pair('max-generation','evaluate',generation,null,'evaluated');
const target=exp.document.timelines[slide].nodes[0].effect.target;
const tx=operations=>({operation:'prepare',snapshot:init.snapshot,transaction:{documentId:exp.document.id,requestId:'animation-edit',baseRevision:init.snapshot.revision,operations:operations.map((operation,i)=>({operationId:'op:'+i,operation}))}});
pair('reject-target-delete','dispatch',tx([{kind:'deleteObject',object:target,policy:'rejectDependencies'}]),null,'REFERENCE_CONFLICT');
const deleted=pair('cascade-target-delete','dispatch',tx([{kind:'deleteObject',object:target,policy:'cascade'}]),null,'prepared');assert(!deleted.snapshot.document.objects[target]);assert(deleted.receipt.changes.changedTimelines.includes(slide));
const replacement=structuredClone(exp.document.timelines[slide]);replacement.nodes[0].effect.to=108000000;
const edited=pair('edit-multiturn','dispatch',tx([{kind:'setTimeline',slide,timeline:replacement}]),null,'prepared');assert(edited.receipt.changes.changedTimelines.includes(slide));
const exports=[];
for(const [name,document] of [['original',exp.document],['edited',edited.snapshot.document]]){
 const request={...exp,document},json=JSON.stringify(request),req=put(out+'/'+name+'.export.json',json);
 const resourcePath='fixtures/presentations/native-export/resources.bin',binary=fs.readFileSync(resourcePath),path=out+'/'+name+'.pptx';
 const response=cli(['pptx-export',req.path,resourcePath,path]);const n=fs.readFileSync(path),w=wasm.export_pptx(json,binary);assert.deepEqual(n,Buffer.from(w));
 const source=JSON.parse(wasm.inspect_pptx(n));assert.equal(source.status,'inspected');assert.equal(source.index.sourceSha256,sha(n));
 const requestTiming={expectedSourceSha256:sha(n),slide:source.index.slides[0].part};
 pair(name+'/native-timing','timing',requestTiming,path,'inspected');
 pair(name+'/wrong-source','timing',{...requestTiming,expectedSourceSha256:'0'.repeat(64)},path,'SOURCE_CONFLICT');
 pair(name+'/wrong-slide','timing',{...requestTiming,slide:'/ppt/slideMasters/slideMaster1.xml'},path,'INPUT_INVALID');
 exports.push({name,request:req,resources:entry(resourcePath),output:entry(path),response:put(out+'/'+name+'.export-response.json',response)});
 // Existing CLI publication semantics: create once, reject overwrite.
 const before=entry(path),again=spawnSync('target/debug/mo-cli',['pptx-export',req.path,resourcePath,path],{env:{},timeout:60000});assert.notEqual(again.status,0);assert.deepEqual(entry(path),before);
}
// An unrepresentable source time cannot produce a rounded PPTX or publish an output.
const invalid=structuredClone(exp);invalid.document.timelines[slide].nodes[0].duration={ticks:'1001',timescale:30000};
const badReq=put(out+'/precision.export.json',invalid),badOut=out+'/precision.pptx';
const rejected=spawnSync('target/debug/mo-cli',['pptx-export',badReq.path,'fixtures/presentations/native-export/resources.bin',badOut],{env:{},timeout:60000});assert.notEqual(rejected.status,0);assert.equal(rejected.stdout.length,0);assert(!fs.existsSync(badOut));const error=rejected.stderr.toString();assert(error.includes('MAPPING_NOT_IMPLEMENTED'));
let wasmError;try{wasm.export_pptx(JSON.stringify(invalid),fs.readFileSync('fixtures/presentations/native-export/resources.bin'));}catch(e){wasmError=String(e);}assert(wasmError?.includes('MAPPING_NOT_IMPLEMENTED'));
const oldWasm=createRequire(import.meta.url)('../../.codex-work/transform-edit/wasm-node/mo_wasm.js');
const oldRequest=fs.readFileSync('fixtures/presentations/native-export/request.json','utf8'),oldDocument=JSON.parse(oldRequest).document;
const staticInit=pair('static/initialize','dispatch',{operation:'initialize',document:oldDocument},null,'initialized');
assert.deepEqual(staticInit,JSON.parse(oldWasm.dispatch_json(JSON.stringify({operation:'initialize',document:oldDocument}))));
const staticNew=wasm.export_pptx(oldRequest,fs.readFileSync('fixtures/presentations/native-export/resources.bin'));
assert.deepEqual(Buffer.from(staticNew),Buffer.from(oldWasm.export_pptx(oldRequest,fs.readFileSync('fixtures/presentations/native-export/resources.bin'))));
const staticCompatibility={previousWasm:entry('.codex-work/transform-edit/wasm-node/mo_wasm_bg.wasm'),exportBytes:put(out+'/static.pptx',Buffer.from(staticNew)),unchangedSnapshot:true,unchangedPptx:true};
const result={staticCompatibility,format:'musteroffice.timeline-parity/1',pairedCalls:records.length,cases:records,exports,precisionFailure:{request:badReq,response:put(out+'/precision.error.txt',error),exitCode:rejected.status,wasmError,outputAbsent:true},artifacts:[entry('target/debug/mo-cli'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify({pairedCalls:records.length,exports:exports.length}));
