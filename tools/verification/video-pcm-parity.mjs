import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
const [nativePath,wasmPath,reportPath]=process.argv.slice(2)
assert(nativePath&&wasmPath&&reportPath,'usage: video-pcm-parity.mjs native wasm.js report.json')
const wasm=createRequire(import.meta.url)(resolve(wasmPath))
const t=n=>({ticks:String(n),timescale:8000})
const pcm=Array.from({length:4117},(_,n)=>((n*1637)%64000-32000)/32768)
const binding={resource:'sound',digest:'13'.repeat(32),stream:'a0',descriptor:{kind:'audio',available:{start:t(-37),end:t(4080)},sampleRate:8000,channels:1}}
const queries=[{first:'-37',frames:1},{first:'219',frames:512},{first:'219',frames:513},{first:'226',frames:100},{first:'4000',frames:80},{first:'4079',frames:1},{first:'-38',frames:1}]
const sha=v=>createHash('sha256').update(v).digest('hex')
const reports=[]
for(const decodeFrames of [pcm.length,1000]) {
  const raw=JSON.stringify({binding,first:'-37',pcm,decodeFrames,queries})
  const native=spawnSync(resolve(nativePath),['--pcm'],{input:raw,encoding:'utf8',maxBuffer:8<<20})
  assert.equal(native.status,0,native.stderr)
  const result=JSON.parse(native.stdout)
  assert.equal(result.status,'complete',result.error)
  assert.deepEqual(JSON.parse(wasm.video_pcm_conformance(raw)),result)
  const complete=decodeFrames===pcm.length
  assert.equal(result.sourceComplete,complete)
  assert.equal(result.tiles.length,complete?17:3)
  assert.deepEqual(result.results.map(r=>r.status),complete?['read','read','read','read','read','read','rejected']:['read','read','rejected','read','rejected','rejected','rejected'])
  if(!complete) assert(result.results[2].error.includes('tile 3'))
  for(let i=0;i<queries.length;i++) if(result.results[i].status==='read') {
    const first=Number(queries[i].first)+37
    const expected=pcm.slice(first,first+queries[i].frames).map(v=>{const b=Buffer.alloc(4);b.writeFloatLE(v);return b.readUInt32LE()})
    assert.deepEqual(result.results[i].bits,expected)
  }
  reports.push({decodeFrames,sourceComplete:complete,sourceIdentity:result.sourceIdentity,queries:queries.length,tiles:result.tiles.length,fixtureSha256:sha(raw),resultSha256:sha(JSON.stringify(result))})
}
assert.equal(reports[0].sourceIdentity,reports[1].sourceIdentity)
const report={format:'musteroffice.video-pcm-parity/1',status:'passed',cases:reports,nativeSha256:sha(readFileSync(resolve(nativePath))),wasmSha256:sha(readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm')))),node:process.version,platform:process.platform,arch:process.arch,scope:'Actual native/WASM canonical tile identity and payload checksums, negative origin, partial materialization, random PCM bits and bounded reader. Not browser OPFS, audio device or full product.'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
