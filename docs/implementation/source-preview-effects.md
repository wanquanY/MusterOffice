# Source presentation preview effects

Implemented 2026-10-22. This is a bounded computation profile, not an Office/WPS interoperability certification or a product deployment receipt.

## Source semantics

The source resolver selects one complete effect declaration through the physical slide/layout/master placeholder hierarchy. A direct empty effect list or effectRef index 0 stops inheritance. Nonzero references use the consuming surface's selected format scheme and the reference color as phClr; color transforms retain the existing precision rules. Source package bytes are never rewritten.

A single unscaled, unskewed outerShdw is supported. Its direction and distance become a world-space translation, optionally rotated/reflected with the shape. Blur radius is mapped to Gaussian sigma = blurRad / 2 in this explicitly versioned profile. Other effect kinds, effect DAGs, 3D properties, scaled/skewed masks and group-level effects remain explicit unsupported outcomes.

## Raster contract

Frame v15 extends each isolated opacity-group record from 3 to 8 words: first draw, exclusive end draw, opacity, shadow-present, packed RGBA, dx, dy, sigma. Captures retain v14 scope words. Frames without shadows retain their previous encoding. A consumer must advertise outer-shadow ABI 1; older consumers reject v15.

The shadow uses the complete object's composited alpha (fill, stroke, image and text), and is composited behind that object before group opacity. Nested and sibling groups share two scratch alpha planes. This raster profile convolves the viewport-clipped object alpha with transparent exterior; source ink outside the viewport is not recovered by the filter.

A normalized separable Gaussian is truncated at four sigma. Fractional translations use bilinear sampling. Each continuation performs at most 4096 filter taps, then yields to the existing cancellation boundary. Sigma is limited to 32 pixels, offsets to 8192 pixels, total group/snapshot/scratch memory to 64 MiB and charged pixel work to 268435456. Limits are checked in Rust and again before native/WASM allocations.

## Imported text

FontManifest.defaultHanScript is an optional, explicit host choice among Hans/Hant/Jpan/Hang for Han text without a disambiguating source language. Source language wins; no operating-system locale is consulted. Absence retains strict source resolution. Actual theme key and native font remain recorded in SourceFontBinding. Ordinary and table frames consume the same manifest choice. Font substitution and glyph coverage continue to require an explicit manifest.

Saved spAutoFit frames are admitted only when completed layout measurements show no horizontal or vertical excess. The containing geometry is then already sufficient. Frames requiring shape growth and normAutofit scaling remain unsupported; no resizing instruction is silently discarded.

## Verification

- Native source effects: direct/theme selection, phClr/alpha, explicit-empty and zero-reference precedence, unsupported effects fail closed.
- Native raster: whole-object opacity, Gaussian symmetry, fractional translation, nested/sibling scopes and malformed/budget rejection.
- tools/verification/outer-shadow-parity.mjs compares five real native frames with WASM pixels.
- Source glyph tests verify strict mode, explicit Han policy and source-language precedence. Source frame tests compare an already-fitting spAutoFit frame and reject one requiring growth.
- A privately supplied three-slide presentation was rendered unchanged with explicit host fonts; no private file or image is included in this public repository.

Paired SDK/Worker packaging and downstream product verification are separate acceptance gates.

## References

- [DrawingML effect styles](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.effectstyle?view=openxml-3.0.1)
- [Office spAutoFit behavior](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oe376/19d31037-28bf-4693-85be-3a37bfe82d51)
