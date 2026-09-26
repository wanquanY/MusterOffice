import fs from 'node:fs';
import assert from 'node:assert/strict';
import create from '../../.codex-work/elliptic-fast/prepared/probe.mjs';
const root='.codex-work/elliptic-fast/prepared/';const m=await create();
const input=fs.readFileSync(root+'input.bin'),n=input.length/28;
const a=m._malloc(input.length),b=m._malloc(n*40);assert.ok(a&&b);
try{
 m.HEAPU32.set(new Uint32Array(input.buffer,input.byteOffset,input.length/4),a/4);
 m._mo_elliptic_prepared_probe(a,b,n);
 const result=Buffer.from(m.HEAPU32.buffer,b,n*40);assert.deepEqual(result,fs.readFileSync(root+'native.bin'));
 fs.writeFileSync(root+'wasm.bin',result);console.log(JSON.stringify({calls:n,byteIdentical:true}));
}finally{m._free(a);m._free(b);}
