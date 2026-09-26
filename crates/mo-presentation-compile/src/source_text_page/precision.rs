use super::*;
/// Bound relative to fixed HarfBuzz integer outputs and selected line breaks.
/// Each design-unit scale/max contributes <= 1/2 raw Q32; natural line height
/// <= 3/2. All preceding heights plus vertical anchoring contribute <= 3L.
/// Baseline/half rounding and path-control rounding are covered by the constant.
/// Prefix displacement and pen-based alignment contribute <= 2F. Inner-region
/// uncertainty occurs at most three times per axis; four is conservative.
/// This excludes source guide error, font-engine error and break-topology changes.
pub(super) fn frame_bound(frame: &SourceFramePlan) -> Result<Fixed, SourcePageError> {
    let mut lines = 0usize;
    let mut fragments = 0usize;
    for p in &frame.paragraphs {
        let layout = p
            .computed
            .geometry
            .paths
            .layout
            .geometry
            .as_ref()
            .expect("complete frame");
        for line in &layout.shaping.lines {
            lines += 1;
            let count: usize = layout.shaping.fallback.items
                [line.fallback_start as usize..line.fallback_end as usize]
                .iter()
                .map(|i| i.fragments.len())
                .sum();
            fragments = fragments.max(count);
        }
    }
    // A native fractional baseline is rounded once to Q32 (<= 1/2 raw).
    // Each line's max ascent/descent, accumulated heights, anchoring and
    // glyph displacement contribute conservatively <= 2L + 2 extra raw units.
    let fractional_baseline = frame
        .inputs
        .iter()
        .any(|p| p.baseline_conversion_error != Fixed::ZERO);
    let baseline_error = if fractional_baseline {
        2 * lines as i128 + 2
    } else {
        0
    };
    // Native percentage heights and both paragraph gaps are each rounded once.
    // Cover cumulative line/paragraph height, half-leading and frame anchoring.
    let spacing_error = if frame
        .paragraphs
        .iter()
        .any(|p| p.spec.spacing_conversion_error != Fixed::ZERO)
    {
        2 * lines as i128 + 4 * frame.paragraphs.len() as i128 + 2
    } else {
        0
    };
    let base = Fixed::from_raw(
        4 * lines as i128
            + 2 * fragments as i128
            + 8
            + baseline_error
            + spacing_error
            + if frame
                .inputs
                .iter()
                .any(|p| p.tracking_conversion_error != Fixed::ZERO)
            {
                // At most one source rounding per shaped cluster. Prefix,
                // alignment, and decorations are conservatively bounded by
                // twice the complete frame's glyph count plus two raw units.
                2 * frame.glyphs.len() as i128 + 2
            } else {
                0
            },
    );
    let region = frame.region.conversion_error_bound;
    Ok(base
        .checked_add(region)?
        .checked_add(region)?
        .checked_add(region)?
        .checked_add(region)?)
}
