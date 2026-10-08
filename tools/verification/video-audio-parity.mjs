import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
const [nativePath, wasmPath, reportPath] = process.argv.slice(2)
assert(nativePath && wasmPath && reportPath, 'usage: video-audio-parity.mjs native wasm.js report.json')
const wasm=createRequire(import.meta.url)(resolve(wasmPath))
import { fixture } from './video-audio-fixture.mjs'
const raw=JSON.stringify(fixture)
const native=spawnSync(resolve(nativePath),['--audio'],{input:raw,encoding:'utf8',maxBuffer:8<<20})
assert.equal(native.status,0,native.stderr)
const result=JSON.parse(native.stdout)
assert.equal(result.status,'complete',result.error)
assert.deepEqual(JSON.parse(wasm.video_audio_conformance(raw)),result)
assert.deepEqual(result.results.map(r=>r.status),['mixed','mixed','mixed','mixed','mixed','rejected','rejected','rejected'])
assert.deepEqual(result.results[1].bits,result.results[0].bits.slice(123,260))
assert.deepEqual(result.results[2].bits,result.results[0].bits.slice(260,1260))
assert(result.results[4].bits.every(n=>n===0))
assert(result.results[5].error.includes('preserve-pitch'))
const sha=v=>createHash('sha256').update(v).digest('hex')
const report={format:'musteroffice.video-audio-parity/1',status:'passed',queries:8,mixed:5,rejected:3,profile:result.profile,fixtureSha256:sha(raw),resultSha256:sha(JSON.stringify(result)),nativeSha256:sha(readFileSync(resolve(nativePath))),wasmSha256:sha(readFileSync(resolve(wasmPath.replace(/\.js$/,'_bg.wasm')))),node:process.version,platform:process.platform,arch:process.arch,scope:'Actual native/WASM block PCM bits, spans, automation and nested DSP. Not browser audio device or full time-stretch quality.'}
writeFileSync(reportPath,JSON.stringify(report,null,2)+'\n')
console.log(JSON.stringify(report))
