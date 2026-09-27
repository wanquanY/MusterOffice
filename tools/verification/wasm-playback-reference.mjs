/** Real independent Node Worker consumes the packaged SDK and owned fixtures. */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';

const [output,bundle,pin,author,source,mode='sync']=process.argv.slice(2);
assert(output&&bundle&&pin&&author&&source,'output bundle pin author-manifest source-manifest');
assert(['sync','stepped'].includes(mode));
await fs.mkdir(output,{recursive:false});
const sha=b=>createHash('sha256').update(b).digest('hex');
const inputs={};
async function load(record){
  const bytes=await fs.readFile(record.path);
  assert.equal(bytes.length,record.byteLength);assert.equal(sha(bytes),record.sha256);
  inputs[record.path]=record.sha256;return bytes;
}
const bundleManifest=await fs.readFile(path.join(bundle,'bundle-manifest.json'));
assert.equal(sha(bundleManifest),pin);
// Example modules and their relative imports are executable host code too.
for(const file of JSON.parse(bundleManifest).files){
  const bytes=await fs.readFile(path.join(bundle,file.path));
  assert.equal(bytes.length,file.byteLength);assert.equal(sha(bytes),file.sha256);
}
const example=JSON.parse(bundleManifest).files.find(f=>f.path==='examples/node-worker.mjs');
assert.equal(sha(await fs.readFile(path.join(bundle,example.path))),example.sha256);
const {openWorker}=await import(pathToFileURL(path.resolve(bundle,example.path)));
// This host example validates the manifest before loading executable SDK code.
await assert.rejects(()=>openWorker(bundle,'0'.repeat(64)),/manifest pin/);
const worker=await openWorker(bundle,pin);
const observations=[];
try{
  for(const [kind,manifestPath] of [['author',author],['source',source]]){
    const raw=await fs.readFile(manifestPath);inputs[manifestPath]=sha(raw);
    const previous=JSON.parse(raw);const groups=new Map();
    for(const c of previous.cases){
      const old=JSON.parse(await load(c.response));if(old.status!=='rendered')continue;
      const q=JSON.parse(await load(c.request));let request,sample;
      if(kind==='author'){
        request={snapshot:q.playback.snapshot,slide:q.playback.slide,binding:q.playback.binding,viewport:q.viewport,defaults:q.defaults};
        sample={at:q.playback.at,history:q.playback.history??null};
      }else{
        request={page:q.page,binding:q.sample.binding};sample={at:q.sample.at,history:q.sample.history??null};
      }
      const key=JSON.stringify(request);if(!groups.has(key))groups.set(key,[]);
      groups.get(key).push({c,old,sample});
    }
    for(const [key,cases] of groups){
      const request=JSON.parse(key);const command={operation:'prepare',kind,request};let transfer=[];
      if(kind==='source'){
        for(const name of ['source','fonts']){
          const bytes=await load(cases[0].c[name]);
          for(const c of cases)assert.deepEqual(await load(c.c[name]),bytes);
          command[name]=Uint8Array.from(bytes);transfer.push(command[name].buffer);
        }
      }
      const prepared=await worker.call(command,transfer);
      if(kind==='source'){
        assert.equal(command.source.byteLength,0);assert.equal(command.fonts.byteLength,0);
        assert.equal(prepared.inputsDetached,true);
      }
      const frames=[];
      for(const {c,old,sample} of cases){
        let frame;
        if(mode==='sync') frame=await worker.call({operation:'sample',...sample});
        else {
          assert.deepEqual(await worker.call({operation:'beginSample',...sample}),{begun:true});
          while(!(await worker.call({operation:'stepSample',workUnits:7})).complete) { /* Host owns scheduling. */ }
          frame=await worker.call({operation:'takeSample'});
        }
        assert.deepEqual(Buffer.from(frame.pixels),await load(c.pixels),c.name);
        if(kind==='author')assert.deepEqual(frame.info,old.info,c.name);
        else{
          assert.deepEqual(frame.info.playback,old.info.playback,c.name);
          assert.deepEqual(frame.info.page.page,old.info.page.page,c.name);
          assert.equal(frame.info.page.textWork.componentCalls,0);
          assert.equal(frame.info.page.textWork.fontUploadBytes,0);
          assert.equal(frame.info.page.gatherCopyBytes,0);
        }
        const stem=String(observations.length).padStart(3,'0')+'-'+String(frames.length).padStart(3,'0');
        await fs.writeFile(path.join(output,stem+'.rgba'),frame.pixels,{flag:'wx'});
        await fs.writeFile(path.join(output,stem+'.json'),JSON.stringify(frame.info),{flag:'wx'});
        frames.push({name:c.name,pixelSha256:sha(frame.pixels),byteLength:frame.pixels.length});
      }
      assert.deepEqual((await worker.call({operation:'timing'})).binding,request.binding);
      await assert.rejects(()=>worker.call({operation:'advance',generation:request.binding.generation}),
        error=>error.name==='PlaybackComputationError'&&error.diagnostic.code==='GENERATION_NOT_INCREASING');
      const generation='9007199254740993';
      const advanced=await worker.call({operation:'advance',generation});
      assert.equal(advanced.planId,prepared.info.planId);assert.equal(advanced.binding.generation,generation);
      assert.deepEqual(await worker.call({operation:'dispose'}),{disposed:true});
      observations.push({kind,frames,planId:prepared.info.planId,disposed:true});
    }
  }
}finally{await worker.close();}
for(const [p,h]of Object.entries(inputs))assert.equal(sha(await fs.readFile(p)),h);
const report={format:'musteroffice.wasm-playback-sdk-reference/1',status:'passed',mode,bundleManifestSha256:pin,
  observations,frameCount:observations.reduce((n,o)=>n+o.frames.length,0),inputs,
  inputBuffersTransferredThenDetachedBeforeSampling:true,implicitFetchDenied:true,
  scope:'Actual packaged Rust/Skia/HarfBuzz WASM in a host Node Worker; not browser, Office/WPS or complete PPT acceptance.'};
await fs.writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({owners:observations.length,frames:report.frameCount,status:report.status}));
