import { extent, invalid } from './program.js';
export interface RawInputPlan {
  readonly profile: 'musteroffice.video.raw-input/1-draft';
  readonly source: { readonly width: number; readonly height: number; readonly format: string; readonly colorSpace: VideoColorSpaceInit };
  readonly byteLength: number; readonly hasHdr: boolean;
  readonly planes: readonly PlaneLayout[]; readonly parameters: readonly number[];
}
/** Validate transport against the actual immutable frame before any allocation.
 * Pixel layouts and color interpretation are computed by Rust, never here. */
export function rawInputPlan(value: unknown, frame: VideoFrame): RawInputPlan {
  if (!value || typeof value !== 'object') invalid('raw input plan');
  const p = structuredClone(value) as RawInputPlan;
  if (p.profile !== 'musteroffice.video.raw-input/1-draft' || !p.source || !p.source.colorSpace || !frame.visibleRect) invalid('raw input profile');
  const [w,h] = extent(p.source.width,p.source.height);
  if (frame.visibleRect.width !== w || frame.visibleRect.height !== h || p.source.format !== frame.format) invalid('raw frame identity');
  for (const key of ['primaries', 'transfer', 'matrix', 'fullRange'] as const) {
    if (p.source.colorSpace[key] !== frame.colorSpace[key] || p.source.colorSpace[key] == null) invalid('raw frame color identity');
  }
  if (typeof p.hasHdr !== 'boolean' || !Number.isSafeInteger(p.byteLength) || p.byteLength <= 0 || p.byteLength % 4 !== 0 || p.byteLength > 256 * 2 ** 20
    || !Array.isArray(p.parameters) || p.parameters.length !== 56 || p.parameters.some(v => !Number.isInteger(v) || v < 0 || v > 0xffffffff)) invalid('raw input ABI');
  if (p.parameters[0] !== w || p.parameters[1] !== h || ![0,1].includes(p.parameters[2]!) || ![8,10,12].includes(p.parameters[3]!)
    || ![1,2].includes(p.parameters[4]!) || ![1,2].includes(p.parameters[5]!) || ![1,2].includes(p.parameters[6]!) || ![0,1,256,257].includes(p.parameters[7]!)) invalid('raw input extent/layout ABI');
  if (!Array.isArray(p.planes) || p.planes.length < 1 || p.planes.length > 4) invalid('raw plane layout');
  for (const [i,plane] of p.planes.entries()) {
    if (!plane || !Number.isInteger(plane.offset) || plane.offset < 0 || plane.offset >= p.byteLength || !Number.isInteger(plane.stride)
      || plane.stride <= 0 || plane.stride > p.byteLength || plane.offset !== p.parameters[8+i*4] || plane.stride !== p.parameters[9+i*4]) invalid('raw plane ABI');
  }
  const bytes = new ArrayBuffer(224), view = new DataView(bytes);
  p.parameters.forEach((v,i) => view.setUint32(i*4,v,true));
  for(let i=28;i<56;i++) if(!Number.isFinite(view.getFloat32(i*4,true))) invalid('raw input coefficient');
  const n = frame.allocationSize({layout: p.planes as PlaneLayout[]});
  if (n > p.byteLength || n <= 0) invalid('raw copy extent');
  return p;
}
