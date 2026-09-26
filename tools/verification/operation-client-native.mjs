import assert from 'node:assert/strict';
import { readFile,writeFile,mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { ClientError,OfficeClient,ResourceClient } from '../../.codex-work/operation-client/build/operation-client/src/index.js';
import { NativePort,sha,digests,artifact } from './operation-client/native-port.mjs';
import { FileCandidates,fileSource } from './operation-client/file-store.mjs';

const [directory,binaryInput,workerInput]=process.argv.slice(2);assert.ok(directory&&binaryInput&&workerInput);
const root=pathToFileURL(resolve(directory)+'/');await mkdir(root);
const binary=resolve(binaryInput),worker=resolve(workerInput),workerSha=sha(await readFile(worker));
const config={binary,worker,workerSha,root,database:resolve(directory,'host.sqlite'),principal:'sdk-verifier',scope:'sdk-fixture'};
const fixture=JSON.parse(await readFile('fixtures/presentations/delivery/input.json','utf8'));
const operation=(action,requestId,outputMode='job')=>({contractVersion:'musteroffice.operations/1-draft',requestId,
  profileId:action.kind==='export'?'presentations-pptx-resource-delivery-v1-draft':'presentations-author-model-v01-draft',outputMode,action});
const port=new NativePort({...config,name:'initial'});let completed,initial,edited,exportRequest,largeAsset;
let recoveredAcknowledgement=false;
const wrapped={dispatch:r=>port.dispatch(r),readAssetRange:(...args)=>port.readAssetRange(...args),
  async appendUpload(...args){const result=await port.appendUpload(...args);
    if(args[0]===lostUploadId&&!recoveredAcknowledgement){recoveredAcknowledgement=true;throw Error('simulated acknowledgement loss after REAL durable append');}
    return result;}};
let lostUploadId;
const client=new OfficeClient(wrapped),resources=new ResourceClient(client);
const settle=async response=>response.outcome==='accepted'?client.waitForJob(response.job.id):response;
const receipt=response=>{assert.equal(response.outcome,'succeeded',JSON.stringify(response));return response.result.job.result.receipt;};
const upload=async(name,path,mediaType)=>{const bytes=await readFile(path);return resources.upload({requestId:name,
  descriptor:{byteLength:String(bytes.length),sha256:sha(bytes),mediaType}},await fileSource(path));};

try{
  const capabilities=await client.result({operation:'capabilities'},'capabilities');
  assert.equal(capabilities.capabilities.queuedExecution,'hostScheduled');
  initial=receipt(await settle(await client.submit(operation({kind:'create',document:fixture.document},'create'))));
  const before=(await client.result({operation:'readDocument',documentId:initial.documentId},'document')).snapshot;
  const edit=operation({kind:'apply',documentId:initial.documentId,baseRevision:initial.revision,
    operations:[{operationId:'title',operation:{kind:'setTitle',title:'SDK real integration'}}]},'edit');
  edited=receipt(await settle(await client.submit(edit)));
  const after=(await client.result({operation:'readDocument',documentId:initial.documentId},'document')).snapshot;
  assert.equal(after.document.title,'SDK real integration');
  assert.deepEqual((await client.result({operation:'readDocument',documentId:initial.documentId,revision:initial.revision},'document')).snapshot,before);
  assert.deepEqual(receipt(await client.submit(edit)),edited);
  const image=await upload('image','fixtures/presentations/native-export/resources.bin','image/png');
  const font=await upload('font','fixtures/fonts/owned.ttf','application/octet-stream');
  // Multi-chunk real storage. Lose only the first durable append acknowledgement.
  const bytes=Uint8Array.from({length:3*262144+37},(_,i)=>(i*13+7)%251);
  const path=new URL('large.bin',root);await writeFile(path,bytes);
  const request={requestId:'large',descriptor:{byteLength:String(bytes.length),sha256:sha(bytes),mediaType:'application/octet-stream'}};
  lostUploadId=(await client.result({operation:'beginUpload',request},'upload')).upload.id;
  await assert.rejects(resources.upload(request,await fileSource(path)),/acknowledgement loss/);
  largeAsset=await resources.upload(request,await fileSource(path));assert.equal(recoveredAcknowledgement,true);
  const copies=await FileCandidates.create(new URL('large-candidate/',root));
  const copied=await resources.copyAsset(largeAsset.id,copies,digests,{expectedDescriptor:request.descriptor});
  assert.deepEqual(await readFile(copied.reference),Buffer.from(bytes));assert.equal(copies.discarded.length,0);
  exportRequest=operation({kind:'export',documentId:initial.documentId,baseRevision:edited.revision,settings:{delivery:fixture.settings,
    resources:[{resourceId:'resource:checker',assetId:image.id}],fontAssetId:font.id,
    renderer:{implementationSha256:workerSha,profile:'drawingml-resource-page-q32-v1-draft'}}},'export');
  completed=await settle(await client.submit(exportRequest));const exported=receipt(completed);
  assert.equal(exported.revision,edited.revision);assert.equal(exported.bundle.assets.length,12);
  const store=await FileCandidates.create(new URL('candidate/',root));
  const assets=[];
  for(const a of exported.bundle.assets){
    const result=await resources.copyAsset(a.id,store,digests,{expectedDescriptor:{byteLength:a.byteLength,sha256:a.sha256,mediaType:a.mediaType}});
    assets.push({asset:a,candidate:await artifact(result.reference)});
  }
  assert.equal(store.discarded.length,0);
  await writeFile(new URL('bundle.json',root),JSON.stringify(exported.bundle,null,2)+'\n');
  await writeFile(new URL('assets.json',root),JSON.stringify(assets,null,2)+'\n');
  assert.deepEqual(await client.cancelJob(completed.result.job.id),completed);
  await port.close();
}catch(error){await port.kill();throw error;}

const reconnected=new NativePort({...config,name:'reconnected'});
try{
  const client=new OfficeClient(reconnected);
  assert.deepEqual(await client.getJob(completed.result.job.id),completed);
  assert.deepEqual(await client.submit(exportRequest),completed);
  await reconnected.close();
}catch(error){await reconnected.kill();throw error;}

const denied=new NativePort({...config,scope:'different-scope',name:'denied'});let privateStores=0;
try{
  const resources=new ResourceClient(new OfficeClient(denied));
  await assert.rejects(resources.copyAsset(largeAsset.id,{async begin(){privateStores++;assert.fail();}},digests),
    e=>e instanceof ClientError&&e.code==='HOST_FAILURE'&&e.failure.code==='NOT_FOUND');
  assert.equal(privateStores,0);await denied.close();
}catch(error){await denied.kill();throw error;}

const report={binary:await artifact(binary),worker:await artifact(worker),node:process.version,platform:process.platform,arch:process.arch,
  jobId:completed.result.job.id,initial,edited,assets:12,recoveredAcknowledgement,largeAsset,
  resumedAppendOffsets:port.binaryCalls.filter(c=>c.kind==='append'&&c.id===lostUploadId).map(c=>c.offset),
  controlCalls:port.calls.length+reconnected.calls.length+denied.calls.length,binaryCalls:port.binaryCalls.length,
  retriesAndReconnectUnchanged:true,crossScopeRejectedBeforePrivateStore:true};
assert.deepEqual(report.resumedAppendOffsets,['0','262144','524288','786432']);
await writeFile(new URL('report.json',root),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report));
