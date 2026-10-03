// Real Rust Native/WASM computation plus an independent FontTools oracle.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root=process.argv[2];assert.ok(root,'usage: font-carets-parity.mjs evidence-directory');
const require=createRequire(import.meta.url),sha=b=>createHash('sha256').update(b).digest('hex');
const wasm=require(path.resolve(root,'wasm-node/mo_wasm.js'));
const {ShapingComponent}=await import(pathToFileURL(path.resolve(root,'text-component/index.js')));
const {default:factory}=await import(pathToFileURL(path.resolve(root,'harfbuzz/mo-hb.mjs')));
const hbBytes=readFileSync(path.join(root,'harfbuzz/mo-hb.wasm'));
const component=await ShapingComponent.create(factory,new WebAssembly.Module(hbBytes));
const output=path.join(root,'parity');mkdirSync(output,{recursive:true});
const cases=[],oracle=JSON.parse(readFileSync(path.join(root,'reference.json')));
const fontPath=name=>path.join('fixtures/fonts',name),font=name=>readFileSync(fontPath(name));
function worker(json,bytes) {
 const request=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(request.length);head.writeUInt32LE(bytes.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--carets'],{input:Buffer.concat([head,request,bytes]),maxBuffer:16*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
const instance=(glyphIds=[0,1,2,3,4,5,6],direction='leftToRight',variations=[])=>({glyphIds,direction,variations});
const query=(name='owned-carets.ttf',instances=[instance()],faceIndex=0)=>({expectedSha256:sha(font(name)),faceIndex,instances});
function run(label,q,name='owned-carets.ttf',expected='queried') {
 const json=typeof q==='string'?q:JSON.stringify(q),bytes=font(name),n=worker(json,bytes),w=wasm.font_carets(json,bytes,component);
 assert.equal(w,n,label);const r=JSON.parse(n);assert.equal(r.status==='error'?r.error.code:r.status,expected,label);
 writeFileSync(path.join(output,label+'.request.json'),json);writeFileSync(path.join(output,label+'.response.json'),n);
 cases.push({name:label,font:name,fontSha256:sha(bytes),requestSha256:sha(json),responseSha256:sha(n),status:r.status==='error'?r.error.code:r.status});return r;
}
for(const [index,c] of oracle.cases.entries()) {
 const q=query(c.name,[instance(c.glyphs.map(g=>g.glyphId),c.direction,c.variations)]);
 const r=run('oracle-'+index,q,c.name);assert.deepEqual(r.carets.instances[0].glyphs,c.glyphs,c.name+' '+index);
}
for(const name of ['owned.ttf','owned.otf','owned.ttc']) for(const face of (name==='owned.ttc'?[0,1]:[0])) {
 const r=run(name+'-'+face,query(name,[instance()],face),name);assert.ok(r.carets.instances[0].glyphs.every(g=>!g.positions.length));
}
run('empty-instances',query('owned-carets.ttf',[]));
run('empty-glyphs',query('owned-carets.ttf',[instance([])]));
run('maximum-instances',query('owned-carets.ttf',Array.from({length:64},()=>instance())));
for(const [name,change,code] of [
 ['wrong-digest',q=>q.expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],
 ['wrong-face',q=>q.faceIndex=99,'FONT_INVALID'],
 ['duplicate-glyph',q=>q.instances[0].glyphIds.push(1),'INPUT_INVALID'],
 ['unknown-glyph',q=>q.instances[0].glyphIds.push(7),'INPUT_INVALID'],
 ['unknown-direction',q=>q.instances[0].direction='sideways','INPUT_INVALID'],
 ['unknown-field',q=>q.path='hidden.ttf','INPUT_INVALID'],
 ['late-invalid-axis',q=>q.instances.push(instance([1],'leftToRight',[{tag:'xxxx',value1616:0}])),'INPUT_INVALID'],
 ['out-of-range-axis',q=>q.instances[0].variations=[{tag:'wght',value1616:901*65536}],'INPUT_INVALID'],
 ['duplicate-axis',q=>q.instances[0].variations=Array(2).fill({tag:'wght',value1616:400*65536}),'INPUT_INVALID'],
 ['too-many-instances',q=>q.instances=Array.from({length:65},()=>instance()),'LIMIT_EXCEEDED'],
 ['too-many-glyphs',q=>q.instances[0].glyphIds=Array(257).fill(1),'LIMIT_EXCEEDED'],
 ['too-many-total-glyphs',q=>q.instances=Array.from({length:17},()=>instance(Array(256).fill(1))),'LIMIT_EXCEEDED'],
 ['too-many-axes',q=>q.instances[0].variations=Array(65).fill({tag:'wght',value1616:400*65536}),'LIMIT_EXCEEDED'],
]) {const q=query();change(q);run(name,q,'owned-carets.ttf',code);}
const q=query(),json=JSON.stringify(q),bytes=font('owned-carets.ttf');
run('duplicate-key',json.replace('"faceIndex":0','"faceIndex":0,"faceIndex":0'),'owned-carets.ttf','INPUT_INVALID');
for(const [label,name,ids] of [['caret-limit','owned-carets-limit.ttf',[6]],['invalid-contour','owned-carets-invalid-point.ttf',[5]],['unsupported-contour','owned-carets-cff-point.otf',[2]]]) {
 run(label,query(name,[instance(ids)]),name,label==='caret-limit'?'LIMIT_EXCEEDED':'COMPONENT_FAILURE');
}
let calls=0,invalidations=0;
const fake={caretBatch(){calls++;throw Error('injected');},invalidate(){invalidations++;}};
const late=query('owned-carets.ttf',[instance(),instance([7])]);
assert.equal(JSON.parse(wasm.font_carets(JSON.stringify(late),bytes,fake)).error.code,'INPUT_INVALID');assert.equal(calls,0);
assert.equal(JSON.parse(wasm.font_carets(json,bytes,fake)).error.code,'HOST_FAILURE');assert.equal(calls,1);assert.equal(invalidations,1);
for(const raw of [[],[0],[0,0],[7,0],[2,99],[0,1,6,0,1,1000,64000,4,7]]) {
 let invalid=false;const r=JSON.parse(wasm.font_carets(json,bytes,{caretBatch(){return Uint32Array.from(raw);},invalidate(){invalid=true;}}));
 assert.equal(r.error.code,'COMPONENT_INVALID');assert.equal(invalid,true);
}
// Confirm the fixture really forms a three-character, single-glyph ligature.
const shape={expectedSha256:q.expectedSha256,faceIndex:0,text:'AAA',runs:[{start:0,end:3,direction:'leftToRight',script:'Latn',language:'und',clusterLevel:'monotoneGraphemes',flags:{beginningOfText:true,endOfText:true,ignorables:'default',suppressDottedCircle:false,unsafeToConcat:false,safeToInsertTatweel:false},features:[],variations:[],maxGlyphs:16}]};
const shaped=JSON.parse(wasm.shape_text(JSON.stringify(shape),bytes,component));
assert.equal(shaped.status,'shaped',JSON.stringify(shaped));
assert.equal(shaped.text.runs[0].glyphs.length,1);assert.equal(shaped.text.runs[0].glyphs[0].glyphId,6);
const requestPath=path.join(output,'cli.json');writeFileSync(requestPath,json);
const cli=spawnSync('target/release/mo-cli',['font-carets',requestPath,fontPath('owned-carets.ttf')],{encoding:'utf8',timeout:30000});
assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trimEnd(),worker(json,bytes));
const report={format:'musteroffice.font-carets-parity/1',oracle:oracle.oracle,oracleCases:oracle.cases.length,
 oraclePositions:oracle.cases.reduce((n,c)=>n+c.glyphs.reduce((a,g)=>a+g.positions.length,0),0),tolerance:0,
 nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),nativeCliSha256:sha(readFileSync('target/release/mo-cli')),
 rustWasmSha256:sha(readFileSync(path.join(root,'wasm-node/mo_wasm_bg.wasm'))),componentSha256:sha(hbBytes),
 cliVerified:true,ligatureShapingVerified:true,allInstancesPreflightVerified:true,malformedRepliesRejected:true,cases};
writeFileSync(path.join(root,'parity.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,oracleCases:report.oracleCases,oraclePositions:report.oraclePositions}));
