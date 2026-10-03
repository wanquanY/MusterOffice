import assert from 'node:assert/strict';
import test from 'node:test';
import {RasterComponent} from '../../.codex-work/raster-component/index.js';

const module = new WebAssembly.Module(Uint8Array.of(0,97,115,109,1,0,0,0));
const frame = () => Uint32Array.of(0x4d4f504b,1,10,10,0,0,0,0,0,1,0,0,0);
const reply = [0x4d4f504b,1,1,0,0,0,1,1];
async function fixture(change = () => {}) {
  let memory = new ArrayBuffer(65536), cursor = 64;
  const events = [];
  const m = {
    HEAPU8: new Uint8Array(memory), HEAPU32: new Uint32Array(memory),
    _mo_skia_abi: () => 4, _mo_skia_pick_abi: () => 1,
    _malloc(bytes) { const p = cursor; cursor += bytes + 16; events.push(['alloc',p]); return p; },
    _free(p) { events.push(['free',p]); },
    _mo_skia_free(p) { events.push(['release',p]); },
    _mo_skia_pick(input, count, output, size) {
      events.push(['pick',input,count]);
      m.HEAPU32.set(reply,1024);
      m.HEAPU32[output/4] = 4096; m.HEAPU32[size/4] = reply.length;
      return 0;
    },
    _mo_skia_raster() { throw Error('unexpected rasterization'); },
    grow() {
      const next = new ArrayBuffer(memory.byteLength * 2);
      new Uint8Array(next).set(m.HEAPU8);
      memory = next; m.HEAPU8 = new Uint8Array(memory); m.HEAPU32 = new Uint32Array(memory);
    },
  };
  change(m, events);
  const component = await RasterComponent.create(async () => m, module);
  return {m, component, events};
}
test('owned result survives subsequent component memory changes and frees exactly once', async () => {
  const {component,m,events} = await fixture();
  const r = component.pick(frame());
  assert.equal(r.status,0); assert.deepEqual([...r.words],reply);
  m.HEAPU32.fill(0); assert.deepEqual([...r.words],reply);
  assert.notEqual(r.words.buffer,m.HEAPU8.buffer);
  assert.equal(events.filter(e => e[0] === 'free').length,2);
  assert.deepEqual(events.filter(e => e[0] === 'release'),[['release',4096]]);
  assert.equal(component.invalid,false);
});
test('fresh heap views are used after allocator and component memory growth', async () => {
  const {component} = await fixture(m => {
    const malloc=m._malloc, pick=m._mo_skia_pick;
    m._malloc = n => { m.grow(); return malloc(n); };
    m._mo_skia_pick = (...args) => { m.grow(); return pick(...args); };
  });
  assert.deepEqual([...component.pick(frame()).words],reply);
});
test('missing capability and invalid host buffers do not poison a live component', async () => {
  const old = await fixture(m => { delete m._mo_skia_pick; });
  assert.equal(old.component.supportsPicking,false);
  assert.throws(() => old.component.pick(frame()),/unavailable/);
  assert.equal(old.component.invalid,false); assert.deepEqual(old.events,[]);
  const {component,m,events} = await fixture();
  for (const input of [m.HEAPU32.subarray(0,13),new Uint32Array(new SharedArrayBuffer(52))]) {
    assert.throws(() => component.pick(input),/independent host/);
  }
  assert.deepEqual(component.pick(new Uint32Array()),{status:1,words:new Uint32Array()});
  assert.equal(component.invalid,false); assert.deepEqual(events,[]);
});
test('status 1 and 3 release inputs; allocation or component faults quarantine the instance', async () => {
  for (const status of [1,2,3,4]) {
    const {component,events} = await fixture(m => { m._mo_skia_pick = () => status; });
    assert.deepEqual(component.pick(frame()),{status,words:new Uint32Array()});
    assert.equal(component.invalid,status === 2 || status === 4);
    assert.equal(events.filter(e => e[0] === 'free').length,component.invalid?0:2);
  }
});
test('malformed ownership, aliases and traps never free untrusted pointers', async () => {
  const faults = [
    m => { m._malloc = () => 0; },
    m => { m._malloc = () => -4; },
    m => { m._malloc = () => 64; },
    m => { m._mo_skia_pick_abi = () => { throw Error('capability trap'); }; },
    m => { m._mo_skia_pick = () => { throw Error('component trap'); }; },
    ...[
      [-1,4096,5], [5,4096,5], [1,4096,5], [0,0,5], [0,4097,5],
      [0,65532,5], [0,4096,0], [0,4096,300000], [0,'input',5], [0,'slots',5],
    ].map(([status,p,n]) => m => {
      m._mo_skia_pick = (input,count,output,size) => {
        m.HEAPU32[output/4] = p === 'input'?input:p === 'slots'?output:p;
        m.HEAPU32[size/4] = n; return status;
      };
    }),
  ];
  for (const fault of faults) {
    const {component,events} = await fixture(fault);
    assert.throws(() => component.pick(frame())); assert.equal(component.invalid,true);
    assert.deepEqual(events.filter(e => ['free','release'].includes(e[0])),[]);
    assert.throws(() => component.pick(frame()),/unavailable/);
  }
});
test('reentrancy is refused without disturbing the active operation', async () => {
  const {component,m} = await fixture();
  const pick=m._mo_skia_pick;
  m._mo_skia_pick = (...args) => {
    assert.throws(() => component.pick(frame()),/unavailable/);
    assert.throws(() => component.raster(frame()),/unavailable/);
    return pick(...args);
  };
  assert.deepEqual([...component.pick(frame()).words],reply);
  assert.equal(component.invalid,false);
});
test('release failures and host invalidation during a call cannot return success', async () => {
  for (const at of ['release','free','during']) {
    const {component,m} = await fixture();
    if (at === 'release') m._mo_skia_free = () => { throw Error('release trap'); };
    if (at === 'free') m._free = () => { throw Error('free trap'); };
    if (at === 'during') {
      const pick=m._mo_skia_pick;
      m._mo_skia_pick = (...args) => { component.invalidate(); return pick(...args); };
    }
    assert.throws(() => component.pick(frame())); assert.equal(component.invalid,true);
  }
});
