import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root = '.codex-work/source-effects';
const wasm = createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha = b => createHash('sha256').update(b).digest('hex');
const cases = [];
const fillCases = [];
function inspect(path) {
  const bytes = fs.readFileSync(path);
  const n = spawnSync('target/release/mo-cli', ['pptx-inspect', path], {encoding:'utf8', timeout:30000, maxBuffer:64*1024*1024});
  assert.equal(n.status, 0, n.stderr);
  const raw = wasm.inspect_pptx(bytes); assert.equal(n.stdout.trimEnd(), raw, path);
  return {value:JSON.parse(raw), raw};
}
function declarations(index) {
  // Text-only editing must preserve the complete non-text source model.
  const value=structuredClone(index);
  delete value.sourceSha256;
  delete value.byteLength;
  for(const s of Object.values(value.surfaces)) {
    delete s.sha256;
    for(const o of s.objects) o.paragraphs=[];
  }
  return value;
}

for (const c of JSON.parse(fs.readFileSync(root+'/manifest.json')).cases) {
  const bytes = fs.readFileSync(c.path); assert.equal(sha(bytes), c.sha256);
  const {value:r, raw} = inspect(c.path);
  assert.equal(r.status === 'error' ? r.error.code : r.status, c.error ?? 'inspected', c.name);
  const responsePath = root+'/'+c.name+'.response.json'; fs.writeFileSync(responsePath, raw);
  cases.push({name:'inspect-'+c.name, status:r.status === 'error' ? r.error.code : r.status, sourcePath:c.path, sourceSha256:c.sha256, responsePath, responseSha256:sha(raw)});
  if (r.status === 'error') continue;
  const object = r.index.surfaces['/ppt/slides/slide1.xml'].objects[0];
  const fillRequest = {expectedSourceSha256:c.sha256, surface:'/ppt/slides/slide1.xml',
    targets:[{kind:'object',nativeId:object.nativeId},{kind:'background'}], profile:'ms-oi29500-fills-2024-draft-v1'};
  const fillJson=JSON.stringify(fillRequest), fillRequestPath=root+'/'+c.name+'.fill.request.json';
  fs.writeFileSync(fillRequestPath,fillJson);
  const nativeFill=spawnSync('target/release/mo-cli',['pptx-fills',fillRequestPath,c.path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});
  assert.equal(nativeFill.status,0,nativeFill.stderr);
  const fillRaw=wasm.resolve_pptx_fills(fillJson,bytes);assert.equal(nativeFill.stdout.trimEnd(),fillRaw);
  const fills=JSON.parse(fillRaw);assert.equal(fills.status,'evaluated');
  if (['scope-background','scope-layout-background','scope-master-background','background-empty-dag'].includes(c.name)) {
    assert.equal(fills.styles.targets[1].outcome.reason.kind,'effectEvaluationRequired',c.name);
  }
  if (c.name==='background-empty-list') assert.equal(fills.styles.targets[1].outcome.fill.kind,'none');
  const fillResponsePath=root+'/'+c.name+'.fill.response.json';fs.writeFileSync(fillResponsePath,fillRaw);
  fillCases.push({name:c.name,status:'evaluated',sourcePath:c.path,sourceSha256:c.sha256,
    requestPath:fillRequestPath,requestSha256:sha(fillJson),responsePath:fillResponsePath,responseSha256:sha(fillRaw)});
  const request = {expectedSourceSha256:c.sha256, edits:[{target:{part:'/ppt/slides/slide1.xml', objectId:object.nativeId, paragraph:0, run:0}, expectedText:object.paragraphs[0][0].text, replacement:'effect source preserved 中文 & < >'}]};
  const json = JSON.stringify(request), requestPath = root+'/'+c.name+'.edit.json', outputPath = root+'/'+c.name+'.edited.pptx';
  fs.writeFileSync(requestPath, json); fs.rmSync(outputPath, {force:true});
  const n = spawnSync('target/release/mo-cli', ['pptx-edit-text', requestPath, c.path, outputPath], {encoding:'utf8', timeout:30000, maxBuffer:64*1024*1024});
  assert.equal(n.status, 0, n.stderr);
  const output = fs.readFileSync(outputPath); assert.deepEqual(output, Buffer.from(wasm.edit_pptx_text(json, bytes))); assert.equal(JSON.parse(n.stdout).report.sha256, sha(output));
  const {value:after, raw:afterRaw} = inspect(outputPath); assert.equal(after.status, 'inspected');
  assert.deepEqual(declarations(after.index), declarations(r.index), c.name);
  const candidateResponsePath = root+'/'+c.name+'.edited.response.json'; fs.writeFileSync(candidateResponsePath, afterRaw);
  cases.push({name:'edit-'+c.name, status:'edited', requestPath, requestSha256:sha(json), sourcePath:c.path, sourceSha256:c.sha256, pptxPath:outputPath, pptxSha256:sha(output), responsePath:candidateResponsePath, responseSha256:sha(afterRaw), fillDeclarationsPreserved:true});
}
const report = {format:'musteroffice.source-effect-parity/1', nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')), rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')), cases, exactIndexAndCandidateBytes:true};
fs.writeFileSync(root+'/parity.json', JSON.stringify(report,null,2)+'\n');
fs.writeFileSync(root+'/fill-parity.json', JSON.stringify({...report,cases:fillCases},null,2)+'\n');
console.log(JSON.stringify({batches:cases.length, edited:cases.filter(c=>c.fillDeclarationsPreserved).length}));
