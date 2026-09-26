import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import create from '../../.codex-work/elliptic-scalar/probe.mjs';
const root='.codex-work/elliptic-scalar/';
const module=await create();
for(const [input,name,stride,outStride,fn] of [
  ['cases.bin','',28,40,'_mo_elliptic_probe'],
  ['interval-cases.bin','-interval',24,48,'_mo_elliptic_interval_probe'],
]) {
  const bytes=await fs.readFile(root+input);
  const count=bytes.length/stride; assert.equal(count,Math.floor(count));
  const a=module._malloc(bytes.length),b=module._malloc(count*outStride);
  assert.ok(a&&b);
  try {
    module.HEAPU32.set(new Uint32Array(bytes.buffer,bytes.byteOffset,bytes.length/4),a/4);
    module[fn](a,b,count);
    const result=Buffer.from(module.HEAPU32.buffer,b,count*outStride);
    assert.deepEqual(result,await fs.readFile(root+'native'+name+'.bin'));
    await fs.writeFile(root+'wasm'+name+'.bin',result);
    console.log(JSON.stringify({input,count,byteIdentical:true}));
  } finally { module._free(a); module._free(b); }
}
