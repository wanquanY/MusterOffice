import {verifyLineReference} from './line-reference.mjs';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import {createHarness} from './paragraph-harness.mjs';
const h=await createHarness('.codex-work/line-shaping');
const {root,fixture,wasm,component,sha}=h,cases=[];
const sources=new Map([[sha(readFileSync('fixtures/fonts/owned.ttf')),'fixtures/fonts/owned.ttf']]);
for(const f of JSON.parse(readFileSync('fixtures/fonts/upstream.json')))if(f.name.endsWith('.ttf'))sources.set(f.sha256,`.codex-work/font-corpus/${f.family}/${f.name}`);
let referenceFragments=0,referenceGlyphs=0;
function native(json,bundle) {
 const q=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(q.length);head.writeUInt32LE(bundle.length,4);
 const r=spawnSync('target/release/mo-text-worker',['--lines'],{input:Buffer.concat([head,q,bundle]),maxBuffer:80*1024*1024,timeout:30000});
 assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}

function run(name,f,ends,expected='evaluated',validRequest=true) {
 const q={paragraph:f.request,lineEnds:ends},json=JSON.stringify(q),before=JSON.stringify(f.request);
 const n=native(json,f.bundle),w=wasm.shape_lines(json,f.bundle,component);assert.equal(w,n,name);assert.equal(JSON.stringify(f.request),before);
 const response=JSON.parse(n);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`,bundlePath=`${root}/bundles/${sha(f.bundle)}.bin`;
 writeFileSync(requestPath,json);writeFileSync(responsePath,n);writeFileSync(bundlePath,f.bundle);
 cases.push({name,kind:'line-shape',validRequest,requestPath,responsePath,bundlePath,requestSha256:sha(json),responseSha256:sha(n),bundleSha256:sha(f.bundle)});
 if(expected==='evaluated') {
  const r=response.result;assert.equal(r.lines.length,ends.length);assert.equal(r.fallback.items.length,r.shapedItemIndices.length);
  assert.deepEqual(r.bidi,JSON.parse(wasm.analyze_bidi(JSON.stringify({paragraphs:[{text:q.paragraph.text,direction:q.paragraph.direction,lineEnds:ends}],characters:[]}))).paragraphs[0]);
  let at=0,atItem=0,atFallback=0;
  for(const line of r.lines) {
   assert.equal(line.start.scalarOffset,at);at=line.end.scalarOffset;assert.equal(line.itemStart,atItem);assert.equal(line.fallbackStart,atFallback);atItem=line.itemEnd;atFallback=line.fallbackEnd;
   for(const index of [line.start,line.end]) {const prefix=[...q.paragraph.text].slice(0,index.scalarOffset).join('');assert.equal(index.utf8Offset,Buffer.byteLength(prefix));assert.equal(index.utf16Offset,prefix.length);}
  }
  assert.equal(at,[...q.paragraph.text].length);assert.equal(atItem,r.items.length);assert.equal(atFallback,r.fallback.items.length);
  const refs=verifyLineReference(q,r,sources);referenceFragments+=refs.referenceFragments;referenceGlyphs+=refs.referenceGlyphs;
 }
 return response;
}
const allGlyphs=r=>r.result.fallback.items.flatMap(i=>i.fragments).filter(f=>f.status==='selected').flatMap(f=>f.shaped.runs[0].glyphs);
const latin=fixture('office AV office');
const full=run('latin-one-line',latin,[16]),cut=run('latin-ligature-cut',latin,[2,4,10,16]);
assert.notDeepEqual(allGlyphs(full).map(g=>g.glyphId),allGlyphs(cut).map(g=>g.glyphId));
const arabic=fixture('بب',['notosansarabic']);
const joined=run('arabic-joined',arabic,[2]),separate=run('arabic-line-context',arabic,[1,2]);
assert.notDeepEqual(allGlyphs(joined).map(g=>g.glyphId),allGlyphs(separate).map(g=>g.glyphId));
assert.deepEqual(separate.result.fallback.items[0].fragments[0].shaped.runs[0].glyphs.map(g=>g.glyphId),separate.result.fallback.items[1].fragments[0].shaped.runs[0].glyphs.map(g=>g.glyphId));
// Actual counterexample: BOT/EOT flags alone do not erase the supplied context.
const retainedContextRequest={expectedSha256:arabic.request.fonts[0].expectedSha256,faceIndex:0,text:'بب',runs:[{start:0,end:1,direction:'rightToLeft',script:'Arab',language:'und',clusterLevel:'monotoneGraphemes',flags:{beginningOfText:true,endOfText:true,ignorables:'default',suppressDottedCircle:false,unsafeToConcat:true,safeToInsertTatweel:false},features:[],variations:[],maxGlyphs:100}]};
const retainedContext=JSON.parse(wasm.shape_text(JSON.stringify(retainedContextRequest),arabic.bundle,component));assert.equal(retainedContext.status,'shaped');
assert.notDeepEqual(retainedContext.text.runs[0].glyphs.map(g=>g.glyphId),separate.result.fallback.items[0].fragments[0].shaped.runs[0].glyphs.map(g=>g.glyphId));
for(const [name,text,names,ends] of [
 ['arabic-lamalef','لا',['notosansarabic'],[1,2]],
 ['arabic-digits-neutral','بب 12 بب',['notosansarabic','notosans'],[3,6,8]],
 ['bidi-isolate-across-lines','A\u2067ب 12\u2069A',['notosans','notosansarabic'],[4,8]],
 ['bracket-across-lines','A (ب 12) A',['notosans','notosansarabic'],[5,10]],
 ['devanagari-egc','कि कि',['notosansdevanagari'],[3,5]],
 ['combining-multibyte','A\u0301 α😀A',['owned'],[3,5,6]],
 ['cjk-lines','中文演示文稿',['notosanssc'],[2,4,6]],
 ['mixed-font-line-fragments','A👩A👩A',['owned','notoemoji'],[3,5]],
 ['emoji-zwj-line','A👩🏽‍💻A',['owned','notoemoji'],[1,5,6]],
 ['tabs-softbreak-controls','A\tA\u2028A',['notosans'],[2,4,5]],
 ['terminal-paragraph','A\r\n',['notosans'],[3]],
 ['missing-font-range','A👩A',['owned'],[1,2,3]],
 ['unsupported-variation','A\ufe02A',['owned'],[2,3]],
 ['empty','',['owned'],[0]],
 ['control-only','\t\u2028\u2029',['owned'],[2,3]],
])run(name,fixture(text,names),ends);
const l1=fixture('α اب  12',['notosans','notosansarabic']);l1.request.direction='leftToRight';
const reset=run('line-l1-reset',l1,[5,8]);assert.equal(reset.result.bidi.resolvedLevels[4],1);assert.equal(reset.result.bidi.lines[0].levels[4],0);
const features=fixture('office office office');features.request.styles[0].features=[{tag:'liga',value:0,start:8,end:12},{tag:'kern',value:0,start:0,end:null}];
run('global-feature-ranges',features,[7,14,20]);
const equivalent=fixture('officeoffice');equivalent.request.styles.push(structuredClone(equivalent.request.styles[0]));equivalent.request.styles[0].language='en-US';equivalent.request.styles[1].language='EN-us';equivalent.request.spans=[{end:3,style:0},{end:12,style:1}];
run('equivalent-styles-across-line',equivalent,[6,12]);
const varied=fixture('office office');varied.request.styles[0].candidates[0].variations=[{tag:'wght',value1616:700*65536+1}];run('variable-line-font',varied,[7,13]);
for(const [name,text,ends] of [['empty-plan','AA',[]],['partial-plan','AA',[1]],['repeated-end','AA',[1,1,2]],['outside','AA',[3]],['split-egc','A\u0301',[1,2]],['empty-invalid','',[1]]])run(name,fixture(text,['owned']),ends,'INPUT_INVALID');
const invalid=fixture('AA',['owned']);invalid.request.styles.push(structuredClone(invalid.request.styles[0]));invalid.request.styles[1].candidates[0].variations=[{tag:'xxxx',value1616:0}];run('unused-invalid-style',invalid,[1,2],'INPUT_INVALID');
const badFont=fixture('AA',['owned','notosans']);badFont.request.fonts[1].expectedSha256='0'.repeat(64);run('unused-invalid-resource',badFont,[1,2],'RESOURCE_CONFLICT');
run('line-count-budget',fixture('A',['owned']),Array(4097).fill(1),'LIMIT_EXCEEDED');
run('maximum-control-lines',fixture('\t'.repeat(4096),['owned']),Array.from({length:4096},(_,i)=>i+1));
run('shape-run-budget',fixture('A'.repeat(1025),['owned']),Array.from({length:1025},(_,i)=>i+1),'LIMIT_EXCEEDED');
const cliQ=JSON.stringify({paragraph:arabic.request,lineEnds:[1,2]});writeFileSync(root+'/cli.json',cliQ);writeFileSync(root+'/cli.bin',arabic.bundle);
const cli=spawnSync('target/release/mo-cli',['shape-lines',root+'/cli.json',root+'/cli.bin'],{encoding:'utf8',timeout:30000});assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trimEnd(),native(cliQ,arabic.bundle));
let raw,calls=0;const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default;
const faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm');
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
const injecting={shapeBatch(font,frame){if(++calls===2)raw._mo_hb_fail_after(0);return fault.shapeBatch(font,frame);},invalidate(){fault.invalidate();}};
const failure=JSON.parse(wasm.shape_lines(cliQ,arabic.bundle,injecting));assert.equal(failure.error.code,'COMPONENT_FAILURE');assert.equal(failure.result,undefined);assert.equal(calls,2);assert.equal(fault.invalid,true);
const reuse=JSON.parse(wasm.shape_lines(cliQ,arabic.bundle,injecting));assert.equal(reuse.error.code,'HOST_FAILURE');assert.equal(wasm.shape_lines(cliQ,arabic.bundle,component),native(cliQ,arabic.bundle));
const report={format:'musteroffice.line-shape-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),
 componentSha256:sha(readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),referenceSha256:sha(readFileSync('.codex-work/harfbuzz/release/hb-shape-reference')),referenceFragments,referenceGlyphs,cliVerified:true,
 retainedContextCounterexample:{request:retainedContextRequest,response:retainedContext},actualSecondLineFailure:{componentSha256:sha(faultBytes),failure,reuse,replacementVerified:true},cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,referenceFragments,referenceGlyphs}));
