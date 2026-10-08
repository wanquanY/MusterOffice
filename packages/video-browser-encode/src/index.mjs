/** Thin transport around the shared native/Wasm AAC + exact MP4 leaf.
 * Host callbacks are synchronous; data views are borrowed only for the call.
 * No output store, global filename, authority, clock conversion or codec policy.
 */
import createModule from './runtime/encode.mjs';

const configFields = [
  ['profile', 'i32'], ['width', 'i32'], ['height', 'i32'], ['videoTimeScale', 'i32'],
  ['frameRateNumerator', 'i32'], ['frameRateDenominator', 'i32'], ['videoDuration', 'i64'],
  ['videoFrames', 'i64'], ['videoBitRate', 'i64'], ['audioSampleRate', 'i32'],
  ['audioChannels', 'i32'], ['audioBitRate', 'i32'], ['gopFrames', 'i32'], ['audioSamples', 'i64'],
];
const limitFields = [
  ['maxFileBytes', 'u64'], ['maxWriteBytes', 'u64'], ['maxPendingBytes', 'u64'], ['maxFrameBytes', 'u64'],
  ['maxPixels', 'u32'], ['maxPackets', 'u32'], ['maxPacketBytes', 'u32'], ['maxPendingPackets', 'u32'],
  ['maxSteps', 'u32'], ['maxWriteCalls', 'u32'],
];
const statFields = ['byteLength', 'writeBytes', 'videoFrames', 'audioSamples', 'videoPackets', 'audioPackets', 'peakPendingBytes'];
const integer = (value, signed) => {
  if (typeof value === 'number' && !Number.isSafeInteger(value)) throw Error('unsafe integer');
  if (!['string', 'bigint', 'number'].includes(typeof value) || !/^-?\d+$/.test(String(value))) throw Error('integer required');
  const n = BigInt(value), min = signed ? -(1n << 63n) : 0n, max = signed ? (1n << 63n) - 1n : (1n << 64n) - 1n;
  if (n < min || n > max) throw Error('integer outside ABI range');
  return n;
};

export async function createEncodeRuntime(wasmModule) {
  if (!(wasmModule instanceof WebAssembly.Module)) throw Error('explicit verified WebAssembly.Module required');
  const hosts = new Map();
  const module = await createModule({
    instantiateWasm(imports, ready) {
      const instance = new WebAssembly.Instance(wasmModule, imports); ready(instance, wasmModule); return instance.exports;
    },
    moWriteAt(id, at, data) {
      const result = hosts.get(id).sink.writeAt(at, data);
      if (!Number.isInteger(result) || result < 1 || result > data.length) throw Error('invalid output write count');
      return result;
    },
    moCancelled(id) { return hosts.get(id).sink.cancelled(); },
    moCallbackError(id, error) { hosts.get(id).error ??= error; },
    print() {}, printErr() {},
  });
  const heap = () => module.HEAPU8;
  const view = () => new DataView(heap().buffer);
  const table = Array.from(new Uint32Array(heap().buffer, module._mo_browser_layout(), 39));
  if (table[0] !== 1) throw Error('browser encoder ABI mismatch');
  const configOffsets = table.slice(2, 16), limitOffsets = table.slice(17, 27), statOffsets = table.slice(28, 35);
  const errorOffsets = table.slice(36, 39);
  const fill = (pointer, fields, offsets, values) => {
    const v = view();
    fields.forEach(([name, type], i) => {
      const at = pointer + offsets[i], value = values[name];
      if (type.endsWith('64')) {
        const n = integer(value, type === 'i64');
        if (type === 'i64') v.setBigInt64(at, n, true); else v.setBigUint64(at, n, true);
      } else {
        if (!Number.isInteger(value) || value < (type === 'u32' ? 0 : -2147483648) || value > (type === 'u32' ? 4294967295 : 2147483647)) throw Error('invalid ' + name);
        if (type === 'i32') v.setInt32(at, value, true); else v.setUint32(at, value, true);
      }
    });
  };

  return Object.freeze({
    open(config, limits, sink, description) {
      if (!sink || typeof sink.writeAt !== 'function' || typeof sink.cancelled !== 'function') throw Error('seekable output and cancellation required');
      const handle = module._mo_browser_create();
      if (!handle) throw Error('browser encoder allocation');
      const host = { sink, error: undefined }; hosts.set(handle, host);
      let closed = false, busy = false, finished = false;
      const close = () => {
        if (busy) throw Error('reentrant encoder call');
        if (!closed) { closed = true; module._mo_browser_close(handle); hosts.delete(handle); }
      };
      const check = status => {
        if (status >= 0 && !host.error) return;
        const pointer = module._mo_browser_error(handle), v = view();
        const bytes = heap().subarray(pointer + errorOffsets[2], pointer + errorOffsets[2] + 192);
        const message = new TextDecoder().decode(bytes.subarray(0, Math.max(0, bytes.indexOf(0))));
        const error = new Error(message || 'browser encoding failed', host.error ? { cause: host.error } : undefined);
        error.status = status; error.upstreamCode = v.getInt32(pointer + errorOffsets[1], true); throw error;
      };
      const invoke = action => {
        if (busy) throw Error('reentrant encoder call');
        if (closed || finished) throw Error('closed or finished encoder');
        busy = true;
        try { return action(); }
        catch (error) { busy = false; close(); throw error; }
        finally { busy = false; }
      };
      const copy = (data, maximum, use) => {
        if (!(data instanceof Uint8Array) || !data.byteLength || data.byteLength > maximum) throw Error('input byte budget');
        const pointer = module._malloc(data.byteLength);
        if (!pointer) throw Error('input allocation');
        try { heap().set(data, pointer); return use(pointer, data.byteLength); }
        finally { module._free(pointer); }
      };
      try {
        fill(module._mo_browser_config(handle), configFields, configOffsets, { ...config, profile: 4 });
        fill(module._mo_browser_limits(handle), limitFields, limitOffsets, limits);
        invoke(() => copy(description, 1048576, (p, n) => check(module._mo_browser_open(handle, p, n))));
      } catch (error) { close(); throw error; }
      // The C session owns immutable copies; mutation of caller objects cannot
      // change admission or channel layout after open.
      const channels = config.audioChannels, packetLimit = limits.maxPacketBytes;
      return Object.freeze({
        video(pts, duration, data, key) {
          return invoke(() => {
            if (typeof key !== 'boolean') throw Error('key flag required');
            copy(data, packetLimit, (p, n) => check(module._mo_browser_video(handle, integer(pts, true), integer(duration, true), p, n, Number(key))));
          });
        },
        audio(firstSample, samples) {
          return invoke(() => {
            if (!(samples instanceof Float32Array) || !channels || samples.length % channels) throw Error('interleaved Float32 PCM required');
            copy(new Uint8Array(samples.buffer, samples.byteOffset, samples.byteLength), 65536 * channels * 4,
              p => check(module._mo_browser_audio(handle, integer(firstSample, true), p, samples.length / channels)));
          });
        },
        finish() {
          return invoke(() => {
            check(module._mo_browser_finish(handle)); finished = true;
            const p = module._mo_browser_stats(handle), v = view();
            return Object.fromEntries(statFields.map((name, i) => [name, v.getBigUint64(p + statOffsets[i], true).toString()]));
          });
        },
        close,
      });
    },
  });
}
