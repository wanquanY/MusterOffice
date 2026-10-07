import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { createHash } from 'node:crypto'
const [nativePath, wasmPath, reportPath] = process.argv.slice(2)
assert(nativePath && wasmPath && reportPath, 'usage: video-viewport-parity.mjs native wasm.js report.json')
const wasm = createRequire(import.meta.url)(resolve(wasmPath))
const time = { time: { numerator: '1001', denominator: '30000' }, side: 'before' }
const solid = (clip, rgba) => ({ track: 'video', clip, local: time, effects: [], source: { kind: 'solid', rgba } })
const transform = (x, y, scale, angle) => ({ kind: 'transform', xMilliPixels: x, yMilliPixels: y, scaleXPpm: scale, scaleYPpm: scale, rotationMilliDegrees: angle, opacityPpm: 1000000 })
const red = solid('red', [255, 0, 0, 255])
red.effects.push(transform(50000, 0, 500000, 0))
const group = { track: 'video', clip: 'group', local: time, effects: [transform(-40000, 0, 1000000, 0), { kind: 'opacity', factor: { numerator: '1', denominator: '2' } }], source: { kind: 'group', sequence: 'child', mapping: { sample: time, rate: { numerator: '1', denominator: '1' } }, layers: [red] } }
const plan = { documentDigest: '11'.repeat(32), frameIndex: '1', sample: time, canvas: { width: 120, height: 80 }, layers: [solid('blue', [0, 0, 255, 255]), group] }
const grids = [[12, 8], [24, 16], [12, 16]]
const viewports = grids.map(([width, height]) => ({ plan, width, height }))
// Non-integral scaling/rotation tests the same Q32 coverage and f32 values.
const rotated = structuredClone(plan)
rotated.layers[1].effects = [transform(21500, -7300, 750000, 37000)]
viewports.push({ plan: rotated, width: 17, height: 13 })
viewports.push({ plan, width: 0, height: 8 }, { plan, width: 12, height: 16385 })
const raw = JSON.stringify({ frames: [], plans: [], viewports })
const native = spawnSync(resolve(nativePath), ['--render'], { input: raw, encoding: 'utf8', maxBuffer: 4 << 20 })
assert.equal(native.status, 0, native.stderr)
const result = JSON.parse(native.stdout)
assert.equal(result.status, 'complete', result.error)
assert.deepEqual(JSON.parse(wasm.video_render_conformance(raw)), result)
assert.deepEqual(result.results.map(r => r.status), ['rendered', 'rendered', 'rendered', 'rendered', 'rejected', 'rejected'])
for (let i = 0; i < grids.length; i++) {
  const [width, height] = grids[i]
  const actual = result.results[i].value.rgba
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const px = (2 * x + 1) * 120, py = (2 * y + 1) * 80
    const inside = px >= 80 * width && px < 160 * width && py >= 40 * height && py < 120 * height
    const at = (y * width + x) * 4
    assert.deepEqual(actual.slice(at, at + 4), inside ? [188, 0, 188, 255] : [0, 0, 255, 255])
  }
}
const sha = value => createHash('sha256').update(value).digest('hex')
const report = { format: 'musteroffice.video-viewport-parity/1', status: 'passed', queries: 6, rendered: 4, rejected: 2, fixtureSha256: sha(raw), resultSha256: sha(JSON.stringify(result)), nativeSha256: sha(readFileSync(resolve(nativePath))), wasmSha256: sha(readFileSync(resolve(wasmPath.replace(/\.js$/, '_bg.wasm')))), scope: 'Actual native/WASM CPU raster outputs, identical float bits and diagnostics; independently checked clipped project-space bounds. Not browser media or product acceptance.' }
writeFileSync(reportPath, JSON.stringify(report, null, 2) + '\n')
console.log(JSON.stringify(report))
