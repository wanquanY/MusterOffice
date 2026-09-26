// Real authored absent/zero/explicit limits -> native package -> source index.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/source-lines/author';fs.mkdirSync(root,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const bundlePath='fixtures/presentations/native-export/resources.bin',bundle=fs.readFileSync(bundlePath),cases=[];
for(const [name,join] of [['absent',{kind:'miter'}],['zero',{kind:'miter',limit:0}],['explicit',{kind:'miter',limit:400000}]]){
 const q=JSON.parse(fs.readFileSync('fixtures/presentations/native-export/request.json'));
 q.document.objects['title:1'].appearance.stroke={kind:'value',value:{kind:'solid',width:'114300',cap:'flat',join,color:{kind:'srgb',rgba:{red:200,green:30,blue:50,alpha:255}}}};
 const json=JSON.stringify(q),requestPath=`${root}/${name}.json`,pptxPath=`${root}/${name}.pptx`;fs.writeFileSync(requestPath,json);fs.rmSync(pptxPath,{force:true});
 const n=spawnSync('target/release/mo-cli',['pptx-export',requestPath,bundlePath,pptxPath],{encoding:'utf8',timeout:30000});assert.equal(n.status,0,n.stderr);
 const bytes=fs.readFileSync(pptxPath);assert.deepEqual(bytes,Buffer.from(wasm.export_pptx(json,bundle)));
 const source=spawnSync('target/release/mo-cli',['pptx-inspect',pptxPath],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(source.status,0,source.stderr);
 const response=wasm.inspect_pptx(bytes);assert.equal(source.stdout.trimEnd(),response);const r=JSON.parse(response);assert.equal(r.status,'inspected');
 const line=r.index.surfaces['/ppt/slides/slide1.xml'].objects.find(o=>o.name==='title:1').line;
 assert.equal(line.join.limit,name==='absent'?null:String(join.limit));
 const responsePath=`${root}/${name}.response.json`;fs.writeFileSync(responsePath,response);
 cases.push({name,requestPath,requestSha256:sha(json),pptxPath,pptxSha256:sha(bytes),responsePath,responseSha256:sha(response),limit:line.join.limit});
}
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.source-line-author-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),bundleSha256:sha(bundle),cases},null,2)+'\n');
console.log(JSON.stringify({authorExportAndSourceCases:cases.length}));
