import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js'),root='.codex-work/bidi';
const sha=b=>createHash('sha256').update(b).digest('hex'),cases=[];
function run(name,request,expected='analyzed',validRequest=true) {
  const json=typeof request==='string'?request:JSON.stringify(request),requestPath=root+'/'+name+'.request.json',responsePath=root+'/'+name+'.response.json';
  writeFileSync(requestPath,json);const n=spawnSync('target/release/mo-cli',['bidi-analyze',requestPath],{encoding:'utf8',maxBuffer:40*1024*1024,timeout:30000});
  assert.equal(n.status,0,n.stderr);const result=n.stdout.trimEnd(),w=wasm.analyze_bidi(json);assert.equal(w,result,name);const response=JSON.parse(result);
  assert.equal(response.status==='error'?response.code:response.status,expected,name);writeFileSync(responsePath,result);
  cases.push({name,validRequest,requestPath,responsePath,requestSha256:sha(json),responseSha256:sha(result)});return response;
}
const paragraph=(text,direction='leftToRight',lineEnds=[])=>({text,direction,lineEnds});
const query=p=>({paragraphs:p,characters:[]});
const properties=JSON.parse(readFileSync(root+'/property-fixtures.json'));
assert.deepEqual(run('property-probes',{paragraphs:[],characters:properties.characters}).characters,properties.expected);
const valid=run('paragraph-lines-and-coordinates',query([
  paragraph('אב  גד','leftToRight',[4,6]),paragraph('😀אA','rightToLeft',[1,3]),paragraph('A\u202ebc\u202cZ'),
  paragraph('א\r\n'),paragraph(''),paragraph('א \u2e62א A\u2e63'),paragraph('A\u2028א','leftToRight',[2,3]),
  paragraph('∝\u{1db10}','rightToLeft'),paragraph('\u2068א\u2069A','autoLeftToRight'),paragraph('123','autoLeftToRight'),
]));
assert.deepEqual(valid.paragraphs[0].resolvedLevels,[1,1,1,1,1,1]);assert.deepEqual(valid.paragraphs[0].lines[0].levels,[1,1,0,0]);
assert.deepEqual(valid.paragraphs[0].lines.map(l=>l.visualOrder),[[1,0,2,3],[5,4]]);
assert.deepEqual(valid.paragraphs[1].lines[1].start,{scalarOffset:1,utf8Offset:4,utf16Offset:2});
assert.deepEqual(valid.paragraphs[1].lines[1].end,{scalarOffset:3,utf8Offset:7,utf16Offset:4});
assert.deepEqual(valid.paragraphs[2].lines[0].visualOrder,[0,3,2,5]);
assert.deepEqual(valid.paragraphs[5].lines[0].levels,[1,0,0,1,0,0,0]);
assert.equal(valid.paragraphs[8].paragraphLevel,0);assert.equal(valid.paragraphs[9].paragraphLevel,0);
// Many explicit lines must not clone the whole paragraph per line.
const lines=16384,longText='א '.repeat(lines);
const long=run('many-lines',query([paragraph(longText,'leftToRight',Array.from({length:lines},(_,i)=>(i+1)*2))]));
assert.equal(long.paragraphs[0].lines.length,lines);
for(let i=0;i<lines;i++){assert.deepEqual(long.paragraphs[0].lines[i].levels,[1,0]);assert.deepEqual(long.paragraphs[0].lines[i].visualOrder,[2*i,2*i+1]);}
const nested='\u2067'.repeat(256)+'א'+'\u2069'.repeat(256);
const controls='\u202b'.repeat(256)+'א'+'\u202c'.repeat(256);
const deep=run('explicit-depth-overflow',query([paragraph(nested),paragraph(controls)]));
for(const p of deep.paragraphs)assert.ok(p.resolvedLevels.every(v=>v===null||v<=126));
const base=query([paragraph('A')]);
for(const [name,change,expected] of [
  ['paragraph-split',q=>q.paragraphs[0].text='A\nB','INPUT_INVALID'],
  ['line-splits-grapheme',q=>{q.paragraphs[0].text='a\u0301';q.paragraphs[0].lineEnds=[1,2];},'INPUT_INVALID'],
  ['line-splits-crlf',q=>{q.paragraphs[0].text='\r\n';q.paragraphs[0].lineEnds=[1,2];},'INPUT_INVALID'],
  ['line-zero',q=>q.paragraphs[0].lineEnds=[0,1],'INPUT_INVALID'],
  ['line-descending',q=>{q.paragraphs[0].text='AB';q.paragraphs[0].lineEnds=[2,1,2];},'INPUT_INVALID'],
  ['line-outside',q=>q.paragraphs[0].lineEnds=[4294967295],'INPUT_INVALID'],
  ['line-incomplete',q=>{q.paragraphs[0].text='AB';q.paragraphs[0].lineEnds=[1];},'INPUT_INVALID'],
  ['scalar-limit',q=>q.paragraphs[0].text='A'.repeat(65537),'LIMIT_EXCEEDED'],
  ['byte-limit',q=>q.paragraphs[0].text='😀'.repeat(65537),'LIMIT_EXCEEDED'],
  ['aggregate-scalars',q=>q.paragraphs=Array.from({length:5},()=>paragraph('A'.repeat(65536))),'LIMIT_EXCEEDED'],
  ['line-budget',q=>q.paragraphs[0].lineEnds=Array(65537).fill(0),'LIMIT_EXCEEDED'],
  ['paragraph-budget',q=>q.paragraphs=Array.from({length:1025},()=>paragraph('')),'LIMIT_EXCEEDED'],
  ['character-budget',q=>q.characters=Array(65537).fill(65),'LIMIT_EXCEEDED'],
  ['surrogate-query',q=>q.characters=[0xd800],'INPUT_INVALID'],
  ['outside-query',q=>q.characters=[0x110000],'INPUT_INVALID'],
]) {const q=structuredClone(base);change(q);run(name,q,expected);}
run('duplicate-key','{"paragraphs":[],"characters":[],"characters":[]}','INPUT_INVALID',false);
run('unknown-field',{...base,arbitraryDirection:'rtl'},'INPUT_INVALID',false);
run('bad-direction',query([{...paragraph('A'),direction:'autoRightToLeft'}]),'INPUT_INVALID',false);
const report={format:'musteroffice.bidi-parity/1',nativeSha256:sha(readFileSync('target/release/mo-cli')),wasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),propertyProbes:properties.characters.length,denseCodepointsCompared:properties.denseCodepointsCompared,explicitLinesVerified:lines,cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,propertyProbes:report.propertyProbes,lines}));
