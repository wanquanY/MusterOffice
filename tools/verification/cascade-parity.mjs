import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/font-cascade';mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const componentBytes=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm');
const compiled=new WebAssembly.Module(componentBytes),component=await ShapingComponent.create(factory,compiled);
const sources={},paths=new Map();
for(const row of JSON.parse(readFileSync('fixtures/fonts/upstream.json'))) {
  if(!row.name.endsWith('.ttf'))continue;
  const bytes=readFileSync(`.codex-work/font-corpus/${row.family}/${row.name}`);
  assert.equal(sha(bytes),row.sha256);sources[row.family]=bytes;paths.set(row.sha256,`.codex-work/font-corpus/${row.family}/${row.name}`);
}
sources.owned=readFileSync('fixtures/fonts/owned.ttf');sources.collection=readFileSync('fixtures/fonts/owned.ttc');
paths.set(sha(sources.owned),'fixtures/fonts/owned.ttf');paths.set(sha(sources.collection),'fixtures/fonts/owned.ttc');
function build(text,names,properties={}) {
  const parts=names.map(n=>sources[n]);let offset=0;
  const fonts=parts.map(bytes=>{const f={expectedSha256:sha(bytes),faceIndex:0,offset:String(offset),byteLength:String(bytes.length)};offset+=bytes.length;return f;});
  return {bundle:Buffer.concat(parts),request:{text,fonts,items:[{start:0,end:[...text].length,direction:'leftToRight',script:'Latn',language:'en',features:[],beginningOfText:true,endOfText:true,suppressDottedCircle:false,maxGlyphs:262144,candidates:fonts.map((_,font)=>({font,variations:[]})),...properties}]}};
}
function native(json,bundle) {
  const req=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(req.length);head.writeUInt32LE(bundle.length,4);
  const r=spawnSync('target/release/mo-text-worker',['--cascade'],{input:Buffer.concat([head,req,bundle]),maxBuffer:80*1024*1024,timeout:30000});
  assert.equal(r.status,0,r.stderr?.toString());assert.equal(r.stdout.length,r.stdout.readUInt32LE(0)+4);return r.stdout.subarray(4).toString();
}
let directGlyphs=0,directItems=0;
const cases=[];
function verifySelected(request,bundle,response) {
  if(response.status!=='evaluated')return;
  for(let i=0;i<response.result.items.length;i++) {
    const selection=response.result.items[i],item=request.items[i];
    if(selection.status!=='selected')continue;
    assert.equal(selection.attempts.length,selection.candidate+1);
    assert.equal(selection.attempts.at(-1).missingGlyphClusters.length,0);
    assert.equal(selection.attempts.at(-1).variationIssues.length,0);
    for(const attempt of selection.attempts.slice(0,-1))assert.ok(attempt.missingGlyphClusters.length||attempt.variationIssues.length);
    const f=request.fonts[selection.font],candidate=item.candidates[selection.candidate];
    const font=bundle.subarray(Number(f.offset),Number(f.offset)+Number(f.byteLength));
    const q={text:request.text,expectedSha256:f.expectedSha256,faceIndex:f.faceIndex,runs:[{
      start:item.start,end:item.end,direction:item.direction,script:item.script,language:item.language,features:item.features,
      variations:candidate.variations,maxGlyphs:item.maxGlyphs,clusterLevel:'monotoneGraphemes',
      flags:{beginningOfText:item.beginningOfText,endOfText:item.endOfText,ignorables:'default',suppressDottedCircle:item.suppressDottedCircle,unsafeToConcat:true,safeToInsertTatweel:false}
    }]};
    const direct=JSON.parse(wasm.shape_text(JSON.stringify(q),font,component));assert.equal(direct.status,'shaped');
    assert.deepEqual(selection.shaped,direct.text);directItems++;directGlyphs+=direct.text.runs[0].glyphs.length;
    const characters=[...request.text];
    const args=[paths.get(f.expectedSha256),'--text='+characters.slice(item.start,item.end).join(''),
      '--text-before='+characters.slice(0,item.start).join(''),'--text-after='+characters.slice(item.end).join(''),
      '--direction='+({leftToRight:'ltr',rightToLeft:'rtl',topToBottom:'ttb',bottomToTop:'btt'}[item.direction]),
      '--script='+item.script,'--language='+item.language,'--face-index='+f.faceIndex,'--font-size='+selection.shaped.positionUnitsPerEm,
      '--font-funcs=ot','--shapers=ot','--cluster-level=0','--unsafe-to-concat','--no-glyph-names','--show-flags','-O','json'];
    if(item.beginningOfText)args.push('--bot');if(item.endOfText)args.push('--eot');
    if(candidate.variations.length)args.push('--variations='+candidate.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`).join(','));
    assert.equal(item.features.length,0,'reference feature range adaptation needed if fixtures add features');
    assert.equal(item.suppressDottedCircle,false,'upstream CLI has no suppression option');
    const ref=spawnSync('.codex-work/harfbuzz/release/hb-shape-reference',args,{encoding:'utf8',maxBuffer:20*1024*1024});assert.equal(ref.status,0,ref.stderr);
    const expected=JSON.parse(ref.stdout).map(g=>({...g,fl:g.fl??0,cl:g.cl+item.start}));
    const actual=selection.shaped.runs[0].glyphs.map(g=>({g:g.glyphId,cl:g.cluster,fl:(g.unsafeToBreak?1:0)|(g.unsafeToConcat?2:0)|(g.safeToInsertTatweel?4:0),ax:g.xAdvance,ay:g.yAdvance,dx:g.xOffset,dy:g.yOffset}));
    assert.deepEqual(actual,expected,'selected output differs from independent upstream CLI');
  }
}
function run(name,fixture,expected='evaluated',validRequest=true) {
  const json=typeof fixture.request==='string'?fixture.request:JSON.stringify(fixture.request);
  const n=native(json,fixture.bundle),w=wasm.shape_cascade(json,fixture.bundle,component);assert.equal(w,n,name);
  const response=JSON.parse(n);assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
  if(typeof fixture.request!=='string')verifySelected(fixture.request,fixture.bundle,response);
  cases.push({name,request:json,validRequest,bundleSha256:sha(fixture.bundle),bundleByteLength:fixture.bundle.length,response,responseSha256:sha(n)});return response;
}
for(const [name,text,fonts,props,selected] of [
  ['latin-ligatures','office affinity',['owned','notosans'],{},1],
  ['arabic','العربية',['notosans','notosansarabic'],{script:'Arab',language:'ar',direction:'rightToLeft'},1],
  ['devanagari','नमस्ते',['owned','notosansdevanagari'],{script:'Deva',language:'hi'},1],
  ['overlapping-devanagari-coverage','नमस्ते',['notosans','notosansdevanagari'],{script:'Deva',language:'hi'},0],
  ['cjk','中文排版',['notosans','notosanssc'],{script:'Hani',language:'zh'},1],
  ['vertical-cjk','中文排版',['notosans','notosanssc'],{script:'Hani',language:'zh',direction:'topToBottom'},1],
  ['emoji-zwj','👩🏽‍💻',['notosans','notoemoji'],{script:'Zyyy',language:'und'},1],
  ['combining','A\u0301',['owned','notosans'],{},0],
  ['canonical-decomposition-without-nominal-glyph','Á',['owned','notosans'],{},0],
  ['first-candidate-wins','office',['notosans','notosansarabic'],{},0],
  ['default-uvs','A\ufe00',['owned','notosans'],{},0],
  ['nondefault-uvs','A\ufe01',['owned','notosans'],{},0],
]) {const r=run(name,build(text,fonts,props));assert.equal(r.result.items[0].status,'selected');assert.equal(r.result.items[0].candidate,selected,name);}
for(const [name,text] of [['missing-every-font','\u{10ffff}'],['unsupported-uvs','A\ufe02'],['orphan-selector','\ufe00']]) {
  const r=run(name,build(text,['owned','notosans']));assert.equal(r.result.items[0].status,'unresolved');assert.equal(r.result.items[0].attempts.length,2);
}
// All items retain the same full logical context. Direction is per item; no bidi inference.
const mixed=build('office العربية नमस्ते 中文 😀',['notosans','notosansarabic','notosansdevanagari','notosanssc','notoemoji']);
let start=0;
mixed.request.items=[['office ', 'Latn','en','leftToRight'],['العربية ','Arab','ar','rightToLeft'],['नमस्ते ','Deva','hi','leftToRight'],['中文 ','Hani','zh','leftToRight'],['😀','Zyyy','und','leftToRight']].map(([text,script,language,direction])=>{
  const end=start+[...text].length,item={...structuredClone(mixed.request.items[0]),start,end,script,language,direction,beginningOfText:start===0,endOfText:end===[...mixed.request.text].length};start=end;return item;
});
const mix=run('five-script-context',mixed);assert.deepEqual(mix.result.items.map(i=>i.candidate),[0,1,0,3,4]);assert.equal(mix.result.verifiedFaces,5);
const repeated=build('A A A',['owned']);repeated.request.fonts.push(structuredClone(repeated.request.fonts[0]));
repeated.request.items=[0,2,4].map(start=>({...structuredClone(repeated.request.items[0]),start,end:start+1,candidates:[{font:start===2?1:0,variations:[]}]}));
assert.equal(run('same-face-binding-reuse',repeated).result.verifiedFaces,1);
const collection=build('A',['collection']);collection.request.fonts.push({...collection.request.fonts[0],faceIndex:1});
collection.request.items[0].candidates=[{font:1,variations:[]}];assert.equal(run('explicit-collection-face',collection).result.verifiedFaces,2);
const axis=build('office',['notosans']);axis.request.items[0].candidates[0].variations=[{tag:'wght',value1616:(700<<16)+1}];run('candidate-axis-quantization',axis);
const context=build('بالعربيةب',['notosans','notosansarabic'],{script:'Arab',language:'ar',direction:'rightToLeft',start:1,end:8,beginningOfText:false,endOfText:false});run('arabic-context-outside-item',context);
const empty=build('',['owned']);empty.request.items=[];assert.equal(run('empty-items',empty).result.shapingCalls,0);
const base=build('A',['owned']);
for(const [name,change,expected] of [
  ['unknown-binding',r=>r.items[0].candidates[0].font=2,'INPUT_INVALID'],
  ['no-candidates',r=>r.items[0].candidates=[],'INPUT_INVALID'],
  ['empty-item',r=>r.items[0].end=0,'INPUT_INVALID'],
  ['out-of-range',r=>r.items[0].end=2,'INPUT_INVALID'],
  ['overlap',r=>r.items.push(structuredClone(r.items[0])),'INPUT_INVALID'],
  ['split-grapheme',r=>r.text='A\u0301','INPUT_INVALID'],
  ['split-emoji',r=>{r.text='👩🏽‍💻';r.items[0].end=2;},'INPUT_INVALID'],
  ['unused-invalid-axis',r=>r.items[0].candidates.push({font:0,variations:[{tag:'xxxx',value1616:0}]}),'INPUT_INVALID'],
  ['unused-invalid-font',r=>r.fonts.push({...r.fonts[0],expectedSha256:'0'.repeat(64)}),'RESOURCE_CONFLICT'],
  ['wrong-digest',r=>r.fonts[0].expectedSha256='0'.repeat(64),'RESOURCE_CONFLICT'],
  ['bundle-range',r=>r.fonts[0].offset='18446744073709551615','INPUT_INVALID'],
  ['bundle-length',r=>r.fonts[0].byteLength='18446744073709551615','INPUT_INVALID'],
  ['font-budget',r=>r.fonts=Array.from({length:33},()=>structuredClone(r.fonts[0])),'LIMIT_EXCEEDED'],
  ['candidate-budget',r=>r.items[0].candidates=Array.from({length:33},()=>({font:0,variations:[]})),'LIMIT_EXCEEDED'],
  ['item-budget',r=>r.items=Array.from({length:257},()=>structuredClone(r.items[0])),'LIMIT_EXCEEDED'],
  ['text-budget',r=>r.text='A'.repeat(65537),'LIMIT_EXCEEDED'],
  ['glyph-budget',r=>r.items[0].maxGlyphs=0,'LIMIT_EXCEEDED'],
  ['bad-language',r=>r.items[0].language='en/US','INPUT_INVALID'],
  ['bad-script',r=>r.items[0].script='Ab12','INPUT_INVALID'],
]) {const f={bundle:base.bundle,request:structuredClone(base.request)};change(f.request);run(name,f,expected);}
run('duplicate-json-key',{bundle:base.bundle,request:JSON.stringify(base.request).replace('"text":"A"','"text":"A","text":"A"')},'INPUT_INVALID',false);
run('unknown-field',{bundle:base.bundle,request:JSON.stringify({...base.request,path:'/not/a/resource'})},'INPUT_INVALID',false);
// Confirm no earlier selected item escapes when a later shaping call fails.
const partial=structuredClone(repeated.request);partial.items[1].maxGlyphs=0;
assert.ok(!('result' in run('atomic-later-item-failure',{bundle:repeated.bundle,request:partial},'LIMIT_EXCEEDED')));
// Corrupt/failing components never cause another font attempt.
let calls=0,invalidations=0;
const failing={shapeBatch(){calls++;return Uint32Array.of(2,0);},invalidate(){invalidations++;}};
const failed=JSON.parse(wasm.shape_cascade(JSON.stringify(build('A',['owned','notosans']).request),build('A',['owned','notosans']).bundle,failing));
assert.equal(failed.error.code,'COMPONENT_FAILURE');assert.equal(calls,1);assert.equal(invalidations,1);
// Real allocation failure on the second font, after a valid missing-glyph result.
const faultFactory=(await import('../../.codex-work/harfbuzz/mo-hb.mjs')).default;
const faultBytes=readFileSync('.codex-work/harfbuzz/mo-hb.wasm');let raw;
const fault=await ShapingComponent.create(async options=>{raw=await faultFactory(options);return raw;},new WebAssembly.Module(faultBytes));
let faultCalls=0;
const injecting={shapeBatch(font,frame){if(++faultCalls===2)raw._mo_hb_fail_after(0);return fault.shapeBatch(font,frame);},invalidate(){fault.invalidate();}};
const faultInput=build('office',['owned','notosans']);
const faultResponse=JSON.parse(wasm.shape_cascade(JSON.stringify(faultInput.request),faultInput.bundle,injecting));
assert.equal(faultResponse.error.code,'COMPONENT_FAILURE');assert.equal(faultCalls,2);assert.equal(fault.invalid,true);assert.ok(!('result' in faultResponse));
const faultReuse=JSON.parse(wasm.shape_cascade(JSON.stringify(faultInput.request),faultInput.bundle,injecting));
assert.equal(faultReuse.error.code,'HOST_FAILURE');
assert.equal(wasm.shape_cascade(JSON.stringify(faultInput.request),faultInput.bundle,component),native(JSON.stringify(faultInput.request),faultInput.bundle));
// Actual CLI isolation path, not only direct worker framing.
writeFileSync(root+'/request.json',JSON.stringify(mixed.request));writeFileSync(root+'/fonts.bin',mixed.bundle);
const cli=spawnSync('target/release/mo-cli',['shape-cascade',root+'/request.json',root+'/fonts.bin'],{encoding:'utf8',maxBuffer:80*1024*1024});
assert.equal(cli.status,0,cli.stderr);assert.equal(cli.stdout.trim(),native(JSON.stringify(mixed.request),mixed.bundle));
component.invalidate();
const report={format:'musteroffice.font-cascade-parity/1',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),referenceSha256:sha(readFileSync('.codex-work/harfbuzz/release/hb-shape-reference')),directShapeItemsCompared:directItems,directShapeGlyphsCompared:directGlyphs,upstreamReferenceItemsCompared:directItems,upstreamReferenceGlyphsCompared:directGlyphs,cliVerified:true,fatalComponentErrorDoesNotFallback:true,actualSecondFontAllocationFailure:{componentSha256:sha(faultBytes),response:faultResponse,reuseResponse:faultReuse,invalidated:true,replacementVerified:true},cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({cases:cases.length,directItems,directGlyphs,cliVerified:true}));
