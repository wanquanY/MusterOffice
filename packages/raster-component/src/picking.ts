import type {RasterModule} from './index.js';
export type PickingReply = {status: number; words: Uint32Array};
export const MAX_PICK_FRAME_WORDS = 10+4096*2+262144*7+4096*4+8192*4+65536*5+64*3;
const MAX_PICK_REPLY_WORDS = 7+64*(2048*2+257);
/** Called under the component's single-operation guard. No output aliases its memory. */
export function pick(m: RasterModule, frame: Uint32Array, live: () => boolean, invalidate: () => void): PickingReply {
  if (!(frame instanceof Uint32Array) || !(frame.buffer instanceof ArrayBuffer) || frame.buffer === m.HEAPU8.buffer) {
    throw Error('Picking input must use an independent host ArrayBuffer');
  }
  if (frame.length < 10 || frame.length > MAX_PICK_FRAME_WORDS) return {status: 1, words: new Uint32Array()};
  const allocations: {pointer: number; bytes: number}[] = [];
  let output = 0;
  const allocate = (bytes: number): number => {
    const p = m._malloc(bytes);
    if (!live() || !Number.isSafeInteger(p) || p <= 0 || p % 4 || p + bytes > m.HEAPU8.length ||
        allocations.some(a => p < a.pointer + a.bytes && a.pointer < p + bytes)) throw Error('Picking allocation failed');
    allocations.push({pointer: p, bytes}); return p;
  };
  try {
    const input = allocate(frame.byteLength), slots = allocate(8);
    m.HEAPU32.set(frame, input / 4); m.HEAPU32.fill(0, slots / 4, slots / 4 + 2);
    const status = m._mo_skia_pick!(input, frame.length, slots, slots + 4);
    output = m.HEAPU32[slots / 4]!;
    const count = m.HEAPU32[slots / 4 + 1]!;
    if (!Number.isInteger(status) || status < 0 || status > 4 ||
        (status !== 0 && (output || count))) throw Error('Invalid picking ownership');
    if (status !== 0) {
      if (status === 2 || status === 4) invalidate();
      return {status, words: new Uint32Array()};
    }
    if (!live() || !output || output % 4 || !Number.isInteger(count) || count < 7 || count > MAX_PICK_REPLY_WORDS ||
        output + count * 4 > m.HEAPU8.length ||
        allocations.some(a => output < a.pointer + a.bytes && a.pointer < output + count * 4)) {
      throw Error('Invalid picking output');
    }
    const words = m.HEAPU32.slice(output / 4, output / 4 + count);
    m._mo_skia_free(output); output = 0;
    return {status, words};
  } catch (error) { invalidate(); throw error; }
  finally {
    if (live()) {
      try { if (output) m._mo_skia_free(output); for (const a of allocations) m._free(a.pointer); }
      catch (error) { invalidate(); throw error; }
    }
  }
}
