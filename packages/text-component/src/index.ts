/** Thin adapter for the separately instantiated pinned C++ WASM module. */
export interface ComponentModule {
  HEAPU8: Uint8Array;
  HEAPU32: Uint32Array;
  _malloc(bytes: number): number;
  _free(pointer: number): void;
  _mo_hb_outline_font(font: number, fontBytes: number, request: number, words: number, output: number, outputWords: number): number;
  _mo_hb_measure_font(font: number, fontBytes: number, request: number, words: number, output: number, outputWords: number): number;
  _mo_hb_free(pointer: number): void;
  _mo_hb_version(): number;
  _mo_hb_shape(font: number, fontBytes: number, request: number, words: number,
    language: number, languageBytes: number, output: number, outputWords: number): number;
}
export type ComponentFactory = (options: {
  instantiateWasm(imports: WebAssembly.Imports, receive: (instance: WebAssembly.Instance, module: WebAssembly.Module) => void): WebAssembly.Exports;
}) => Promise<ComponentModule>;
const profiles = {
  shape: {magic:0x4d4f5342, count:256, request:2200000, min:13, max:2200000, output:8, reply:2+256*9+262144*7},
  metrics: {magic:0x4d4f4d42, count:256, request:41732, min:6, max:162, output:6, reply:2+256*(1+6+28*3)},
  outlines: {magic:0x4d4f4f42, count:64, request:4+64*137+4096, min:8, max:392, output:6, reply:2+64*7+4096*3+262144*7},
} as const;
type Operation = keyof typeof profiles;

