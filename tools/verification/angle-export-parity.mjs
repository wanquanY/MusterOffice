// Exercise the actual native/WASM exporter and source reader using authored
// signed, full-turn, boundary, and i32-extreme static rotations.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/angle-export',read=p=>JSON.parse(fs.readFileSync(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const fixture=read('fixtures/presentations/native-export/request.json');
const resourcePath='fixtures/presentations/native-export/resources.bin',resources=fs.readFileSync(resourcePath);
const cases=[],angles=[-2147483648,-2147483647,-43200001,-43200000,-21600001,-21600000,-18900000,-8100000,-2700000,-1,0,1,2699999,2700000,8100000,13500000,18900000,21600000,24300000,2147483647];
for(const angle of angles){
 const request=structuredClone(fixture);
 for(const o of Object.values(request.document.objects))if(o.content.kind!=='connector'||angle%21600000===0)o.transform.rotation=angle;
 const normalized=structuredClone(request);
 for(const o of Object.values(normalized.document.objects))o.transform.rotation=((o.transform.rotation%21600000)+21600000)%21600000;
 const stem=`${root}/angle-${angle}`,requestPath=stem+'.request.json',canonicalRequestPath=stem+'.canonical.json';
 const json=JSON.stringify(request),canonical=JSON.stringify(normalized);fs.writeFileSync(requestPath,json);fs.writeFileSync(canonicalRequestPath,canonical);
 const pptxPath=stem+'.pptx',canonicalPath=stem+'.canonical.pptx';
 for(const [input,output]of [[requestPath,pptxPath],[canonicalRequestPath,canonicalPath]]){
  fs.rmSync(output,{force:true});const r=spawnSync('target/release/mo-cli',['pptx-export',input,resourcePath,output],{encoding:'utf8',maxBuffer:16*1024*1024,timeout:30000});assert.equal(r.status,0,r.stderr);
 }
 const bytes=fs.readFileSync(pptxPath);assert.deepEqual(bytes,fs.readFileSync(canonicalPath));
 assert.deepEqual(bytes,Buffer.from(wasm.export_pptx(json,resources)));assert.deepEqual(bytes,Buffer.from(wasm.export_pptx(canonical,resources)));
 assert.equal(fs.readFileSync(requestPath,'utf8'),json);
 const n=spawnSync('target/release/mo-cli',['pptx-inspect',pptxPath],{encoding:'utf8',maxBuffer:16*1024*1024,timeout:30000});assert.equal(n.status,0,n.stderr);
 const w=wasm.inspect_pptx(bytes);assert.equal(n.stdout.trimEnd(),w);const index=JSON.parse(w).index;
 for(const surface of Object.values(index.surfaces))for(const object of surface.objects){
  const expected=normalized.document.objects[object.name];assert(expected);assert.equal(object.transform.rotation,expected.transform.rotation);
 }
 const responsePath=stem+'.index.json';fs.writeFileSync(responsePath,w);
 cases.push({name:'angle-'+angle,angle,requestPath,requestSha256:sha(json),canonicalRequestPath,canonicalRequestSha256:sha(canonical),pptxPath,pptxSha256:sha(bytes),byteLength:bytes.length,responsePath,responseSha256:sha(w)});
}
const report={format:'musteroffice.angle-export-parity/1',scope:'20 logical batches, each with raw and equivalent canonical author requests through native and WASM export, plus native/WASM source-index validation. Native authored input is not mutated.',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases};
fs.writeFileSync(root+'/angle-parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,equivalentExportRequests:cases.length*2}));
