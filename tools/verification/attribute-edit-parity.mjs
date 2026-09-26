import fs from 'node:fs';import assert from 'node:assert/strict';
const root='.codex-work/attribute-edit';
const {instance}=await WebAssembly.instantiate(fs.readFileSync(root+'/probe.wasm'),{}),e=instance.exports;
const input=fs.readFileSync(root+'/probe-input.bin'),count=input.readUInt32LE();let at=4;const outputs=[];
assert.equal(e.reset(4*1024*1024+1),0);assert.equal(e.get(0),256);
for(let n=0;n<count;n++){
 const len=input.readUInt32LE(at);at+=4;assert.equal(e.reset(len),1);
 for(let i=0;i<len;i++)assert.equal(e.put(i,input[at+i]),1);
 assert.equal(e.put(len,0),0);assert.equal(e.put(0,256),0);at+=len;
 const size=e.execute(),body=Buffer.alloc(size);for(let i=0;i<size;i++)body[i]=e.get(i);
 assert.equal(e.get(size),256);const h=Buffer.alloc(4);h.writeUInt32LE(size);outputs.push(h,body);
}
assert.equal(at,input.length);const bytes=Buffer.concat(outputs);
assert.deepEqual(bytes,fs.readFileSync(root+'/probe-native.bin'));fs.writeFileSync(root+'/probe-wasm.bin',bytes);
console.log(JSON.stringify({pairedCalls:count,byteIdentical:true}));
