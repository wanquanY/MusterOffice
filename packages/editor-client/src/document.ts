import type {EditorDocumentInput} from '../../contracts/src/generated/editor-page-request.js';
import type {EditorDocumentInfo} from '../../contracts/src/generated/editor-page-response.js';
export type {EditorDocumentInput, EditorDocumentInfo};

/** Validate binding correspondence at the trusted module boundary. Plan hashes
 * and native addresses are always computed by Rust, never reconstructed here. */
export function validateDocumentInfo(info: EditorDocumentInfo, input: EditorDocumentInput): void {
  const fail = () => { throw new Error('Invalid editor document inspection reply'); };
  const digest = (value: string) => /^[0-9a-f]{64}$/.test(value);
  if (!digest(info.sourceSha256) || !Array.isArray(info.slides)) fail();
  const model = input.kind === 'pptx' ? null : input.document;
  if (model) {
    if (info.model?.id !== model.id || !digest(info.model.semanticDigest) ||
        info.slides.length !== model.slideOrder.length ||
        info.pageSize?.width !== model.pageSize.width || info.pageSize.height !== model.pageSize.height) fail();
  } else if (info.model !== null) fail();
  const parts = new Set<string>(), nativeIds = new Set<number>();
  for (const [i, slide] of info.slides.entries()) {
    if (typeof slide.slide !== 'string' || !slide.slide || parts.has(slide.slide) ||
        !Number.isInteger(slide.nativeId) || slide.nativeId < 0 || slide.nativeId > 0xffffffff || nativeIds.has(slide.nativeId) ||
        typeof slide.hidden !== 'boolean' || (slide.name !== null && typeof slide.name !== 'string') ||
        slide.slideId !== (model?.slideOrder[i] ?? null)) fail();
    parts.add(slide.slide); nativeIds.add(slide.nativeId);
  }
}
