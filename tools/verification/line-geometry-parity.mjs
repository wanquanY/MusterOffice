import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {createHarness} from './paragraph-harness.mjs';
import {verifyLineReference} from './line-reference.mjs';
const {root,fixture,wasm,component,sha,sources}=await createHarness('.codex-work/line-geometry');
const paths=new Map([[sha(sources.owned),'fixtures/fonts/owned.ttf']]);
for(const f of JSON.parse(readFileSync('fixtures/fonts/upstream.json')))if(f.name.endsWith('.ttf'))paths.set(f.sha256,`.codex-work/font-corpus/${f.family}/${f.name}`);
for(const mode of ['hhea','typo']) {const path=`fixtures/fonts/owned-metrics-${mode}.ttf`;sources[mode]=readFileSync(path);paths.set(sha(sources[mode]),path);}
const cases=[];let referenceFragments=0,referenceGlyphs=0,referenceMetrics=0;
function native(json,bundle) {
 const q=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(q.length);head.writeUInt32LE(bundle.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--geometry'],{input:Buffer.concat([head,q,bundle]),maxBuffer:80*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
function input(text,names=['notosans'],ends=[[...text].length]) {
 const f=fixture(text,names);return {bundle:f.bundle,request:{shaping:{paragraph:f.request,lineEnds:ends},styles:[{fontSize:'254000',baselineShift:'0'}],strutStyle:0,spacing:{kind:'natural'}}};
}
function run(name,f,expected='evaluated',validRequest=true) {
 const json=typeof f.request==='string'?f.request:JSON.stringify(f.request),n=native(json,f.bundle),w=wasm.layout_lines(json,f.bundle,component);assert.equal(w,n,name);
 const response=JSON.parse(n);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`,bundlePath=`${root}/bundles/${sha(f.bundle)}.bin`;
 writeFileSync(requestPath,json);writeFileSync(responsePath,n);writeFileSync(bundlePath,f.bundle);
 cases.push({name,kind:'line-geometry',validRequest,requestPath,responsePath,bundlePath,requestSha256:sha(json),responseSha256:sha(n),bundleSha256:sha(f.bundle)});
 if(response.status==='evaluated') {
  const r=response.result,q=f.request;
  const refs=verifyLineReference(q.shaping,r.shaping,paths);referenceFragments+=refs.referenceFragments;referenceGlyphs+=refs.referenceGlyphs;
  for(const m of r.metricInstances) {
   const font=q.shaping.paragraph.fonts[m.font];
   const ref=spawnSync('.codex-work/font-metrics/hb-metrics-reference',[paths.get(font.expectedSha256),String(font.faceIndex),'haschdschlgp',...m.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`)],{encoding:'utf8',timeout:30000});
   assert.equal(ref.status,0,ref.stderr);assert.deepEqual(m.measured.values.map(v=>v.position),JSON.parse(ref.stdout));referenceMetrics+=3;
  }
  assert.equal(r.layout===null,r.issues.length>0);
  if(r.layout)assert.equal(r.layout.lines.length,q.shaping.lineEnds.length);
 }
 return response;
}
for(const [name,text,names,ends] of [
 ['latin','office AV office',['notosans'],[7,16]],
 ['arabic','بب 12 بب',['notosansarabic','notosans'],[3,6,8]],
 ['bidi-isolates','A\u2067ب 12\u2069A',['notosans','notosansarabic'],[4,8]],
 ['bidi-embeddings','A\u202bب\u202c A',['notosans','notosansarabic'],[6]],
 ['line-l1-reset','α اب  12',['notosans','notosansarabic'],[5,8]],
 ['marks','A\u0301 α😀A',['owned'],[3,5,6]],
 ['devanagari','कि कि',['notosansdevanagari'],[3,5]],
 ['cjk','中文演示文稿',['notosanssc'],[2,4,6]],
 ['mixed-upem','A👩A👩A',['owned','notoemoji'],[3,5]],
 ['emoji-zwj','A👩🏽‍💻A',['owned','notoemoji'],[1,5,6]],
 ['empty','',['owned'],[0]],
 ['terminator','A\r\n',['owned'],[3]],
 ['controls','\u2028\u2029',['owned'],[1,2]],
 ['unresolved-font','A👩A',['owned'],[3]],
 ['tabs','A\tA',['owned'],[3]],
])run(name,input(text,names,ends));
for(const [name,spacing] of [['exact-tight',{kind:'exact',height:'1000'}],['at-least',{kind:'atLeast',height:'400001'}],['at-least-small',{kind:'atLeast',height:'1'}]]) {const f=input('A A',['owned'],[2,3]);f.request.spacing=spacing;run(name,f);}
const styled=input('office ب AV',['notosans','notosansarabic']);
const p=styled.request.shaping.paragraph;p.styles.push(structuredClone(p.styles[0]),structuredClone(p.styles[0]));p.spans=[{end:3,style:0},{end:8,style:1},{end:11,style:2}];
styled.request.styles=[{fontSize:'254003',baselineShift:'10001'},{fontSize:'508001',baselineShift:'-20003'},{fontSize:'127001',baselineShift:'4001'}];run('sizes-and-baselines',styled);
for(const mode of ['hhea','typo']) {
 const f=input('AAAA',['owned',mode]);const p=f.request.shaping.paragraph;
 p.styles=Array.from({length:4},(_,i)=>({...structuredClone(p.styles[0]),candidates:[{font:1,variations:[{tag:'wght',value1616:(400+i*100)*65536+1},{tag:'wdth',value1616:100*65536+1}]}]}));
 p.spans=p.styles.map((_,i)=>({end:i+1,style:i}));f.request.styles=p.styles.map((_,i)=>({fontSize:String(127003+i*23),baselineShift:String((i-1)*10003)}));
 run('mvar-'+mode,f);
}
const precision=input('A'.repeat(300),['owned']);precision.request.styles[0].fontSize='1';const precise=run('tiny-prefix-precision',precision);assert.equal(precise.result.layout.lines[0].advance,'180');
const large=input('AA',['owned']);large.request.styles[0].fontSize='9007199254740993';run('beyond-js-integer',large);
const long=input('\u2028'.repeat(4096),['owned'],Array.from({length:4096},(_,i)=>i+1));long.request.styles[0].fontSize='1';run('maximum-empty-lines',long);
for(const [name,change,code,valid=true] of [
 ['zero-size',q=>q.styles[0].fontSize='0','INPUT_INVALID'],
 ['negative-size',q=>q.styles[0].fontSize='-1','INPUT_INVALID'],
 ['numeric-size',q=>q.styles[0].fontSize=254000,'INPUT_INVALID',false],
 ['noncanonical-size',q=>q.styles[0].fontSize='01','INPUT_INVALID',false],
 ['size-range',q=>q.styles[0].fontSize='9223372036854775808','INPUT_INVALID'],
 ['overflow',q=>q.styles[0].fontSize='9223372036854775807','LIMIT_EXCEEDED'],
 ['missing-style',q=>q.styles=[],'INPUT_INVALID'],
 ['strut-outside',q=>q.strutStyle=1,'INPUT_INVALID'],
 ['zero-spacing',q=>q.spacing={kind:'exact',height:'0'},'INPUT_INVALID'],
 ['unknown-field',q=>q.path='hidden.ttf','INPUT_INVALID',false],
 ['split-grapheme',q=>{q.shaping.paragraph.text='A\u0301';q.shaping.paragraph.spans=[{end:2,style:0}];q.shaping.lineEnds=[1,2];},'INPUT_INVALID'],
]) {const f=input('AAA',['owned']);change(f.request);run(name,f,code,valid);}
const plain=input('AA',['owned']);const json=JSON.stringify(plain.request);writeFileSync(root+'/cli.json',json);writeFileSync(root+'/cli.bin',plain.bundle);
const cli=spawnSync('target/release/mo-cli',['layout-lines',root+'/cli.json',root+'/cli.bin'],{encoding:'utf8',timeout:30000});assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trimEnd(),native(json,plain.bundle));
let raw,shapeCalls=0,metricCalls=0;const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default,faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm');
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
const injecting={shapeBatch(font,frame){shapeCalls++;return fault.shapeBatch(font,frame);},measureBatch(font,frame){metricCalls++;raw._mo_hb_fail_after(0);return fault.measureBatch(font,frame);},invalidate(){fault.invalidate();}};
const failure=JSON.parse(wasm.layout_lines(json,plain.bundle,injecting));assert.equal(shapeCalls,1);assert.equal(metricCalls,1);assert.equal(failure.error.code,'COMPONENT_FAILURE');assert.equal(failure.result,undefined);assert.equal(fault.invalid,true);
const reuse=JSON.parse(wasm.layout_lines(json,plain.bundle,injecting));assert.equal(reuse.error.code,'HOST_FAILURE');assert.equal(wasm.layout_lines(json,plain.bundle,component),native(json,plain.bundle));
const report={format:'musteroffice.line-geometry-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),
 shapeReferenceSha256:sha(readFileSync('.codex-work/harfbuzz/release/hb-shape-reference')),metricsReferenceSha256:sha(readFileSync('.codex-work/font-metrics/hb-metrics-reference')),referenceFragments,referenceGlyphs,referenceMetrics,cliVerified:true,
 actualMetricFailureAfterShaping:{componentSha256:sha(faultBytes),failure,reuse,replacementVerified:true},cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,referenceFragments,referenceGlyphs,referenceMetrics}));
