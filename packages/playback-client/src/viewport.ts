import type {RasterViewport} from '../../contracts/src/generated/playback-session-request.js';
/** Exact request/receipt comparison; raster normalization stays in Rust. */
export function sameViewport(a: RasterViewport, b: RasterViewport): boolean {
  return a.width === b.width && a.height === b.height &&
    a.origin.x === b.origin.x && a.origin.y === b.origin.y &&
    a.scale.numerator === b.scale.numerator && a.scale.denominator === b.scale.denominator &&
    a.coordinateTolerance === b.coordinateTolerance &&
    a.background.length === 4 && a.background.every((v, i) => v === b.background[i]);
}

export type ViewportTarget = {viewport: RasterViewport} | {width: number; height: number};
export function fitsViewport(viewport: RasterViewport, target: ViewportTarget): boolean {
  if ('viewport' in target) return sameViewport(viewport, target.viewport);
  return Number.isInteger(viewport.width) && Number.isInteger(viewport.height) &&
    viewport.width > 0 && viewport.height > 0 &&
    viewport.width <= Math.min(8192, target.width) && viewport.height <= Math.min(8192, target.height) &&
    viewport.width * viewport.height <= 16777216;
}
