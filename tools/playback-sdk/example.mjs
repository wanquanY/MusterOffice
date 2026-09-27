/** Node host example. Files, Worker lifetime and transport are owned here by the
 * receiving product, not by the synchronous computation SDK. Import openWorker. */
import {Worker, isMainThread, parentPort, workerData} from 'node:worker_threads';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {createDispatcher, failure} from './dispatch.mjs';

const sha=bytes=>createHash('sha256').update(bytes).digest('hex');

/** Verify the externally pinned package before importing executable modules. */
export async function openWorker(directory, manifestSha256) {
  if(!isMainThread)throw Error('Host entry must run outside the computation Worker');
  directory=path.resolve(directory);
  const manifestBytes=await fs.readFile(path.join(directory,'bundle-manifest.json'));
  if(manifestBytes.length>1024*1024||sha(manifestBytes)!==manifestSha256)throw Error('SDK manifest pin mismatch');
  const manifest=JSON.parse(manifestBytes);
  if(manifest.format!=='musteroffice.wasm-playback-sdk/1-draft'||manifest.releaseCleared!==false)throw Error('SDK format');
  const names=new Set();
  for(const file of manifest.files){
    if(!/^[a-zA-Z0-9_.-]+(?:\/[a-zA-Z0-9_.-]+)*$/.test(file.path)||file.path.split('/').includes('..')||names.has(file.path))throw Error('SDK path');
    names.add(file.path);
    let current=directory;
    for(const part of file.path.split('/')){current=path.join(current,part);if((await fs.lstat(current)).isSymbolicLink())throw Error('SDK symlink');}
    const bytes=await fs.readFile(current);
    if(bytes.length!==file.byteLength||sha(bytes)!==file.sha256)throw Error('SDK bytes differ');
  }
  const actual=new Set();
  async function scan(relative=''){
    for(const child of await fs.readdir(path.join(directory,relative),{withFileTypes:true})){
      const name=relative?relative+'/'+child.name:child.name;
      if(child.isDirectory())await scan(name);
      else if(child.isFile())actual.add(name);
      else throw Error('SDK contains a non-regular entry');
    }
  }
  await scan();actual.delete('bundle-manifest.json');
  if(actual.size!==names.size||[...actual].some(name=>!names.has(name)))throw Error('SDK inventory differs');
  const required=['index.mjs','runtime/mo_wasm_bg.wasm','runtime/mo-skia.wasm','runtime/mo-hb.wasm'];
  if(required.some(p=>!names.has(p)))throw Error('SDK runtime missing');
  const code={};
  for(const [key,name] of [['kernel','mo_wasm_bg.wasm'],['raster','mo-skia.wasm'],['text','mo-hb.wasm']]){
    const bytes=await fs.readFile(path.join(directory,'runtime',name));
    if(sha(bytes)!==manifest.files.find(f=>f.path==='runtime/'+name).sha256)throw Error('SDK code changed');
    code[key]=await WebAssembly.compile(bytes);
  }
  // Keep this trusted directory immutable through the Worker lifetime.
  return new HostWorker(new Worker(new URL(import.meta.url),{
    workerData:{kind:'musteroffice.playback-example',entry:pathToFileURL(path.join(directory,'index.mjs')).href,code},
  }));
}

class HostWorker {
  #worker; #pending; #next=0; #closed=false; #terminated;
  constructor(worker){
    this.#worker=worker;
    worker.on('message',reply=>{
      if(this.#closed)return;
      const pending=this.#pending;
      if(!pending||reply.id!==pending.id){this.#fail(Error('Uncorrelated worker response'));return;}
      if(!reply.ok&&reply.fatal){this.#fail(Object.assign(Error(reply.error.message),reply.error));return;}
      this.#pending=undefined;clearTimeout(pending.timer);
      if(reply.ok)pending.resolve(reply.value);
      else pending.reject(Object.assign(Error(reply.error.message),reply.error));
    });
    worker.on('error',error=>this.#fail(error));
    worker.on('exit',code=>this.#fail(Error('Worker exited: '+code)));
  }
  #fail(error){
    this.#closed=true;
    const pending=this.#pending;
    if(pending){clearTimeout(pending.timer);this.#pending=undefined;}
    this.#terminated??=this.#worker.terminate();
    if(pending)void this.#terminated.then(()=>pending.reject(error),
      cleanup=>pending.reject(new AggregateError([error,cleanup],'Worker failure and termination failure')));
  }
  call(command,transfer=[],timeoutMs=30000){
    if(this.#closed)return Promise.reject(Error('Worker closed'));
    if(this.#pending)return Promise.reject(Error('Worker busy'));
    if(!Number.isFinite(timeoutMs)||timeoutMs<=0||timeoutMs>3600000)return Promise.reject(Error('Invalid deadline'));
    const id=++this.#next;
    return new Promise((resolve,reject)=>{
      this.#pending={id,resolve,reject,timer:setTimeout(()=>this.#fail(Error('Worker deadline')),timeoutMs)};
      try{this.#worker.postMessage({id,command},transfer);}catch(error){this.#fail(error);}
    });
  }
  async close(){this.#fail(Error('Worker stopped'));await this.#terminated;}
}

if(!isMainThread&&workerData?.kind==='musteroffice.playback-example'){
  // Missing code/input must fail; it cannot trigger runtime network fallback.
  globalThis.fetch=()=>{throw Error('Implicit fetch denied in example Worker');};
  const {createPlaybackRuntime}=await import(workerData.entry);
  const runtime=await createPlaybackRuntime(workerData.code);
  const dispatch=createDispatcher(runtime);
  parentPort.on('message',({id,command:q})=>{
    try{
      const value=dispatch(q);
      parentPort.postMessage({id,ok:true,value},value?.pixels?[value.pixels.buffer]:[]);
    }catch(error){parentPort.postMessage(failure(id,error,runtime.raster.invalid||runtime.shaping.invalid));}
  });
}
