import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { ClientError, OfficeClient, ResourceClient } from '../../.codex-work/operation-client/build/operation-client/src/index.js';

const deferred = () => { let resolve, reject; const promise = new Promise((a,b) => { resolve=a; reject=b; }); return {promise,resolve,reject}; };
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const digests = {sha256() { const h=createHash('sha256'); return {update: bytes=>h.update(bytes),finish:()=>h.digest('hex')}; }};
const source = bytes => ({byteLength:BigInt(bytes.length), async readRange(offset,n) { return bytes.slice(Number(offset),Number(offset)+n); }});
const info = bytes => ({id:'asset:one',verification:'bytesSha256',descriptor:{sha256:sha(bytes),byteLength:String(bytes.length),mediaType:'application/octet-stream'}});
const basePort = () => ({async dispatch(){throw Error('unexpected dispatch');},async appendUpload(){throw Error('unexpected append');},async readAssetRange(){throw Error('unexpected range');}});
const errorCode = code => error => error instanceof ClientError && error.code === code;
const job = (id='job:one') => ({id,requestId:'request:one',state:'queued',contractVersion:'musteroffice.operations/1-draft',
  operation:'presentations.create',documentId:'document:one',requestDigest:'0'.repeat(64),executorDigest:'1'.repeat(64),
  cancelRequested:false,fence:'0',createdAt:'0',updatedAt:'0',leaseUntil:null,result:null});
const accepted = () => ({outcome:'accepted',job:job(),pollAfterMs:50});

test('aborted submit preserves identity and observes late host rejection without cancel/resubmit', async()=>{
  const work=deferred(), calls=[], controller=new AbortController();
  const client=new OfficeClient({...basePort(),dispatch(request){calls.push(request.operation);return work.promise;}});
  const pending=client.submit({requestId:'stable-id'}, {signal:controller.signal});
  controller.abort();
  await assert.rejects(pending,e=>errorCode('WAIT_ABORTED')(e)&&e.recovery.requestId==='stable-id');
  work.reject(Error('late transport failure'));
  await new Promise(resolve=>setImmediate(resolve));
  assert.deepEqual(calls,['submit']);
});

test('pre-aborted call has no host effect', async()=>{
  const controller=new AbortController();controller.abort();let called=0;
  const client=new OfficeClient({...basePort(),async dispatch(){called++;return accepted();}});
  await assert.rejects(client.submit({requestId:'r'},{signal:controller.signal}),errorCode('WAIT_ABORTED'));
  assert.equal(called,0);
});

test('wait timeout interrupts a stalled transport and keeps durable job identity', async()=>{
  const work=deferred(), calls=[];
  const client=new OfficeClient({...basePort(),dispatch(r){calls.push(r.operation);return work.promise;}});
  await assert.rejects(client.waitForJob('job:one',{timeoutMs:20}),e=>errorCode('WAIT_TIMEOUT')(e)&&e.recovery.jobId==='job:one');
  work.resolve(accepted());
  assert.deepEqual(calls,['getJob']);
});

test('poll hint is honored; explicit cancellation remains a separate operation', async()=>{
  const calls=[];
  const client=new OfficeClient({...basePort(),async dispatch(r){calls.push(r.operation);return accepted();}});
  await assert.rejects(client.waitForJob('job:one',{timeoutMs:20}),errorCode('WAIT_TIMEOUT'));
  assert.deepEqual(calls,['getJob']);
  assert.equal((await client.cancelJob('job:one')).outcome,'accepted');
  assert.deepEqual(calls,['getJob','cancelJob']);
});

test('foreign job replies and success without a receipt are rejected', async()=>{
  const other={...accepted(),job:job('job:other')};
  const client=new OfficeClient({...basePort(),async dispatch(){return other;}});
  await assert.rejects(client.getJob('job:one'),errorCode('HOST_PROTOCOL'));
  other.outcome='succeeded';other.result={kind:'job',job:{...job(),state:'succeeded'}};
  await assert.rejects(client.getJob('job:one'),errorCode('HOST_PROTOCOL'));
});

