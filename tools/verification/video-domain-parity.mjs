import assert from 'node:assert/strict'
import { readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { resolve } from 'node:path'

const [nativePath, wasmPath, reportPath] = process.argv.slice(2)
assert(nativePath && wasmPath && reportPath, 'usage: node video-domain-parity.mjs native wasm.js report.json')
const wasm = createRequire(import.meta.url)(resolve(wasmPath))
const t = (ticks, timescale = 1) => ({ ticks: String(ticks), timescale })
const range = (start, end) => ({ start: t(start), end: t(end) })
const map = (start, end, duration) => ({ anchors: [{ local: t(0), source: t(start) }, { local: t(duration), source: t(end) }] })
const clip = (id, start, end) => ({ id, name: id, timeline: range(start, end), source: {
  kind: 'media', resource: 'movie', stream: 'v0', timeMap: map(start + 20, end + 20, end - start),
}, effects: [] })
const track = (id, clips) => ({ id, name: id, kind: 'video', clips, muted: false, solo: false, locked: false })
const document = {
  format: 'musteroffice.video/1', id: 'film', title: '', canvas: { width: 1920, height: 1080 },
  frameRate: { numerator: 30000, denominator: 1001 }, audioSampleRate: 48000, rootSequence: 'main',
  sequences: { main: { id: 'main', tracks: ['video', 'overlay'] } },
  tracks: { video: track('video', ['a', 'b', 'c']), overlay: track('overlay', ['x']) },
  clips: { a: clip('a', 0, 10), b: clip('b', 10, 20), c: clip('c', 20, 30), x: clip('x', 0, 10) },
  resources: { movie: { id: 'movie', digest: '07'.repeat(32), byteLength: '1000', descriptor: {
    kind: 'media', streams: { v0: { kind: 'video', available: range(-5, 100), dimensions: { width: 1920, height: 1080 }, frameRate: null } },
  } } }, linkGroups: {},
}
const independent = { linkedPolicy: 'independent' }
const requireAll = { linkedPolicy: 'requireAll' }
const fixture = { document, transactions: [
  [{ type: 'setTitle', title: '中文 · Agent video' }],
  [{ type: 'move', clips: ['a'], delta: t(1), targetTrack: null, ...requireAll }], // overlap failure
  [{ type: 'setEffects', clip: 'a', effects: [{ kind: 'fade', fadeIn: t(4), fadeOut: t(3), envelope: range(0,10) }] }],
  [{ type: 'split', targets: [{ clip: 'a', right: 'a2' }], at: t(1001,30000), rightGroups: [], ...requireAll }],
  [{ type: 'trim', clips: ['b'], edge: 'end', delta: t(-1), rippleTracks: ['video'], ...requireAll }],
  [{ type: 'slip', clips: ['x'], delta: t(-2), ...requireAll }],
  [{ type: 'link', group: { id: 'sync', members: ['a','x'] } }],
  [{ type: 'move', clips: ['x'], delta: t(1), targetTrack: null, ...requireAll }], // incomplete link selection
  [{ type: 'move', clips: ['x'], delta: t(1), targetTrack: null, ...independent }],
  [{ type: 'unlink', group: 'sync' }],
  [{ type: 'roll', left: 'b', right: 'c', delta: t(1), ...requireAll }],
  [{ type: 'slide', left: 'a2', clip: 'b', right: 'c', delta: t(1), ...requireAll }],
  [{ type: 'delete', clips: ['a2'], ...requireAll }],
  [{ type: 'insert', track: 'video', clip: clip('new',5,6), rippleTracks: [] }],
  [{ type: 'removeResource', resource: 'movie' }],
  [{ type: 'replaceSource', clip: 'x', source: { kind: 'media', resource: 'movie', stream: 'v0', timeMap: map(40,20,10) } }],
  [{ type: 'split', targets: [{ clip: 'x', right: 'x2' }], at: t(4), rightGroups: [], ...requireAll }],
  [{ type: 'setTrack', track: 'overlay', name: 'Locked', locked: true, muted: false, solo: true }],
  [{ type: 'delete', clips: ['x2'], ...requireAll }],
] }
const raw = JSON.stringify(fixture)
const native = spawnSync(resolve(nativePath), [], { input: raw, encoding: 'utf8', maxBuffer: 32*1024*1024 })
assert.equal(native.status, 0, native.stderr)
const nativeResult = JSON.parse(native.stdout)
assert.equal(nativeResult.status, 'complete', nativeResult.error)
const wasmResult = JSON.parse(wasm.video_conformance(raw))
assert.deepEqual(wasmResult, nativeResult)
const prepared = nativeResult.steps.filter(s => s.status === 'prepared').length
const rejected = nativeResult.steps.length - prepared
assert(prepared >= 12 && rejected >= 4)
// The duplicate-key path must also match across the strict JSON boundary.
const malformed = '{"document":null,"document":null,"transactions":[]}'
const invalidNative = spawnSync(resolve(nativePath), [], { input: malformed, encoding:'utf8' })
assert.equal(invalidNative.status, 0)
assert.equal(JSON.parse(invalidNative.stdout).status, 'invalidFixture')
assert.deepEqual(JSON.parse(wasm.video_conformance(malformed)),JSON.parse(invalidNative.stdout))
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')
const report = { format:'musteroffice.video-domain-conformance/1', status:'pass', node:process.version,
  platform:process.platform, arch:process.arch, fixtureSha256:sha256(raw), resultSha256:sha256(native.stdout),
  steps:fixture.transactions.length, prepared, rejected, duplicateKeyRejected:true,
  nativeSha256:sha256(readFileSync(resolve(nativePath))), wasmBindingSha256:sha256(readFileSync(resolve(wasmPath))),
  scope:'Native/WASM exact document, diagnostics, read/write sets, invalidations and conditional inverse parity. Test-only binding; no media/render, durable host or product acceptance.' }
writeFileSync(reportPath, `${JSON.stringify(report,null,2)}\n`)
console.log(JSON.stringify(report))