/** Host supplies already verified code; this class does no fetch or path access. */
export class ShapingComponent {
  #module: ComponentModule | undefined;
  #busy = false;
  #fonts = new Map<number, {pointer:number; bytes:number}>();
  #fontBytes = 0;
  #nextFont = 0;
  private constructor(module: ComponentModule) { this.#module = module; }
  static async create(factory: ComponentFactory, compiled: WebAssembly.Module): Promise<ShapingComponent> {
    for (const item of WebAssembly.Module.imports(compiled)) {
      const memory = item.module === "env" && item.name === "emscripten_resize_heap";
      const denied = item.module === "wasi_snapshot_preview1" && ["fd_close", "fd_write", "fd_seek"].includes(item.name);
      if (item.kind !== "function" || (!memory && !denied)) throw new Error("Unexpected shaping component import");
    }
    const module = await factory({ instantiateWasm(imports, receive) {
      const wasi = imports.wasi_snapshot_preview1;
      for (const name of ["fd_close", "fd_write", "fd_seek"]) if (wasi?.[name]) {
        wasi[name] = () => { throw new Error("Shaping component I/O denied"); };
      }
      const instance = new WebAssembly.Instance(compiled, imports); receive(instance, compiled); return instance.exports;
    }});
    if (module._mo_hb_version() !== 0x0e0500) throw new Error("Shaping component version mismatch");
    return new ShapingComponent(module);
  }
  get invalid(): boolean { return !this.#module; }
  invalidate(): void { this.#fonts.clear(); this.#fontBytes=0; this.#module = undefined; }

  registerFont(font: Uint8Array): number {
    const m=this.#module;
    if(!m || this.#busy) throw new Error("Shaping instance unavailable");
    let pointer=0;
    try {
      if(!font.length || font.length>128*1024*1024 || this.#fontBytes+font.length>128*1024*1024 || this.#fonts.size>=32 || this.#nextFont>=0xffffffff)
        throw new Error("Resident font budget exceeded");
      pointer=m._malloc(font.length);
      if(!pointer) throw new Error("Resident font allocation failed");
      m.HEAPU8.set(font,pointer);
      const handle=++this.#nextFont;
      this.#fonts.set(handle,{pointer,bytes:font.length});this.#fontBytes+=font.length;
      return handle;
    } catch(error) {
      try { if(pointer) m._free(pointer); } finally { this.invalidate(); }
      throw error;
    }
  }
  unregisterFont(handle: number): void {
    const m=this.#module, font=this.#fonts.get(handle);
    if(!m || this.#busy || !font) throw new Error("Unknown resident font");
    try { m._free(font.pointer); this.#fonts.delete(handle); this.#fontBytes-=font.bytes; }
    catch(error) { this.invalidate(); throw error; }
  }
  shapeRegistered(handle:number,frame:Uint32Array):Uint32Array {return this.#batch(handle,frame,"shape");}
  measureRegistered(handle:number,frame:Uint32Array):Uint32Array {return this.#batch(handle,frame,"metrics");}
  outlineRegistered(handle:number,frame:Uint32Array):Uint32Array {return this.#batch(handle,frame,"outlines");}

  shapeBatch(font: Uint8Array, frame: Uint32Array): Uint32Array {return this.#batch(font,frame,"shape");}
  measureBatch(font: Uint8Array, frame: Uint32Array): Uint32Array {return this.#batch(font,frame,"metrics");}
  outlineBatch(font: Uint8Array, frame: Uint32Array): Uint32Array {return this.#batch(font,frame,"outlines");}
  #batch(font: Uint8Array | number, frame: Uint32Array, operation: Operation): Uint32Array {
    const profile = profiles[operation], maximum = profile.reply, shaping = operation === "shape";
    const m = this.#module;
    if (!m || this.#busy) throw new Error("Shaping instance unavailable");
    this.#busy = true;
    const allocations: number[] = [];
    let output = 0;
    const allocate = (bytes: number): number => {
      const pointer = m._malloc(Math.max(bytes, 1));
      if (!pointer) throw new Error("Shaping input allocation failed");
      allocations.push(pointer); return pointer;
    };
    try {
      const resident=typeof font==="number"?this.#fonts.get(font):undefined;
      const fontLength=typeof font==="number"?(resident?.bytes??0):font.length;
      if (!fontLength || fontLength > 128 * 1024 * 1024 || frame.length < 4 || frame.length > profile.request ||
          frame[0] !== profile.magic || frame[1] !== 1 || frame[2] !== 0x0e0500 || frame[3]! > profile.count) {
        throw new Error("Invalid shaping transport frame");
      }
      const fontPointer = resident?.pointer ?? allocate(fontLength);
      if(typeof font!=="number") m.HEAPU8.set(font, fontPointer);
      const slots = allocate(8), requestPointer = allocate(frame.length * 4), languagePointer = shaping?allocate(256):0;
      // Getter accesses below always retrieve the current view after memory growth.
      const results: Uint32Array[] = [];
      let offset = 4, replyWords = 2;
      for (let run = 0; run < frame[3]!; run++) {
        if (offset + (shaping?2:1) > frame.length) throw new Error("Truncated shaping transport");
        const languageLength = shaping?frame[offset++]!:0, requestLength = frame[offset++]!;
        if ((shaping && (languageLength < 1 || languageLength > 255)) || requestLength < profile.min || requestLength>profile.max || offset + languageLength + requestLength > frame.length) {
          throw new Error("Invalid shaping transport lengths");
        }
        for (let i = 0; i < languageLength; i++) {
          const byte = frame[offset++]!;
          if (!(byte === 45 || (byte >= 48 && byte <= 57) || (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122))) {
            throw new Error("Invalid shaping language bytes");
          }
          m.HEAPU8[languagePointer + i] = byte;
        }
        m.HEAPU32.set(frame.subarray(offset, offset + requestLength), requestPointer / 4); offset += requestLength;
        m.HEAPU32.fill(0, slots / 4, slots / 4 + 2);
        const status = operation === "metrics" ? m._mo_hb_measure_font(fontPointer,fontLength,requestPointer,requestLength,slots,slots+4) :
          operation === "outlines" ? m._mo_hb_outline_font(fontPointer,fontLength,requestPointer,requestLength,slots,slots+4) :
          m._mo_hb_shape(fontPointer, fontLength, requestPointer, requestLength, languagePointer, languageLength, slots, slots + 4);
        output = m.HEAPU32[slots / 4]!;
        const count = m.HEAPU32[slots / 4 + 1]!;
        if (!Number.isInteger(status) || status < 0 || status > 6 || (status !== 0 && (output !== 0 || count !== 0))) {
          throw new Error("Invalid shaping status or ownership");
        }
        if (status !== 0) {
          if (status === 2 || status === 6) this.invalidate();
          return Uint32Array.of(status, run);
        }
        if (!output || output % 4 || count < profile.output || count > maximum || output / 4 + count > m.HEAPU32.length) {
          throw new Error("Invalid shaping result allocation");
        }
        if (replyWords + count + 1 > maximum) return Uint32Array.of(4, run);
        const words = m.HEAPU32.slice(output / 4, output / 4 + count);
        m._mo_hb_free(output); output = 0;
        results.push(words); replyWords += 1 + count;
      }
      if (offset !== frame.length) throw new Error("Trailing shaping request words");
      const reply = new Uint32Array(replyWords); reply.set([0, results.length]); offset = 2;
      for (const words of results) { reply[offset++] = words.length; reply.set(words, offset); offset += words.length; }
      return reply;
    } catch (error) { this.invalidate(); throw error; }
    finally {
      try { if (output) m._mo_hb_free(output); for (const pointer of allocations) m._free(pointer); }
      catch (error) { this.invalidate(); throw error; }
      finally { this.#busy = false; }
    }
  }
}
