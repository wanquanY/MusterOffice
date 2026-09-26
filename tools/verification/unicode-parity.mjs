import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const root='.codex-work/unicode/verification',fixtures=JSON.parse(readFileSync(root+'/fixtures.json'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const cases=[];
function query(name,input,expected='analyzed',validRequest=true) {
  const request=typeof input==='string'?input:JSON.stringify(input),requestPath=root+'/'+name+'.request.json';writeFileSync(requestPath,request);
  const r=spawnSync('target/release/mo-cli',['text-analyze',requestPath],{encoding:'utf8',maxBuffer:64*1024*1024});assert.equal(r.status,0,r.stderr);
  const native=r.stdout.trim(),web=wasm.analyze_text(request);assert.equal(native,web,name);const response=JSON.parse(web);
  assert.equal(response.status==='analyzed'?response.status:response.code,expected,name);
  const responsePath=root+'/'+name+'.response.json';writeFileSync(responsePath,web);
  cases.push({name,requestPath,responsePath,requestSha256:sha(request),responseSha256:sha(web),validRequest,status:response.status});return response;
}
for(let i=0;i<fixtures.officialGraphemeCases.length;i+=128) {
  const samples=fixtures.officialGraphemeCases.slice(i,i+128),response=query('uax29-'+i,{texts:samples.map(s=>s.text),characters:[]});
  assert.equal(response.unicodeVersion,'18.0.0');
  samples.forEach((sample,index)=>assert.deepEqual(response.texts[index].boundaries,sample.boundaries,'official line '+sample.line));
}
const properties=query('property-probes',{texts:[],characters:fixtures.propertyProbes.map(p=>p.codepoint)});
assert.deepEqual(properties.characters,fixtures.propertyProbes);
const extra=query('coordinate-and-empty',{texts:['','A😀','\r\n','क्','\u094dक','A\uFE00\u{E0100}\u200d'],characters:[0x180b,0xfe0f,0xe0100,0x200d]});
assert.deepEqual(extra.texts[0].boundaries,[{scalarOffset:0,utf8Offset:0,utf16Offset:0}]);
assert.deepEqual(extra.texts[1].boundaries,[{scalarOffset:0,utf8Offset:0,utf16Offset:0},{scalarOffset:1,utf8Offset:1,utf16Offset:1},{scalarOffset:2,utf8Offset:5,utf16Offset:3}]);
assert.deepEqual(extra.texts[4].boundaries.map(b=>b.scalarOffset),[0,2]);
assert.deepEqual(extra.texts[5].variationSelectors,[1,2]);assert.deepEqual(extra.texts[5].defaultIgnorables,[1,2,3]);
const long=query('long-contexts',{texts:['A'+'\u0301'.repeat(65535),'🇨'.repeat(32768),'\u094d'+'\u0301'.repeat(65534)+'क'],characters:[]});
assert.equal(long.texts[0].boundaries.length,2);assert.equal(long.texts[1].boundaries.length,16385);assert.equal(long.texts[2].boundaries.length,2);
query('surrogate-property',{texts:[],characters:[0xd800]},'INPUT_INVALID');
query('overflow-property',{texts:[],characters:[0x110000]},'INPUT_INVALID');
query('negative-property',{texts:[],characters:[-1]},'INPUT_INVALID',false);
query('scalar-budget',{texts:['a'.repeat(65537)],characters:[]},'LIMIT_EXCEEDED');
query('byte-budget',{texts:['😀'.repeat(65537)],characters:[]},'LIMIT_EXCEEDED');
query('aggregate-budget',{texts:Array(5).fill('a'.repeat(65536)),characters:[]},'LIMIT_EXCEEDED');
query('texts-budget',{texts:Array(257).fill('a'),characters:[]},'LIMIT_EXCEEDED');
query('queries-budget',{texts:[],characters:Array(65537).fill(65)},'LIMIT_EXCEEDED');
query('invalid-json-surrogate','{"texts":["\\ud800"],"characters":[]}','INPUT_INVALID',false);
query('duplicate-key','{"texts":[],"texts":[],"characters":[]}','INPUT_INVALID',false);
query('unknown-field',{texts:[],characters:[],unicodeVersion:'17.0.0'},'INPUT_INVALID',false);
const report={format:'musteroffice.unicode-verification/1',nativeSha256:sha(readFileSync('target/release/mo-cli')),wasmSha256:sha(readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),fixturesSha256:sha(readFileSync(root+'/fixtures.json')),officialGraphemeCases:fixtures.officialGraphemeCases.length,officialCasesSkipped:0,propertyProbes:fixtures.propertyProbes.length,denseCodepointsCompared:fixtures.denseCodepointsCompared,denseLittleEndianSha256:fixtures.denseLittleEndianSha256,cases};
writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({nativeWasmCases:cases.length,officialGraphemeCases:report.officialGraphemeCases,propertyProbes:report.propertyProbes,skipped:0}));
