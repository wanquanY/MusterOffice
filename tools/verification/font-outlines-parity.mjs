import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/font-outlines';mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const binary=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm');
const component=await ShapingComponent.create(factory,new WebAssembly.Module(binary));
const cases=[];let referenceInstances=0,referenceGlyphs=0,referenceCommands=0;
function worker(json,font) {
 const req=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(req.length);head.writeUInt32LE(font.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--outlines'],{input:Buffer.concat([head,req,font]),maxBuffer:40*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
const instance=(glyphIds=[0,1,2,3],variations=[])=>({glyphIds,variations,maxCommands:4096,maxOperations:1048576});
const axis=(tag,value)=>({tag,value1616:Math.round(value*65536)});
const request=(font,instances=[instance()],faceIndex=0)=>({expectedSha256:sha(readFileSync(font)),faceIndex,instances});
function record(c){switch(c.kind){case'move':return[1,c.to.x,c.to.y,0,0,0,0];case'line':return[2,c.to.x,c.to.y,0,0,0,0];case'quadratic':return[3,c.control.x,c.control.y,c.to.x,c.to.y,0,0];case'cubic':return[4,c.control1.x,c.control1.y,c.control2.x,c.control2.y,c.to.x,c.to.y];case'close':return[5,0,0,0,0,0,0];default:throw Error('unknown command');}}
function run(name,q,fontPath,expected='outlined',validRequest=true) {
 const json=typeof q==='string'?q:JSON.stringify(q),font=readFileSync(fontPath);
 const native=worker(json,font),web=wasm.outline_font(json,font,component);assert.equal(web,native,name);
 const response=JSON.parse(native);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`;
 writeFileSync(requestPath,json);writeFileSync(responsePath,native);
 cases.push({name,fontPath,fontSha256:sha(font),validRequest,requestPath,responsePath,requestSha256:sha(json),responseSha256:sha(native)});
 if(expected==='outlined') for(let i=0;i<q.instances.length;i++) {
  const input=q.instances[i],args=[fontPath,String(q.faceIndex),input.glyphIds.join(','),...input.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`)];
  const ref=spawnSync(root+'/hb-outlines-reference',args,{encoding:'utf8',timeout:30000,maxBuffer:40*1024*1024});assert.equal(ref.status,0,ref.stderr);
  const paths=response.outlines.instances[i].glyphs.map(g=>g.path?.map(record)??null);assert.deepEqual(paths,JSON.parse(ref.stdout),name);
  referenceInstances++;referenceGlyphs+=paths.length;referenceCommands+=paths.reduce((a,p)=>a+(p?.length??0),0);
 }
 return response;
}
const owned='fixtures/fonts/owned.ttf';
for(const file of ['owned.ttf','owned.otf','owned.ttc']) {
 const path='fixtures/fonts/'+file;
 for(let face=0;face<(file.endsWith('.ttc')?2:1);face++)run(file+'-'+face,request(path,undefined,face),path);
}
const curved='fixtures/fonts/owned-outlines.ttf',cff='fixtures/fonts/owned-outlines.otf';
const curves=run('owned-quadratic-variable',request(curved,[instance(),instance(undefined,[axis('wght',900)]),instance([3,2,2,1],[axis('wght',650)])]),curved);
assert.deepEqual(curves.outlines.instances[0].glyphs[1].path,[]);
assert.notDeepEqual(curves.outlines.instances[0].glyphs[2].path,curves.outlines.instances[1].glyphs[2].path);
run('owned-cubic',request(cff),cff);
const no='.codex-work/font-outlines/no-outline.ttf';const missing=run('unavailable-outline',request(no),no);assert.ok(missing.outlines.instances[0].glyphs.every(g=>g.path===null));
for(const f of JSON.parse(readFileSync('fixtures/fonts/upstream.json')))if(f.name.endsWith('.ttf')) {
 const path=`.codex-work/font-corpus/${f.family}/${f.name}`;assert.equal(sha(readFileSync(path)),f.sha256);
 run(f.family,request(path,[instance([0,1,2,3,10,20,40,99]),instance([0,1,2,3,10,20,40,99],[axis('wght',700)])]),path);
}
run('empty-instances',request(owned,[]),owned);
run('empty-glyphs',request(owned,[instance([])]),owned);
run('maximum-instances',request(owned,Array.from({length:64},()=>instance([1]))),owned);
run('maximum-glyphs',request(owned,Array.from({length:16},()=>instance(Array(256).fill(2)))),owned);
run('zero-commands-empty',request(owned,[{...instance([1]),maxCommands:0}]),owned);
for(const [name,change,code,valid=true] of [
 ['wrong-digest',q=>q.expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],
 ['wrong-face',q=>q.faceIndex=99,'FONT_INVALID'],
 ['invalid-glyph',q=>q.instances[0].glyphIds.push(7),'INPUT_INVALID'],
 ['unknown-field',q=>q.path='hidden.ttf','INPUT_INVALID',false],
 ['late-invalid-instance',q=>q.instances.push(instance([2],[axis('xxxx',4)])),'INPUT_INVALID'],
 ['duplicate-axis',q=>q.instances[0].variations=[axis('wght',400),axis('wght',500)],'INPUT_INVALID'],
 ['out-of-range-axis',q=>q.instances[0].variations=[axis('wght',901)],'INPUT_INVALID'],
 ['too-many-instances',q=>q.instances=Array.from({length:65},()=>instance()),'LIMIT_EXCEEDED'],
 ['too-many-glyphs',q=>q.instances[0].glyphIds=Array(257).fill(2),'LIMIT_EXCEEDED'],
 ['too-many-axes',q=>q.instances[0].variations=Array.from({length:65},()=>axis('wght',400)),'LIMIT_EXCEEDED'],
 ['aggregate-glyphs',q=>q.instances=Array.from({length:17},()=>instance(Array(256).fill(1))),'LIMIT_EXCEEDED'],
 ['aggregate-commands',q=>q.instances[0].maxCommands=262145,'LIMIT_EXCEEDED'],
 ['zero-work',q=>q.instances[0].maxOperations=0,'LIMIT_EXCEEDED'],
 ['too-much-work',q=>q.instances[0].maxOperations=1048577,'LIMIT_EXCEEDED'],
 ['command-budget',q=>q.instances[0].maxCommands=0,'LIMIT_EXCEEDED'],
 ['interpreter-budget',q=>q.instances[0].maxOperations=1,'LIMIT_EXCEEDED'],
]) {const q=request(owned);change(q);run(name,q,owned,code,valid);}
const q=request(owned),json=JSON.stringify(q),font=readFileSync(owned);
run('duplicate-key',json.replace('"faceIndex":0','"faceIndex":0,"faceIndex":0'),owned,'INPUT_INVALID',false);
let calls=0,invalidations=0;
const fake={outlineBatch(){calls++;throw Error('injected transport failure');},invalidate(){invalidations++;}};
const late=request(owned,[instance(),instance([7])]);assert.equal(JSON.parse(wasm.outline_font(JSON.stringify(late),font,fake)).error.code,'INPUT_INVALID');assert.equal(calls,0);
assert.equal(JSON.parse(wasm.outline_font(json,font,fake)).error.code,'HOST_FAILURE');assert.equal(calls,1);assert.equal(invalidations,1);
for(const raw of [[],[0],[0,0],[7,0],[2,99],[0,1,6,0,1,1000,64000,0,0]]) {
 let invalid=false;const r=JSON.parse(wasm.outline_font(json,font,{outlineBatch(){return Uint32Array.from(raw);},invalidate(){invalid=true;}}));
 assert.equal(r.error.code,'COMPONENT_INVALID');assert.equal(invalid,true);
}
assert.equal(component.invalid,false);
const cliRequest=root+'/owned-quadratic-variable.request.json';
const cli=spawnSync('target/release/mo-cli',['font-outlines',cliRequest,curved],{encoding:'utf8',timeout:30000});
assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trim(),readFileSync(root+'/owned-quadratic-variable.response.json','utf8'));
// Invalid private transport is a host defect and permanently discards that module.
let rejectedTransports=0;
for(const frame of [[0,1,0x0e0500,0],[0x4d4f4f42,1,0x0e0500,65],[0x4d4f4f42,1,0x0e0500,1,8],[0x4d4f4f42,1,0x0e0500,0,0]]) {
 const isolated=await ShapingComponent.create(factory,new WebAssembly.Module(binary));
 assert.throws(()=>isolated.outlineBatch(font,Uint32Array.from(frame)));assert.equal(isolated.invalid,true);rejectedTransports++;
}
const report={format:'musteroffice.font-outlines-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,referenceInstances,referenceGlyphs,referenceCommands,referenceEntry:'separate upstream API, same HarfBuzz algorithm',componentWasmSha256:sha(binary),referenceSha256:sha(readFileSync(root+'/hb-outlines-reference')),cliWorkerVerified:true,rejectedTransports,lateInputPreflight:true,malformedReplyQuarantine:true};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,referenceInstances,referenceGlyphs,referenceCommands}));
