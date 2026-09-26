import fs from 'node:fs';import assert from 'node:assert/strict';
import factory from '../../.codex-work/gradient-coordinates/probe.mjs';
const root='.codex-work/gradient-coordinates',m=await factory(),input=fs.readFileSync(root+'/input.bin'),reference=fs.readFileSync(root+'/native.bin');
const count=input.readUInt32LE();assert.equal(input.length,4+count*52);assert.equal(reference.length,count*16);
const p=m._malloc(52),q=m._malloc(16);assert(p&&q);const output=Buffer.alloc(reference.length);
for(let i=0;i<count;i++){
 for(let j=0;j<13;j++)m.HEAPU32[p/4+j]=input.readUInt32LE(4+i*52+j*4);
 m._mo_coordinates_probe(p,q);
 for(let j=0;j<4;j++)output.writeUInt32LE(m.HEAPU32[q/4+j],i*16+j*4);
}
m._free(p);m._free(q);assert.deepEqual(output,reference);fs.writeFileSync(root+'/wasm.bin',output);console.log(JSON.stringify({cases:count}));
