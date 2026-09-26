import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/font-metrics';mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const binary=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm');
const component=await ShapingComponent.create(factory,new WebAssembly.Module(binary));
export const metrics=[
 ['horizontalAscender','hasc'],['horizontalDescender','hdsc'],['horizontalLineGap','hlgp'],
 ['horizontalClippingAscent','hcla'],['horizontalClippingDescent','hcld'],
 ['verticalAscender','vasc'],['verticalDescender','vdsc'],['verticalLineGap','vlgp'],
 ['horizontalCaretRise','hcrs'],['horizontalCaretRun','hcrn'],['horizontalCaretOffset','hcof'],
 ['verticalCaretRise','vcrs'],['verticalCaretRun','vcrn'],['verticalCaretOffset','vcof'],
 ['xHeight','xhgt'],['capHeight','cpht'],
 ['subscriptXSize','sbxs'],['subscriptYSize','sbys'],['subscriptXOffset','sbxo'],['subscriptYOffset','sbyo'],
 ['superscriptXSize','spxs'],['superscriptYSize','spys'],['superscriptXOffset','spxo'],['superscriptYOffset','spyo'],
 ['strikeoutSize','strs'],['strikeoutOffset','stro'],['underlineSize','unds'],['underlineOffset','undo'],
];
const tags=new Map(metrics),cases=[];let referenceInstances=0,referenceValues=0;
function worker(json,font) {
 const req=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(req.length);head.writeUInt32LE(font.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--metrics'],{input:Buffer.concat([head,req,font]),maxBuffer:20*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
const instance=(variations=[])=>({metrics:metrics.map(m=>m[0]),variations});
const axis=(tag,value)=>({tag,value1616:Math.round(value*65536)});
const request=(font,instances=[instance()],faceIndex=0)=>({expectedSha256:sha(readFileSync(font)),faceIndex,instances});
function run(name,q,fontPath,expected='measured',validRequest=true) {
 const json=typeof q==='string'?q:JSON.stringify(q),font=readFileSync(fontPath);
 const native=worker(json,font),web=wasm.measure_font(json,font,component);assert.equal(web,native,name);
 const response=JSON.parse(native);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`;
 writeFileSync(requestPath,json);writeFileSync(responsePath,native);
 cases.push({name,fontPath,fontSha256:sha(font),validRequest,requestPath,responsePath,requestSha256:sha(json),responseSha256:sha(native)});
 if(expected==='measured') for(let i=0;i<q.instances.length;i++) {
  const input=q.instances[i],args=[fontPath,String(q.faceIndex),input.metrics.map(m=>tags.get(m)).join(''),...input.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`)];
  const ref=spawnSync(root+'/hb-metrics-reference',args,{encoding:'utf8',timeout:30000});assert.equal(ref.status,0,ref.stderr);
  assert.deepEqual(response.metrics.instances[i].values.map(v=>v.position),JSON.parse(ref.stdout),name);
  referenceInstances++;referenceValues+=input.metrics.length;
 }
 return response;
}
const owned='fixtures/fonts/owned.ttf';
for(const file of ['owned.ttf','owned.otf','owned.ttc']) {
 const path='fixtures/fonts/'+file;
 for(let face=0;face<(file.endsWith('.ttc')?2:1);face++)run(file+'-'+face,request(path,undefined,face),path);
}
const variations=[[],[axis('wght',900)],[axis('wdth',125)],[axis('wght',650),axis('wdth',112.5)],
 [axis('wght',100),axis('wdth',75)],[axis('wght',700+1/65536),axis('wdth',99+1/65536)]];
