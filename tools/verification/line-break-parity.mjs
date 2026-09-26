// Complete official corpus plus real API/property/resource checks on both ends.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/line-break/verification';mkdirSync(root,{recursive:true});
const sha=b=>createHash('sha256').update(b).digest('hex');
const manifest=JSON.parse(readFileSync('crates/mo-unicode/data/line-break-manifest.json'));
for(const row of manifest.inputs){const b=readFileSync('.codex-work/line-break/'+row.name);assert.equal(b.length,row.byteLength);assert.equal(sha(b),row.sha256);}
const cases=[];
function check(name,input,expected='analyzed',validRequest=true,extra={}) {
  const json=typeof input==='string'?input:JSON.stringify(input),requestPath=root+'/'+name+'.request.json',responsePath=root+'/'+name+'.response.json';
  writeFileSync(requestPath,json);
  const result=spawnSync('target/release/mo-cli',['line-break-analyze',requestPath],{encoding:'utf8',maxBuffer:80*1024*1024,timeout:30000});
  assert.equal(result.status,0,result.stderr);const n=result.stdout.trimEnd(),w=wasm.analyze_line_breaks(json);assert.equal(w,n,name);
  const r=JSON.parse(n);assert.equal(r.status==='error'?r.code:r.status,expected,name);
  if(r.status==='error')assert.equal(r.texts,undefined);
  else {
    assert.equal(r.unicodeVersion,'18.0.0');assert.equal(r.texts.length,input.texts.length);
    for(let i=0;i<r.texts.length;i++) {
      const source=input.texts[i],scalars=[...source],text=r.texts[i];let last=0;
      assert.equal(text.profile,'unicode18.0.0-uax14-r57-default-v1');
      assert.deepEqual(text.end,{scalarOffset:scalars.length,utf8Offset:Buffer.byteLength(source),utf16Offset:source.length});
      const offsets=[{scalarOffset:0,utf8Offset:0,utf16Offset:0}];
      for(let j=0;j<scalars.length;j++)offsets.push({scalarOffset:j+1,utf8Offset:offsets[j].utf8Offset+Buffer.byteLength(scalars[j]),utf16Offset:offsets[j].utf16Offset+scalars[j].length});
      for(const {boundary,kind} of text.opportunities) {
        assert.ok(boundary.scalarOffset>last&&boundary.scalarOffset<=scalars.length);last=boundary.scalarOffset;
        assert.deepEqual(boundary,offsets[last]);
        // The official file marks both optional and mandatory breaks with ÷.
        // Mandatory characters are independently enumerated from UAX14 BK/NLF.
        const hard=scalars[last-1]?.codePointAt(0);
        assert.equal(kind,last===scalars.length||[10,11,12,13,0x85,0x2028,0x2029].includes(hard)?'mandatory':'allowed');
      }
      assert.equal(last,scalars.length);
    }
  }
  writeFileSync(responsePath,n);
  cases.push({name,validRequest,requestPath,responsePath,requestSha256:sha(json),responseSha256:sha(n),...extra});return r;
}
let official=[],officialCases=0,officialBatches=0;
function flush() {
  if(!official.length)return;
  const r=check('official-'+String(officialBatches).padStart(3,'0'),{texts:official.map(c=>c.text),characters:[]},'analyzed',true,
    {officialCases:official.length,firstSourceLine:official[0].line,lastSourceLine:official.at(-1).line});
  for(let i=0;i<official.length;i++)assert.deepEqual(r.texts[i].opportunities.map(p=>p.boundary.scalarOffset),official[i].expected,'official source line '+official[i].line);
  officialBatches++;official=[];
}
const lines=readFileSync('.codex-work/line-break/LineBreakTest-18.0.0.txt','utf8').split(/\r?\n/);
for(let index=0;index<lines.length;index++) {
  const line=lines[index].split('#')[0].trim();if(!line)continue;
  const chars=[],expected=[];
  for(const part of line.split(/\s+/)) {
    if(part==='÷')expected.push(chars.length);
    else if(part!=='×') {const cp=parseInt(part,16);assert.ok(cp<=0x10ffff&&!(cp>=0xd800&&cp<=0xdfff));chars.push(String.fromCodePoint(cp));}
  }
  official.push({text:chars.join(''),expected,line:index+1});officialCases++;
  if(official.length===1024)flush();
}
flush();assert.equal(officialCases,19346);assert.equal(officialBatches,19);
const probes=JSON.parse(readFileSync('.codex-work/line-break/property-probes.json'));
for(let start=0;start<probes.length;start+=4096) {
  const chunk=probes.slice(start,start+4096),r=check('properties-'+start,{texts:[],characters:chunk.map(c=>c.codepoint)});
  assert.deepEqual(r.characters,chunk);
}
const basic=check('basic-controls-dictionary',{texts:['','a\r\nb\u0085c\u2028d',' \u0308A','ไทย','中（文）文','😀 中'],characters:[]});
assert.deepEqual(basic.texts[0].opportunities,[]);assert.deepEqual(basic.texts[2].opportunities.map(p=>p.boundary.scalarOffset),[1,3]);
assert.deepEqual(basic.texts[3].complexContextScalars,[0,1,2]);
const spaces=check('long-opening-spaces',{texts:['('+' '.repeat(65534)+'x'],characters:[]});assert.equal(spaces.texts[0].opportunities.length,1);
const marks=check('long-combining-numeric-context',{texts:['1'+'\u0308'.repeat(32000)+','.repeat(32000)+'%'],characters:[]});assert.equal(marks.texts[0].opportunities.length,1);
const ris=check('long-regional-indicators',{texts:['🇨\u0308'.repeat(16000)],characters:[]});assert.equal(ris.texts[0].opportunities.length,8000);
const maximum=check('maximum-scalar-text',{texts:['中'.repeat(65536)],characters:[]});assert.equal(maximum.texts[0].opportunities.length,65536);
check('maximal-property-batch',{texts:[],characters:Array(65536).fill(0x10ffff)});
for(const [name,input] of [
  ['scalar-budget',{texts:['a'.repeat(65537)],characters:[]}],
  ['byte-budget',{texts:['😀'.repeat(65537)],characters:[]}],
  ['aggregate-budget',{texts:Array(5).fill('a'.repeat(65536)),characters:[]}],
  ['text-count-budget',{texts:Array(1025).fill(''),characters:[]}],
  ['property-count-budget',{texts:[],characters:Array(65537).fill(65)}],
])check(name,input,'LIMIT_EXCEEDED');
for(const cp of [0xd800,0xdfff,0x110000])check('invalid-scalar-'+cp,{texts:['valid first'],characters:[65,cp]},'INPUT_INVALID');
for(const [name,json] of [
  ['duplicate-key','{"texts":[],"texts":[],"characters":[]}'],
  ['unpaired-surrogate','{"texts":["\\ud800"],"characters":[]}'],
  ['implicit-system-option','{"texts":[],"characters":[],"language":"system"}'],
  ['negative-codepoint','{"texts":[],"characters":[-1]}'],
  ['fractional-codepoint','{"texts":[],"characters":[0.5]}'],
  ['missing-texts','{"characters":[]}'],
])check(name,json,'INPUT_INVALID',false);
const report={format:'musteroffice.line-break-verification/1',unicodeVersion:'18.0.0',uax14Revision:57,officialCases,officialBatches,casesSkipped:0,propertyProbes:probes.length,
  nativeSha256:sha(readFileSync('target/release/mo-cli')),wasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases};
writeFileSync('.codex-work/line-break/parity.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,officialCases,officialBatches,propertyProbes:probes.length,skipped:0}));
