// Test-only adapters for host-owned files. All paths come from this verifier's
// fresh private directory, never from documents or model/tool parameters.
import assert from 'node:assert/strict';
import { open, mkdir, stat, rm } from 'node:fs/promises';

export async function fileSource(path){
  const size=(await stat(path,{bigint:true})).size;
  return {byteLength:size,async readRange(offset,length){
    assert.ok(offset>=0n&&offset+BigInt(length)<=size&&offset<=BigInt(Number.MAX_SAFE_INTEGER));
    const f=await open(path,'r');const bytes=Buffer.alloc(length);let done=0;
    try{while(done<length){const r=await f.read(bytes,done,length-done,Number(offset)+done);assert.ok(r.bytesRead);done+=r.bytesRead;}}
    finally{await f.close();}
    return bytes;
  }};
}

export class FileCandidates {
  constructor(root){this.root=root;this.next=0;this.created=[];this.discarded=[];}
  async begin(asset){
    const path=new URL(`${String(this.next++).padStart(3,'0')}.bin`,this.root);
    const f=await open(path,'wx');let closed=false;
    this.created.push({assetId:asset.id,path:String(path)});
    return {async write(bytes){let done=0;while(done<bytes.length){const r=await f.write(bytes,done,bytes.length-done);assert.ok(r.bytesWritten);done+=r.bytesWritten;}},
      async seal(){await f.sync();await f.close();closed=true;return {reference:path,source:await fileSource(path),async release(){}};},
      discard:async()=>{if(!closed){await f.close();closed=true;}await rm(path);this.discarded.push(String(path));}};
  }
  static async create(root){await mkdir(root);return new FileCandidates(root);}
}
