import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
export async function createHarness(root) {
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
mkdirSync(root+'/bundles',{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const componentBytes=readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm');
const component=await ShapingComponent.create(factory,new WebAssembly.Module(componentBytes));
const sources={},paths=new Map();
for(const f of JSON.parse(readFileSync('fixtures/fonts/upstream.json')))if(f.name.endsWith('.ttf')) {
 const p=`.codex-work/font-corpus/${f.family}/${f.name}`,bytes=readFileSync(p);assert.equal(sha(bytes),f.sha256);sources[f.family]=bytes;paths.set(f.sha256,p);
}
sources.owned=readFileSync('fixtures/fonts/owned.ttf');paths.set(sha(sources.owned),'fixtures/fonts/owned.ttf');
const count=s=>[...s].length;
const simple=text=>({text,direction:'autoLeftToRight',spans:text?[{end:count(text),style:0}]:[]});
const cases=[];let referenceItems=0,referenceGlyphs=0;
function save(name,request,response,kind,validRequest=true,extra={}) {
 const requestPath=`${root}/${name}.request.json`,responsePath=`${root}/${name}.response.json`;
 writeFileSync(requestPath,request);writeFileSync(responsePath,response);
 cases.push({name,kind,validRequest,requestPath,responsePath,requestSha256:sha(request),responseSha256:sha(response),...extra});
}
function itemize(name,request,expected='itemized',validRequest=true) {
 const json=typeof request==='string'?request:JSON.stringify(request);writeFileSync(root+'/input.json',json);
 const n=spawnSync('target/release/mo-cli',['itemize-paragraph',root+'/input.json'],{encoding:'utf8',maxBuffer:40*1024*1024,timeout:30000});assert.equal(n.status,0,n.stderr);
 const native=n.stdout.trimEnd(),w=wasm.itemize_paragraph(json);assert.equal(w,native,name);const r=JSON.parse(w);
 assert.equal(r.status==='error'?r.error.code:r.status,expected,name);save(name,json,native,'itemization',validRequest);
 if(r.status==='itemized') {
  let end=0;const chars=[...request.text];
  for(const item of r.result.items) {
   assert.equal(item.start.scalarOffset,end);end=item.end.scalarOffset;assert.ok(end>item.start.scalarOffset);
   for(const b of [item.start,item.end]) {const prefix=chars.slice(0,b.scalarOffset).join('');assert.equal(b.utf8Offset,Buffer.byteLength(prefix));assert.equal(b.utf16Offset,prefix.length);}
  }
  assert.equal(end,chars.length);
 }
 return r;
}
function fixture(text,names=['notosans']) {
 let offset=0;const parts=names.map(n=>sources[n]);const fonts=parts.map(bytes=>{const f={expectedSha256:sha(bytes),faceIndex:0,offset:String(offset),byteLength:String(bytes.length)};offset+=bytes.length;return f;});
 const style={language:'und',features:[],candidates:fonts.map((_,font)=>({font,variations:[]})),suppressDottedCircle:false,maxGlyphs:262144};
 return {bundle:Buffer.concat(parts),request:{...simple(text),styles:[style],fonts}};
}
function nativeParagraph(json,bundle) {
 const request=Buffer.from(json),head=Buffer.alloc(8);head.writeUInt32LE(request.length);head.writeUInt32LE(bundle.length,4);
 const result=spawnSync('target/release/mo-text-worker',['--paragraph'],{input:Buffer.concat([head,request,bundle]),maxBuffer:80*1024*1024,timeout:30000});assert.equal(result.status,0,result.stderr?.toString());assert.equal(result.stdout.length,result.stdout.readUInt32LE(0)+4);return result.stdout.subarray(4).toString();
}
function reference(request,result) {
 for(let i=0;i<result.fallback.items.length;i++) {
  const item=result.itemization.items[result.shapedItemIndices[i]];
  for(const selection of result.fallback.items[i].fragments) {
  assert.equal(item.kind,'text');if(selection.status!=='selected')continue;
  const style=request.styles[item.style],candidate=style.candidates[selection.candidate],font=request.fonts[candidate.font],chars=[...request.text];
  const start=selection.start,end=selection.end,shaped=selection.shaped;
  const args=[paths.get(font.expectedSha256),'--text='+chars.slice(start,end).join(''),'--text-before='+chars.slice(0,start).join(''),'--text-after='+chars.slice(end).join(''),
   '--direction='+(item.level%2?'rtl':'ltr'),'--script='+item.script,'--language='+style.language,'--face-index='+font.faceIndex,'--font-size='+shaped.positionUnitsPerEm,
   '--font-funcs=ot','--shapers=ot','--cluster-level=0','--unsafe-to-concat','--no-glyph-names','--show-flags','-O','json'];
  if(start===0)args.push('--bot');if(end===chars.length)args.push('--eot');
  if(candidate.variations.length)args.push('--variations='+candidate.variations.map(v=>`${v.tag}=${Math.fround(v.value1616/65536)}`).join(','));
  if(style.features.length) {assert.ok(style.features.every(f=>f.start===0&&f.end===null));args.push('--features='+style.features.map(f=>f.tag+'='+f.value).join(','));}
  assert.equal(style.suppressDottedCircle,false);
  const external=spawnSync('.codex-work/harfbuzz/release/hb-shape-reference',args,{encoding:'utf8',maxBuffer:20*1024*1024,timeout:30000});assert.equal(external.status,0,external.stderr);
  const expected=JSON.parse(external.stdout).map(g=>({...g,fl:g.fl??0,cl:g.cl+start}));
  const actual=shaped.runs[0].glyphs.map(g=>({g:g.glyphId,cl:g.cluster,fl:(g.unsafeToBreak?1:0)|(g.unsafeToConcat?2:0)|(g.safeToInsertTatweel?4:0),ax:g.xAdvance,ay:g.yAdvance,dx:g.xOffset,dy:g.yOffset}));
  assert.deepEqual(actual,expected,'automatic item differs from upstream shaper');referenceItems++;referenceGlyphs+=actual.length;
  }
 }
}
function paragraph(name,f,expected='evaluated',validRequest=true) {
 const json=typeof f.request==='string'?f.request:JSON.stringify(f.request);const n=nativeParagraph(json,f.bundle),w=wasm.shape_paragraph(json,f.bundle,component);assert.equal(w,n,name);
 const r=JSON.parse(n);assert.equal(r.status==='error'?r.error.code:r.status,expected,name);
 const bundlePath=root+'/bundles/'+sha(f.bundle)+'.bin';if(!existsSync(bundlePath))writeFileSync(bundlePath,f.bundle);
 save(name,json,n,'paragraph-shape',validRequest,{bundlePath,bundleSha256:sha(f.bundle)});
 if(r.status==='evaluated') {
  assert.equal(r.result.shapedItemIndices.length,r.result.fallback.items.length);
  assert.deepEqual(r.result.itemization.items.filter(i=>i.kind==='text'),r.result.shapedItemIndices.map(i=>r.result.itemization.items[i]));
  for(let i=0;i<r.result.fallback.items.length;i++) {
   const part=r.result.fallback.items[i],logical=r.result.itemization.items[r.result.shapedItemIndices[i]];let end=part.start;
   assert.equal(part.start,logical.start.scalarOffset);assert.equal(part.end,logical.end.scalarOffset);
   for(const fragment of part.fragments) {
    assert.equal(fragment.start,end);assert.ok(fragment.end>fragment.start);assert.ok(!part.protectedBoundaries.includes(fragment.start));end=fragment.end;
    if(fragment.status==='selected')assert.ok(fragment.shaped.runs[0].glyphs.every(g=>g.glyphId!==0));
   }
   assert.equal(end,part.end);
  }
  reference(f.request,r.result);
 }
 return r;
}

function finish(extra={}) {
 const report={format:'musteroffice.paragraph-verification/2',nativeCliSha256:sha(readFileSync('target/release/mo-cli')),nativeWorkerSha256:sha(readFileSync('target/release/mo-text-worker')),rustWasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),componentSha256:sha(componentBytes),referenceSha256:sha(readFileSync('.codex-work/harfbuzz/release/hb-shape-reference')),upstreamReferenceItems:referenceItems,upstreamReferenceGlyphs:referenceGlyphs,...extra,cases};
 writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify({cases:cases.length,itemization:cases.filter(c=>c.kind==='itemization').length,paragraph:cases.filter(c=>c.kind==='paragraph-shape').length,referenceItems,referenceGlyphs,...extra}));return report;
}
return {root,fixture,paragraph,itemize,finish,cases,nativeParagraph,component,wasm,sources,sha};
}