test('lost upload acknowledgement resumes at host-committed offset and never auto-cancels', async()=>{
  const bytes=Uint8Array.from([1,2,3,4,5,6,7]), declaration=info(bytes), offsets=[], operations=[];
  let received=0, lose=true;
  const upload=()=>({id:'upload:one',requestId:'upload-request',descriptor:declaration.descriptor,
    state:'uploading',chunkBytes:3,receivedBytes:String(received),createdAt:'0',updatedAt:'0',asset:null,error:null});
  const port={...basePort(),async dispatch(r){operations.push(r.operation);const value=upload();
    if(r.operation==='sealUpload'){value.state='sealed';value.asset=declaration;}
    return {outcome:'succeeded',result:{kind:'upload',upload:value}};
  },async appendUpload(id,offset,part){assert.equal(id,'upload:one');assert.equal(Number(offset),received);
    offsets.push(offset);assert.deepEqual(part,bytes.slice(received,received+part.length));received+=part.length;
    if(lose){lose=false;throw Error('ack lost after durable append');} return upload();}};
  const client=new ResourceClient(new OfficeClient(port));const request={requestId:'upload-request',descriptor:declaration.descriptor};
  await assert.rejects(client.upload(request,source(bytes)),/ack lost/);
  assert.deepEqual(await client.upload(request,source(bytes)),declaration);
  assert.deepEqual(offsets,['0','3','6']);assert.deepEqual(operations,['beginUpload','beginUpload','sealUpload']);
});

test('upload resume uses exact offsets above the JS safe integer range', async()=>{
  const offset=9007199254740993n, size=offset+1n, descriptor={byteLength:String(size),sha256:'0'.repeat(64),mediaType:'x/test'};
  let received=offset, calls=[];
  const value=state=>({id:'upload:big',requestId:'big',descriptor,state,receivedBytes:String(received),chunkBytes:1,
    createdAt:'0',updatedAt:'0',error:null,asset:state==='sealed'?{id:'asset:big',descriptor,verification:'bytesSha256'}:null});
  const port={...basePort(),async dispatch(r){return {outcome:'succeeded',result:{kind:'upload',upload:value(r.operation==='sealUpload'?'sealed':'uploading')}};},
    async appendUpload(_id,start,bytes){calls.push(start);assert.equal(start,String(offset));assert.equal(bytes.length,1);received=size;return value('uploading');}};
  const reader={byteLength:size,async readRange(start,n){assert.equal(start,offset);assert.equal(n,1);return new Uint8Array([4]);}};
  await new ResourceClient(new OfficeClient(port),{maxAssetBytes:size}).upload({requestId:'big',descriptor},reader);
  assert.deepEqual(calls,[String(offset)]);
});

function download(bytes, change={}) {
  const asset=info(bytes), calls=[], port={...basePort(),async dispatch(){return {outcome:'succeeded',result:{kind:'asset',asset}};},
    async readAssetRange(_id,offset,n){calls.push(['read',offset,n]);return bytes.slice(Number(offset),Number(offset)+n);}};
  const pieces=[];let discarded=0,sealed=0;
  const sink={async write(part){pieces.push(part.slice());},async seal(){sealed++;const raw=Buffer.concat(pieces);
    return {reference:'private:one',source:source(change.corrupt?Uint8Array.from(raw,b=>b^1):raw),async release(){}};},
    async discard(){discarded++;},...change.sink};
  const store={async begin(){return sink;}};
  return {client:new ResourceClient(new OfficeClient(port),{chunkBytes:3}),store,sink,asset,calls,
    state:()=>({discarded,sealed}),digests};
}

test('download hashes sealed stored bytes and does not publish', async()=>{
  const d=download(new Uint8Array([3,4,5,6,7,8,9]));
  assert.deepEqual(await d.client.copyAsset('asset:one',d.store,d.digests),{asset:d.asset,reference:'private:one'});
  assert.deepEqual(d.calls,[['read','0',3],['read','3',3],['read','6',1]]);
  assert.deepEqual(d.state(),{discarded:0,sealed:1});
});

test('a sink that corrupts stored bytes is discarded even though input bytes matched', async()=>{
  const d=download(new Uint8Array([1,2,3]),{corrupt:true});
  await assert.rejects(d.client.copyAsset('asset:one',d.store,d.digests),errorCode('INTEGRITY'));
  assert.deepEqual(d.state(),{discarded:1,sealed:1});
});

