import { colorWords } from './color.js';
import { invalid } from './program.js';

/** Caller-owned bytes with explicit I420 plane layout. No codec/canvas conversion. */
export interface I420Readback {
  readonly width: number; readonly height: number; readonly data: Uint8Array<ArrayBuffer>;
  readonly layout: readonly [Readonly<PlaneLayout>, Readonly<PlaneLayout>, Readonly<PlaneLayout>];
}
export type I420Kernels = readonly [GPUComputePipeline, GPUComputePipeline, GPUComputePipeline];
export function i420Words(value: unknown, width: number, height: number, hdr: boolean) {
  if (!Array.isArray(value) || value.length !== 44 || value.some(x => !Number.isInteger(x) || x < 0 || x > 0xffffffff)) invalid('I420 ABI');
  const words = [...value] as number[];
  colorWords(words.slice(0, 20), width, height, false, hdr);
  if (width < 2 || height < 2 || width > 8192 || height > 8192 || width % 2 || height % 2) invalid('I420 extent');
  const bytes = new ArrayBuffer(176), view = new DataView(bytes);
  words.forEach((word, index) => view.setUint32(index * 4, word, true));
  for (let i = 20; i < 40; i++) if (!Number.isFinite(view.getFloat32(i * 4, true))) invalid('I420 coefficients');
  const [yStride, cStride, uOffset, vOffset] = words.slice(40) as [number, number, number, number];
  if (words[23] !== 0 || yStride !== Math.ceil(width / 4) * 4 || cStride !== Math.ceil(width / 8) * 4
    || uOffset !== yStride * height || vOffset !== uOffset + cStride * height / 2) invalid('I420 plane layout');
  const layout = [{ offset: 0, stride: yStride }, { offset: uOffset, stride: cStride }, { offset: vOffset, stride: cStride }] as const;
  return { words, layout, bytes: vOffset + cStride * height / 2 };
}
export async function i420Kernels(device: GPUDevice, shader: string): Promise<I420Kernels> {
  if (typeof shader !== 'string' || shader.length > 65536) invalid('I420 shader');
  const module = device.createShaderModule({ code: shader });
  const errors = (await module.getCompilationInfo()).messages.filter(message => message.type === 'error');
  if (errors.length) throw new Error('I420 shader compilation failed: ' + errors.map(e => `${e.lineNum}:${e.linePos} ${e.message}`).join('; '));
  // Serial compilation preserves the executor's device error-scope lifetime.
  const kernel = (entryPoint: string) => device.createComputePipelineAsync({ layout: 'auto', compute: { module, entryPoint } });
  return [await kernel('television_signal'), await kernel('chroma_horizontal'), await kernel('pack_i420')];
}
export function encodeI420(device: GPUDevice, encoder: GPUCommandEncoder, kernels: I420Kernels,
  buffers: readonly GPUBuffer[], width: number, height: number): void {
  const bindings = [[0, 1, 2], [0, 2, 3], [0, 2, 3, 4]];
  const sizes = [[width, height], [width / 2, height], [Math.ceil(width / 4), height * 2]];
  for (let i = 0; i < 3; i++) {
    const kernel = kernels[i]!;
    const group = device.createBindGroup({ layout: kernel.getBindGroupLayout(0),
      entries: bindings[i]!.map(binding => ({ binding, resource: { buffer: buffers[binding]! } })) });
    const pass = encoder.beginComputePass(); pass.setPipeline(kernel); pass.setBindGroup(0, group);
    pass.dispatchWorkgroups(Math.ceil(sizes[i]![0]! / 8), Math.ceil(sizes[i]![1]! / 8)); pass.end();
  }
}