for(const mode of ['hhea','typo']) {
 const path=`fixtures/fonts/owned-metrics-${mode}.ttf`;
 const r=run('mvar-'+mode,request(path,variations.map(instance)),path);
 assert.notDeepEqual(r.metrics.instances[0].values,r.metrics.instances[1].values);
 assert.equal(r.metrics.instances[0].values[0].position,(mode==='hhea'?800:750)*64);
}
for(const f of JSON.parse(readFileSync('fixtures/fonts/upstream.json')))if(f.name.endsWith('.ttf')) {
 const path=`.codex-work/font-corpus/${f.family}/${f.name}`;assert.equal(sha(readFileSync(path)),f.sha256);
 const q=request(path,[instance(),instance([axis('wght',700)])]);run(f.family,q,path);
}
// These derivatives remove only optional tables from the project's owned fonts.
const variants=JSON.parse(readFileSync(root+'/font-variants.json'));
for(const v of variants) {
 const r=run(v.name,request(v.path),v.path);
 for(const m of v.missing)assert.equal(r.metrics.instances[0].values.find(x=>x.metric===m).position,null,v.name+' '+m);
 for(const m of v.zero)assert.equal(r.metrics.instances[0].values.find(x=>x.metric===m).position,0,v.name+' '+m);
}
run('empty-instances',request(owned,[]),owned);
run('empty-metrics',request(owned,[{metrics:[],variations:[]}]),owned);
run('maximum-instances',request(owned,Array.from({length:256},()=>instance())),owned);
for(const [name,change,code,valid=true] of [
 ['wrong-digest',q=>q.expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],
 ['wrong-face',q=>q.faceIndex=99,'FONT_INVALID'],
 ['duplicate-metric',q=>q.instances[0].metrics.push('xHeight'),'INPUT_INVALID'],
 ['unknown-metric',q=>q.instances[0].metrics=['invented'],'INPUT_INVALID',false],
 ['unknown-field',q=>q.path='hidden.ttf','INPUT_INVALID',false],
 ['late-invalid-instance',q=>q.instances.push(instance([axis('xxxx',4)])),'INPUT_INVALID'],
 ['duplicate-axis',q=>q.instances[0].variations=[axis('wght',400),axis('wght',500)],'INPUT_INVALID'],
 ['out-of-range-axis',q=>q.instances[0].variations=[axis('wght',901)],'INPUT_INVALID'],
 ['too-many-instances',q=>q.instances=Array.from({length:257},()=>instance()),'LIMIT_EXCEEDED'],
 ['too-many-metrics',q=>q.instances[0].metrics.push('xHeight'),'LIMIT_EXCEEDED'],
 ['too-many-axes',q=>q.instances[0].variations=Array.from({length:65},()=>axis('wght',400)),'LIMIT_EXCEEDED'],
]) {
 const q=request(owned);if(name==='duplicate-metric')q.instances[0].metrics=['xHeight'];change(q);run(name,q,owned,code,valid);
}
const q=request(owned),json=JSON.stringify(q),font=readFileSync(owned);
run('duplicate-key',json.replace('"faceIndex":0','"faceIndex":0,"faceIndex":0'),owned,'INPUT_INVALID',false);
let calls=0,invalidations=0;
const fake={measureBatch(){calls++;throw Error('injected transport failure');},invalidate(){invalidations++;}};
const late=request(owned,[instance(),instance([axis('xxxx',0)])]);
assert.equal(JSON.parse(wasm.measure_font(JSON.stringify(late),font,fake)).error.code,'INPUT_INVALID');assert.equal(calls,0);
assert.equal(JSON.parse(wasm.measure_font(json,font,fake)).error.code,'HOST_FAILURE');assert.equal(calls,1);assert.equal(invalidations,1);
for(const raw of [[],[0],[0,0],[7,0],[2,99],[0,1,6,0,1,1000,64000,0,0]]) {
 let invalid=false;const r=JSON.parse(wasm.measure_font(json,font,{measureBatch(){return Uint32Array.from(raw);},invalidate(){invalid=true;}}));
 assert.equal(r.error.code,'COMPONENT_INVALID');assert.equal(invalid,true);
}
writeFileSync(root+'/cli.json',json);
const cli=spawnSync('target/release/mo-cli',['font-metrics',root+'/cli.json',owned],{encoding:'utf8',timeout:30000});assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trimEnd(),worker(json,font));
const report={format:'musteroffice.font-metrics-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),
 rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(binary),referenceSha256:sha(readFileSync(root+'/hb-metrics-reference')),
 referenceInstances,referenceValues,cliVerified:true,malformedTransportVerified:true,allInstancesPreflightVerified:true,cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,referenceInstances,referenceValues}));