test('abort waits for private write completion before discard; no late seal', async()=>{
  const write=deferred(), started=deferred(), controller=new AbortController(), order=[];
  const d=download(new Uint8Array([1,2,3]),{sink:{async write(){order.push('write');started.resolve();await write.promise;order.push('written');},
    async seal(){assert.fail('must not seal after abort');},async discard(){order.push('discard');}}});
  const pending=d.client.copyAsset('asset:one',d.store,d.digests,{signal:controller.signal});
  await started.promise;controller.abort();await new Promise(resolve=>setImmediate(resolve));assert.deepEqual(order,['write']);
  write.resolve();await assert.rejects(pending,errorCode('WAIT_ABORTED'));assert.deepEqual(order,['write','written','discard']);
});

test('late candidate creation is cleaned after abort, without writing', async()=>{
  const begin=deferred(), started=deferred(), controller=new AbortController();let discarded=0;
  const d=download(new Uint8Array([1]));
  const pending=d.client.copyAsset('asset:one',{async begin(){started.resolve();return begin.promise;}},d.digests,{signal:controller.signal});
  await started.promise;controller.abort();
  begin.resolve({async write(){assert.fail();},async seal(){assert.fail();},async discard(){discarded++;}});
  await assert.rejects(pending,errorCode('WAIT_ABORTED'));assert.equal(discarded,1);
});

test('empty asset is sealed and its actual empty bytes are hashed', async()=>{
  const d=download(new Uint8Array());const copied=await d.client.copyAsset('asset:one',d.store,d.digests);
  assert.equal(copied.asset.descriptor.sha256,sha(new Uint8Array()));assert.deepEqual(d.calls,[]);
});

test('transfer and cleanup failures both remain observable', async()=>{
  const d=download(new Uint8Array([1]),{sink:{async write(){throw Error('disk full');},async discard(){throw Error('cleanup unavailable');}}});
  await assert.rejects(d.client.copyAsset('asset:one',d.store,d.digests),e=>e instanceof AggregateError&&e.errors[0].message==='disk full'&&e.errors[1].message==='cleanup unavailable');
});

test('bundle pin mismatch is rejected before creating private output', async()=>{
  const d=download(new Uint8Array([1,2]));let starts=0;
  await assert.rejects(d.client.copyAsset('asset:one',{async begin(){starts++;assert.fail();}},d.digests,
    {expectedDescriptor:{...d.asset.descriptor,sha256:'f'.repeat(64)}}),errorCode('HOST_PROTOCOL'));
  assert.equal(starts,0);assert.deepEqual(d.calls,[]);
});

test('aborted private verification read settles, then releases, then discards', async()=>{
  const read=deferred(), started=deferred(), controller=new AbortController(), order=[];
  const d=download(new Uint8Array([1]),{sink:{async seal(){return {reference:'private:one',
    source:{byteLength:1n,async readRange(){started.resolve();await read.promise;order.push('read');return new Uint8Array([1]);}},
    async release(){order.push('release');}};},async discard(){order.push('discard');}}});
  const pending=d.client.copyAsset('asset:one',d.store,d.digests,{signal:controller.signal});
  await started.promise;controller.abort();await new Promise(resolve=>setImmediate(resolve));assert.deepEqual(order,[]);
  read.resolve();await assert.rejects(pending,errorCode('WAIT_ABORTED'));assert.deepEqual(order,['read','release','discard']);
});

test('reader release failure rejects and discards otherwise correct data', async()=>{
  const d=download(new Uint8Array([1]),{sink:{async seal(){return {reference:'private:one',source:source(new Uint8Array([1])),
    async release(){throw Error('reader lease still held');}};}}});
  await assert.rejects(d.client.copyAsset('asset:one',d.store,d.digests),/reader lease still held/);
  assert.equal(d.state().discarded,1);
});

test('numeric or noncanonical byte lengths cannot cross the binary boundary', async()=>{
  const client=new ResourceClient(new OfficeClient(basePort()));
  for(const length of [1,'01','18446744073709551616'])
    await assert.rejects(client.upload({requestId:'r',descriptor:{byteLength:length,sha256:'0'.repeat(64),mediaType:'x/test'}},source(new Uint8Array([1]))),errorCode('HOST_PROTOCOL'));
});
