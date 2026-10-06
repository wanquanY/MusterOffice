/** Exercise the shipped ESM entry and explicit module lifetime, not a source mock. */
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {Worker} from 'node:worker_threads';
const [directory,reportPath]=process.argv.slice(2);
assert(reportPath,'usage: video-sdk-check.mjs SDK_DIRECTORY REPORT');
const root=resolve(directory);
const entry=pathToFileURL(join(root,'index.mjs')).href;
const bytes=await readFile(join(root,'runtime/mo_video_wasm_bg.wasm'));
const module=await WebAssembly.compile(bytes);
const {createVideoRuntime,VideoComputationError}=await import(entry);
const sdk=await createVideoRuntime(module);
const create={documentId:'packaged-sdk',title:'本地剪辑',rootSequence:'main',canvas:{width:1280,height:720},frameRate:{numerator:30000,denominator:1001},audioSampleRate:48000};
const project=sdk.create(create);
const candidate=project.prepare({contract:'musteroffice.video.edit/1',expectedDigest:project.summary().documentDigest,operations:[{type:'setTitle',title:'已编辑'}]});
assert.equal(project.summary().title,create.title);
const changed=sdk.open(candidate.snapshot);
assert.equal(changed.summary().title,'已编辑');
const inverse=changed.prepareInverse({expectedDigest:changed.summary().documentDigest,inverse:candidate.inverse});
assert.deepEqual(inverse.snapshot,project.snapshot());
assert.throws(()=>changed.frame({expectedDigest:project.summary().documentDigest,frame:'0'}),e=>e instanceof VideoComputationError&&e.failure.code==='STALE_SNAPSHOT');
assert.deepEqual(changed.resources().resources,[]);
assert.equal(changed.list({query:{kind:'sequences'},limit:10}).items.length,1);
changed.close();project.close();
assert.throws(()=>project.summary(),/closed/);
await assert.rejects(()=>createVideoRuntime(module),/already initialized/);
// Fresh worker means fresh module ownership. A rejected implicit fetch input
// consumes this worker's initialization, rather than silently trying a URL.
const outcome=await new Promise((resolve,reject)=>{
  const worker=new Worker(`const {parentPort,workerData}=require('node:worker_threads');
    (async()=>{const sdk=await import(workerData.entry);let first,second;
      try{await sdk.createVideoRuntime('https://invalid.example/video.wasm')}catch(e){first=e.message}
      try{await sdk.createVideoRuntime(workerData.module)}catch(e){second=e.message}
      parentPort.postMessage({first,second});
    })().catch(e=>{throw e});`,{eval:true,workerData:{entry,module}});
  let result;worker.once('message',v=>{result=v});worker.once('error',reject);
  worker.once('exit',code=>code===0?resolve(result):reject(new Error(`SDK worker exited ${code}`)));
});
assert.match(outcome.first,/explicit compiled/);
assert.match(outcome.second,/already initialized/);
const sha=data=>createHash('sha256').update(data).digest('hex');
const report={format:'musteroffice.video-sdk-runtime/1',status:'passed',wasmSha256:sha(bytes),wasmBytes:bytes.length,
  manifestSha256:sha(await readFile(join(root,'bundle-manifest.json'))),
  scope:'Packaged ESM runtime executed in Node with explicit WebAssembly.Module, edits, inverse, typed failure and handle/module lifetime. No browser media or product persistence acceptance.'};
await writeFile(reportPath,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
