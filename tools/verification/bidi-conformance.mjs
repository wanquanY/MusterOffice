// Run every official Unicode 18 type/direction and character case through both
// real APIs. No sampled cases or x9 placeholders counted as successful glyphs.
import assert from 'node:assert/strict';
import {createReadStream,readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createInterface} from 'node:readline';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/bidi';mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const manifest=JSON.parse(readFileSync('crates/mo-unicode/data/bidi-manifest.json'));
for(const input of manifest.inputs){const b=readFileSync(root+'/'+input.name);assert.equal(b.length,input.byteLength);assert.equal(sha(b),input.sha256);}
const representatives={L:'A',R:'א',AL:'ا',EN:'1',ES:'+',ET:'$',AN:'١',CS:',',NSM:'\u0300',BN:'\u2060',B:'\u2029',S:'\t',WS:' ',ON:'!',LRE:'\u202a',LRO:'\u202d',RLE:'\u202b',RLO:'\u202e',PDF:'\u202c',LRI:'\u2066',RLI:'\u2067',FSI:'\u2068',PDI:'\u2069'};
const properties=JSON.parse(wasm.analyze_bidi(JSON.stringify({paragraphs:[],characters:Object.values(representatives).map(c=>c.codePointAt(0))})));
assert.equal(properties.status,'analyzed');assert.deepEqual(properties.characters.map(c=>c.properties.class),Object.keys(representatives));
for(const c of properties.characters)assert.equal(c.properties.bracket,null);
let batch=[],batches=[],typeRows=0,typeCases=0,characterCases=0;
function flush(source) {
  if(!batch.length)return;
  const request=JSON.stringify({paragraphs:batch.map(c=>({text:c.text,direction:c.direction,lineEnds:[]})),characters:[]});
  writeFileSync(root+'/conformance-request.json',request);
  const native=spawnSync('target/release/mo-cli',['bidi-analyze',root+'/conformance-request.json'],{encoding:'utf8',maxBuffer:32*1024*1024,timeout:30000});
  assert.equal(native.status,0,native.stderr);const n=native.stdout.trimEnd(),w=wasm.analyze_bidi(request);assert.equal(w,n,'native/WASM batch '+batches.length);
  const response=JSON.parse(n);assert.equal(response.status,'analyzed',n);
  assert.equal(response.paragraphs.length,batch.length);
  for(let i=0;i<batch.length;i++) {
    const c=batch[i],r=response.paragraphs[i],label=`${source}:${c.line}:${c.direction}`;
    assert.equal(r.lines.length,1,label);
    assert.deepEqual(r.lines[0].levels,c.levels,label+' levels');
    assert.deepEqual(r.lines[0].visualOrder,c.order,label+' order');
    if(c.base!==null)assert.equal(r.paragraphLevel,c.base,label+' base');
    assert.equal(r.resolvedLevels.length,[...c.text].length,label+' scalar count');
  }
  batches.push({source,firstSourceLine:batch[0].line,lastSourceLine:batch.at(-1).line,cases:batch.length,requestSha256:sha(request),responseSha256:sha(n)});
  batch=[];if(batches.length%100===0)console.log(JSON.stringify({batches:batches.length,typeCases,characterCases}));
}
const parseLevels=s=>s.trim()?s.trim().split(/\s+/).map(x=>x==='x'?null:Number(x)):[];
const parseOrder=s=>s.trim()?s.trim().split(/\s+/).map(Number):[];
let levels=[],order=[],lineNumber=0;
for await(const raw of createInterface({input:createReadStream(root+'/BidiTest-18.0.0.txt'),crlfDelay:Infinity})) {
  lineNumber++;const line=raw.split('#')[0].trim();if(!line)continue;
  if(line.startsWith('@Levels:')){levels=parseLevels(line.slice(8));continue;}
  if(line.startsWith('@Reorder:')){order=parseOrder(line.slice(9));continue;}
  if(line.startsWith('@'))continue;
  const [tokens,maskText]=line.split(';');const classes=tokens.trim().split(/\s+/),mask=parseInt(maskText,16);
  assert.ok(classes.every(c=>c in representatives));assert.equal(classes.length,levels.length);assert.ok(mask>0&&mask<=7);typeRows++;
  for(const [bit,direction,base] of [[1,'autoLeftToRight',null],[2,'leftToRight',0],[4,'rightToLeft',1]])if(mask&bit) {
    batch.push({text:classes.map(c=>representatives[c]).join(''),direction,base,levels,order,line:lineNumber});typeCases++;
    if(batch.length===1024)flush('BidiTest');
  }
}
flush('BidiTest');lineNumber=0;
for await(const raw of createInterface({input:createReadStream(root+'/BidiCharacterTest-18.0.0.txt'),crlfDelay:Infinity})) {
  lineNumber++;const line=raw.split('#')[0].trim();if(!line)continue;
  const f=line.split(';');assert.equal(f.length,5);
  const scalars=f[0].trim().split(/\s+/).map(s=>parseInt(s,16));assert.ok(scalars.every(c=>c<=0x10ffff&&!(c>=0xd800&&c<=0xdfff)));
  const direction=['leftToRight','rightToLeft','autoLeftToRight'][Number(f[1])];assert.ok(direction);
  const levels=parseLevels(f[3]);assert.equal(levels.length,scalars.length);
  batch.push({text:String.fromCodePoint(...scalars),direction,base:Number(f[2]),levels,order:parseOrder(f[4]),line:lineNumber});characterCases++;
  if(batch.length===1024)flush('BidiCharacterTest');
}
flush('BidiCharacterTest');assert.equal(typeRows,490846);assert.equal(typeCases,770241);assert.equal(characterCases,91707);
const report={format:'musteroffice.bidi-conformance/1',unicodeVersion:'18.0.0',uax9Revision:52,typeRows,typeDirectionCases:typeCases,characterCases,casesSkipped:0,nativeWasmBatches:batches.length,nativeSha256:sha(readFileSync('target/release/mo-cli')),wasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),inputs:manifest.inputs.filter(v=>v.name.startsWith('BidiTest')||v.name.startsWith('BidiCharacterTest')),representatives,batches};
writeFileSync(root+'/conformance.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({complete:true,typeRows,typeCases,characterCases,batches:batches.length,skipped:0}));
