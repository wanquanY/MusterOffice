// Real CLI and Rust WASM: resource catalogs, exact encoded bytes and atomic host publication.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root=process.argv[2]??'.codex-work/source-images';
const wasm=createRequire(import.meta.url)(path.resolve(root,'wasm-node/mo_wasm.js'));
const out=root+'/'+(process.argv[3]??'runtime');fs.mkdirSync(out,{recursive:true});
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const save=(name,data)=>{const file=out+'/'+name;fs.writeFileSync(file,data);return entry(file);};
const cli=args=>{const r=spawnSync('target/debug/mo-cli',args,{encoding:'utf8',timeout:60000,maxBuffer:40*1024*1024});assert.equal(r.status,0,r.stderr);return r.stdout.trimEnd();};
const cases=[];
const fixtureRoot=process.argv[4]??root+'/cases', reportName=process.argv[5]??'parity';
const names=fs.readdirSync(fixtureRoot).filter(n=>n.endsWith('.request.json')).map(n=>n.slice(0,-13)).sort();
assert.equal(names.length,19);
for(const name of names){
 const base=fixtureRoot+'/'+name,request=fs.readFileSync(base+'.request.json','utf8'),source=fs.readFileSync(base+'.pptx');
 const native=cli(['pptx-images',base+'.request.json',base+'.pptx']);
 assert.equal(wasm.inspect_pptx_images(request,source),native,name);
 assert.deepEqual(JSON.parse(native),{status:'inspected',images:JSON.parse(fs.readFileSync(base+'.projection.json','utf8'))});
 const output=out+'/'+name+'.bin';assert(!fs.existsSync(output),'use a fresh runtime output directory');
 assert.equal(cli(['pptx-extract-images',base+'.request.json',base+'.pptx',output]),native);
 const extracted=wasm.extract_pptx_images(request,source);
 assert.equal(extracted.metadata,native);
 const bytes=Buffer.from(extracted.take_bytes());
 assert.deepEqual(bytes,fs.readFileSync(output));assert.deepEqual(bytes,fs.readFileSync(base+'.encoded.bin'));
 const before=entry(output);
 const exists=spawnSync('target/debug/mo-cli',['pptx-extract-images',base+'.request.json',base+'.pptx',output],{encoding:'utf8',timeout:60000});
 assert.notEqual(exists.status,0);assert.match(exists.stderr,/exist/i);assert.deepEqual(entry(output),before);
 cases.push({name,request:entry(base+'.request.json'),source:entry(base+'.pptx'),projection:entry(base+'.projection.json'),expectedBytes:entry(base+'.encoded.bin'),response:save(name+'.json',native),bytes:entry(output)});
}
const base=fixtureRoot+'/native-picture-reuse',valid=JSON.parse(fs.readFileSync(base+'.request.json','utf8')),source=fs.readFileSync(base+'.pptx');
const invalid=[
 ['unknown-field',JSON.stringify({...valid,extra:true}),'INPUT_INVALID'],
 ['unknown-selection',JSON.stringify({...valid,selection:'automatic'}),'INPUT_INVALID'],
 ['missing-selection',JSON.stringify({fill:valid.fill}),'INPUT_INVALID'],
 ['duplicate-member','{"selection":"linkedSource",'+JSON.stringify(valid).slice(1),'INPUT_INVALID'],
 ['bad-fill-target',JSON.stringify({...valid,fill:{...valid.fill,targets:[{kind:'picture',nativeId:-1}]}}),'INPUT_INVALID'],
 ['source-conflict',JSON.stringify({...valid,fill:{...valid.fill,expectedSourceSha256:'0'.repeat(64)}}),'SOURCE_CONFLICT'],
];
const failures=[];
for(const [name,request,code] of invalid){
 const q=save(name+'.request.json',request),output=out+'/'+name+'.bin';
 assert(!fs.existsSync(output));
 const native=cli(['pptx-extract-images',q.path,base+'.pptx',output]);
 assert.equal(cli(['pptx-images',q.path,base+'.pptx']),native);
 const extracted=wasm.extract_pptx_images(request,source);
 assert.equal(extracted.metadata,native);assert.equal(wasm.inspect_pptx_images(request,source),native);assert.equal(extracted.take_bytes().length,0);
 assert(!fs.existsSync(output));assert.equal(JSON.parse(native).status,'error');assert.equal(JSON.parse(native).error.code,code);
 failures.push({name,request:q,source:entry(base+'.pptx'),response:save(name+'.json',native),validSchema:name==='source-conflict'});
}
assert(!fs.readdirSync(out).some(n=>n.startsWith('.musteroffice-export-')));
const counts={catalogs:cases.length,invalidRequests:failures.length,pairedCalls:2*(cases.length+failures.length),noOverwriteChecks:cases.length};
fs.writeFileSync(root+'/'+reportName+'.json',JSON.stringify({format:'musteroffice.source-image-parity/1',counts,cases,failures},null,2)+'\n');
console.log(JSON.stringify(counts));
