import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve, relative, join } from 'node:path';
import { spawnSync } from 'node:child_process';
const [cliArg, wasmArg, manifestArg] = process.argv.slice(2);
if (!manifestArg) throw new Error('usage: font-parity.mjs <cli> <wasm-js> <manifest>');
const cli=resolve(cliArg), wasmPath=resolve(wasmArg), wasm=createRequire(import.meta.url)(wasmPath);
const manifest=JSON.parse(readFileSync(manifestArg));
const directory=mkdtempSync(resolve('.codex-work/font-query-'));
const hash=b=>createHash('sha256').update(b).digest('hex');
const cases=[];
function query(name,sample,input,error=null,validRequest=true) {
  const bytes=readFileSync(sample.path);assert.equal(hash(bytes),sample.sha256);
  const path=join(directory,name+'.json');writeFileSync(path,input);
  const native=spawnSync(cli,['font-inspect',path,resolve(sample.path)],{encoding:'utf8',maxBuffer:64*1024*1024});
  assert.equal(native.status,0,native.stderr);
  const raw=wasm.inspect_font(input,bytes);
  assert.equal(raw,native.stdout.trimEnd(),name+': Native/WASM bytes differ');
  const response=JSON.parse(raw);
  if (error) { assert.equal(response.status,'error',name);assert.equal(response.error.code,error,name); }
  else { assert.equal(response.status,'inspected',name+': '+raw);assert.equal(response.font.sha256,sample.sha256); }
  assert.equal(hash(readFileSync(sample.path)),sample.sha256,'source modified');
  cases.push({name,source:relative(process.cwd(),resolve(sample.path)),sourceSha256:sample.sha256,request:input,validRequest,response});
}
for (const c of manifest.cases) query(c.name,c,JSON.stringify(c.request),c.error);
const base=manifest.cases.find(c=>c.name==='owned-ttf-0');
for (const [name,changes,error] of [
  ['digest-conflict',{expectedSha256:'0'.repeat(64)},'RESOURCE_CONFLICT'],
  ['unknown-face',{faceIndex:1},'FONT_INVALID'],
  ['invalid-scalar',{characters:[{codepoint:0xd800,variationSelector:null}]},'INPUT_INVALID'],
  ['invalid-selector',{characters:[{codepoint:65,variationSelector:65}]},'INPUT_INVALID'],
  ['query-budget',{characters:Array(100001).fill({codepoint:65,variationSelector:null})},'LIMIT_EXCEEDED'],
]) query(name,base,JSON.stringify({...base.request,...changes}),error);
for (const [name,input] of [
  ['unknown-field',JSON.stringify({...base.request,path:'/implicit/system/font'})],
  ['unknown-character-field',JSON.stringify({...base.request,characters:[{codepoint:65,variationSelector:null,fontFamily:'Arial'}]})],
  ['negative-scalar',JSON.stringify({...base.request,characters:[{codepoint:-1,variationSelector:null}]})],
  ['duplicate-field',JSON.stringify(base.request).replace('"faceIndex":','"faceIndex":0,"faceIndex":')],
]) query(name,base,input,'INPUT_INVALID',false);
console.log(JSON.stringify({format:'musteroffice.font-parity/1',nativeSha256:hash(readFileSync(cli)),
  wasmSha256:hash(readFileSync(join(resolve(wasmPath,'..'),'mo_wasm_bg.wasm'))),cases},null,2));
