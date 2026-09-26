// Independent test driver for the existing native owner's NDJSON/binary CLI.
// Not a shipped production transport or an EmbeddedHost implementation.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
export const digests={sha256(){const hash=createHash('sha256');return {update:b=>hash.update(b),finish:()=>hash.digest('hex')};}};

export class NativePort {
  constructor({binary,database,principal,scope,worker,workerSha,root,name}) {
    this.binary=binary;this.base=[database,principal,scope];this.root=root;this.name=name;
    this.calls=[];this.binaryCalls=[];this.pending=null;this.bytes=Buffer.alloc(0);this.stderr=[];
    this.p=spawn(binary,[...this.base,'--preview-worker',worker,workerSha,'--scheduled','1'],{stdio:['pipe','pipe','pipe']});
    this.exit=once(this.p,'close');this.p.stderr.on('data',b=>this.stderr.push(b));
    this.p.stdout.on('data',chunk=>{
      this.bytes=Buffer.concat([this.bytes,chunk]);
      if(this.bytes.length>32*1024*1024){this.fail(Error('test control response exceeded budget'));return;}
      const newline=this.bytes.indexOf(10);if(newline<0)return;
      const raw=this.bytes.subarray(0,newline);this.bytes=this.bytes.subarray(newline+1);
      const pending=this.pending;this.pending=null;
      if(!pending){this.fail(Error('unexpected response'));return;}
      try{const response=JSON.parse(raw);this.calls.push({request:pending.request,response});pending.resolve(response);}
      catch(error){pending.reject(error);}
    });
    this.p.on('error',e=>this.fail(e));
    this.p.on('close',code=>{if(this.pending)this.fail(Error(`native owner closed with ${code}`));});
  }
  fail(error){this.failure=error;this.pending?.reject(error);this.pending=null;}
  async dispatch(request){
    if(this.failure)throw this.failure;
    assert.equal(this.pending,null,'test driver allows one outstanding control request');
    return new Promise((resolve,reject)=>{
      this.pending={request:structuredClone(request),resolve,reject};
      this.p.stdin.write(JSON.stringify(request)+'\n',e=>{if(e)this.fail(e);});
    });
  }
  async binaryCommand(args,input=Buffer.alloc(0),maxOutput=1024*1024){
    const p=spawn(this.binary,[...this.base,...args],{stdio:['pipe','pipe','pipe']});
    const chunks=[],errors=[];let bytes=0,overflow=false;
    p.stdout.on('data',b=>{bytes+=b.length;if(bytes>maxOutput){overflow=true;p.kill();}else chunks.push(b);});
    p.stderr.on('data',b=>errors.push(b));p.stdin.end(input);
    const timer=setTimeout(()=>p.kill(),20_000);
    const [code]=await once(p,'close').finally(()=>clearTimeout(timer));
    assert.equal(overflow,false);assert.equal(code,0,Buffer.concat(errors).toString());assert.equal(errors.length,0);
    return Buffer.concat(chunks);
  }
  async appendUpload(id,offset,bytes){
    const response=JSON.parse(await this.binaryCommand(['append',id,offset],bytes));
    this.binaryCalls.push({kind:'append',id,offset,bytes:bytes.length,response});
    assert.equal(response.outcome,'succeeded',JSON.stringify(response));assert.equal(response.result.kind,'upload');
    return response.result.upload;
  }
  async readAssetRange(id,offset,length){
    const bytes=await this.binaryCommand(['read-asset',id,offset,String(length)],undefined,length);
    this.binaryCalls.push({kind:'read',id,offset,length,actualBytes:bytes.length,sha256:sha(bytes)});
    return bytes;
  }
  async close(){
    assert.equal(this.pending,null);this.p.stdin.end();
    const timer=setTimeout(()=>this.p.kill(),30_000);
    const [code]=await this.exit.finally(()=>clearTimeout(timer));
    await writeFile(new URL(`${this.name}.calls.json`,this.root),JSON.stringify(this.calls,null,2)+'\n');
    await writeFile(new URL(`${this.name}.binary.json`,this.root),JSON.stringify(this.binaryCalls,null,2)+'\n');
    await writeFile(new URL(`${this.name}.stderr`,this.root),Buffer.concat(this.stderr));
    assert.equal(code,0);assert.equal(this.stderr.length,0);if(this.failure)throw this.failure;
  }
  async kill(){if(this.p.exitCode===null)this.p.kill();await this.exit;}
}

export async function artifact(path){const bytes=await readFile(path);return {path:String(path),byteLength:bytes.length,sha256:sha(bytes)};}
