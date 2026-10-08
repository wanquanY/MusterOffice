/** Validate device allocation/binding safety. Geometry and color stay in Rust. */
export type ImageInfo = { id: number; width: number; height: number; hasHdr: boolean };
export type Source = { kind: 'solid' } | { kind: 'image' | 'surface'; id: number };
export type Command = { kind: 'create'; id: number; width: number; height: number }
  | { kind: 'paint'; target: number; source: Source; parameters: number[] };
export interface Program {
  profile: string; commands: Command[]; images: ImageInfo[]; output: number;
  width: number; height: number; hasHdr: boolean; allocatedBytes: number; pixelVisits: number;
}
export const PROFILE = 'musteroffice.video.gpu-plan/1-draft';
export function invalid(detail: string): never { throw new TypeError(`Invalid GPU program: ${detail}`); }
function object(v: unknown): Record<string, unknown> {
  if (!v || typeof v !== 'object' || Array.isArray(v)) invalid('object');
  return v as Record<string, unknown>;
}
function uint(v: unknown, max = 0xffffffff): number {
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0 || v > max) invalid('integer');
  return v;
}
function bool(v: unknown): boolean { if (typeof v !== 'boolean') invalid('boolean'); return v; }
export function extent(w: unknown, h: unknown): [number, number] {
  const width = uint(w, 16384), height = uint(h, 16384);
  if (!width || !height) invalid('extent'); return [width, height];
}
/** Snapshot every transport value before the first await. */
export function parseProgram(value: unknown, limits: GPUSupportedLimits): Program {
  const v = object(value);
  if (v.profile !== PROFILE) invalid('profile');
  const [width, height] = extent(v.width, v.height);
  const output = uint(v.output), hasHdr = bool(v.hasHdr);
  if (!Array.isArray(v.commands) || !v.commands.length || v.commands.length > 8192
    || !Array.isArray(v.images) || v.images.length > 64) invalid('command/image count');
  const surfaces = new Map<number, [number, number]>(), imageIds = new Map<number, [number, number]>();
  let allocatedBytes = 16, pixelVisits = 0;
  const charge = (size: number) => {
    if (size > limits.maxBufferSize || size > limits.maxStorageBufferBindingSize) invalid('device buffer extent');
    allocatedBytes += size;
    if (allocatedBytes > 512 * 2 ** 20) invalid('allocation budget');
  };
  const images = v.images.map(item => {
    const i = object(item), id = uint(i.id), [w, h] = extent(i.width, i.height);
    if (imageIds.has(id)) invalid('duplicate image');
    imageIds.set(id, [w, h]); charge(w * h * 16);
    return { id, width: w, height: h, hasHdr: bool(i.hasHdr) };
  });
  const commands: Command[] = v.commands.map(item => {
    const c = object(item);
    if (c.kind === 'create') {
      const id = uint(c.id), [w, h] = extent(c.width, c.height);
      if (surfaces.has(id)) invalid('duplicate surface');
      surfaces.set(id, [w, h]); charge(w * h * 16);
      return { kind: 'create', id, width: w, height: h };
    }
    if (c.kind !== 'paint') invalid('command');
    const target = uint(c.target), targetSize = surfaces.get(target);
    const s = object(c.source);
    let source: Source, sourceSize: [number, number] | undefined;
    if (s.kind === 'solid') { source = { kind: 'solid' }; }
    else if (s.kind === 'surface' || s.kind === 'image') {
      const id = uint(s.id); source = { kind: s.kind, id };
      sourceSize = (s.kind === 'surface' ? surfaces : imageIds).get(id);
      if (!sourceSize || (s.kind === 'surface' && id === target)) invalid('source capability');
    } else invalid('source kind');
    if (!targetSize || !Array.isArray(c.parameters) || c.parameters.length !== 28) invalid('paint ABI');
    const parameters = c.parameters.map(x => uint(x));
    if (parameters[0] !== targetSize[0] || parameters[1] !== targetSize[1]) invalid('target extent');
    if (sourceSize && (parameters[2] !== sourceSize[0] || parameters[3] !== sourceSize[1])) invalid('source extent');
    extent(parameters[2], parameters[3]);
    if (parameters[24] !== (source.kind === 'solid' ? 1 : 0) || parameters[25]! > 1
      || parameters[26] !== 0 || parameters[27] !== 0) invalid('paint options');
    if (Math.ceil(targetSize[0] / 8) > limits.maxComputeWorkgroupsPerDimension
      || Math.ceil(targetSize[1] / 8) > limits.maxComputeWorkgroupsPerDimension) invalid('dispatch extent');
    charge(112); pixelVisits += targetSize[0] * targetSize[1];
    if (pixelVisits > 100000000) invalid('work budget');
    return { kind: 'paint', target, source, parameters };
  });
  const result = surfaces.get(output);
  if (!result || result[0] !== width || result[1] !== height || allocatedBytes !== v.allocatedBytes
    || pixelVisits !== v.pixelVisits) invalid('output/accounting');
  return { profile: PROFILE, commands, images, output, width, height, hasHdr, allocatedBytes, pixelVisits };
}
