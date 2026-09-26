import type { Emu, Document } from '../src/generated/document.js';
import type { KernelRequest } from '../src/generated/kernel-request.js';
import type { KernelResponse } from '../src/generated/kernel-response.js';
import type { ByteLength, PackageInspectionResponse } from '../src/generated/package-inspection.js';
import type { PptxTextPageRequest } from '../src/generated/pptx-text-page-request.js';
import type { PptxTextPageRasterResponse } from '../src/generated/pptx-text-page-raster-response.js';

const exactCoordinate: Emu = '9223372036854775807';
// @ts-expect-error Coordinates must never pass through JavaScript Number.
const lossyCoordinate: Emu = 9223372036854775807;
const exactLength: ByteLength = '18446744073709551615';
// @ts-expect-error Package byte lengths must remain exact on the wire.
const lossyLength: ByteLength = 18446744073709551615;

function inspectedParts(response: PackageInspectionResponse): number | undefined {
  return response.status === 'inspected' ? response.report.parts.length : undefined;
}

declare const document: Document;
const request: KernelRequest = { operation: 'initialize', document };
// @ts-expect-error Unimplemented operations cannot masquerade as the computation API.
const unknownOperation: KernelRequest = { operation: 'exportPptx', document };

function revision(response: KernelResponse): string | undefined {
  if (response.status === 'prepared' || response.status === 'initialized') {
    return response.snapshot.revision;
  }
  return undefined;
}

void [exactCoordinate, lossyCoordinate, request, unknownOperation, revision];
void [exactLength, lossyLength, inspectedParts];

type FontOffset = PptxTextPageRequest['fonts']['fonts'][number]['offset'];
const textPageFontOffset: FontOffset = '0';
// @ts-expect-error Explicit font resource offsets remain exact decimal strings.
const numericTextPageFontOffset: FontOffset = 0;
function textPageResult(response: PptxTextPageRasterResponse): string {
  if (response.status === 'rendered') return response.info.page.scene.raster.sha256;
  if (response.error.stage === 'fonts') return response.error.error.code;
  if (response.error.stage === 'page') return response.error.error.location?.part ?? '';
  return response.error.error.message;
}
void [textPageFontOffset, numericTextPageFontOffset, textPageResult];

function requiredTextFont(response: PptxTextPageRasterResponse): string | undefined {
  if (response.status !== 'error' || response.error.stage !== 'page') return;
  const detail = response.error.detail;
  if (detail?.kind === 'fontSelection') {
    const use = detail.failure.uses[0];
    const run: number | null | undefined = use?.run;
    void run;
    return detail.failure.selection.typeface;
  }
  const paragraph: number | undefined = response.error.paintLocation?.paragraph;
  void paragraph;
}
void requiredTextFont;

import type { PptxResourcePageRequest } from '../src/generated/pptx-resource-page-request.js';
import type { PptxResourcePageRasterResponse } from '../src/generated/pptx-resource-page-raster-response.js';
const embeddedPageSource: PptxResourcePageRequest['imageSource'] = 'embeddedSnapshot';
// @ts-expect-error Source selection is explicit, without an implicit host fetch policy.
const automaticPageSource: PptxResourcePageRequest['imageSource'] = 'autoFetch';
function resourcePageResult(response: PptxResourcePageRasterResponse): string {
  if (response.status === 'rendered') return response.info.resourcesSha256;
  if (response.error.stage === 'page') {
    const image = response.error.image;
    if (image?.kind === 'resource') return image.result.outcome.status;
    return response.error.error.code;
  }
  return response.error.error.code;
}
void [embeddedPageSource, automaticPageSource, resourcePageResult];
