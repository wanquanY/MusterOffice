//! One exact pen evaluator for fitting, glyph origins and advance decorations.
use super::GeometryStyle;
use crate::{ShapedGlyph, TextError};
use mo_geometry::{Fixed, Point};

pub struct GlyphPen {
    pub glyph: u32,
    /// Relative glyph origin, y-up, before the baseline shift.
    pub origin: Point,
    pub before: Point,
    /// Natural advance endpoint before the cluster's extra spacing.
    pub unspaced_end: Point,
    pub after: Point,
}
pub struct FragmentPen<'a> {
    glyphs: &'a [ShapedGlyph],
    style: GeometryStyle,
    scale: u32,
    index: usize,
    dx: i64,
    dy: i64,
    extra: Fixed,
    position: Point,
}
impl<'a> FragmentPen<'a> {
    pub fn new(glyphs: &'a [ShapedGlyph], style: GeometryStyle, scale: u32) -> Self {
        Self {
            glyphs,
            style,
            scale,
            index: 0,
            dx: 0,
            dy: 0,
            extra: Fixed::ZERO,
            position: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
        }
    }
    pub fn position(&self) -> Point {
        self.position
    }
    pub fn advance(&mut self) -> Result<Option<GlyphPen>, TextError> {
        let Some(g) = self.glyphs.get(self.index) else {
            return Ok(None);
        };
        let scale = |v| Fixed::scale(v, self.style.font_size, self.scale);
        // Design-unit prefix sums are converted as a whole. Tracking is already
        // Q32 and accumulates exactly, separately from font scaling.
        let add = |a: i64, b: i32| {
            a.checked_add(i64::from(b))
                .ok_or(TextError::Invalid("pen design-unit range"))
        };
        let origin = Point {
            x: scale(add(self.dx, g.x_offset)?)?.checked_add(self.extra)?,
            y: scale(add(self.dy, g.y_offset)?)?,
        };
        let dx = add(self.dx, g.x_advance)?;
        let dy = add(self.dy, g.y_advance)?;
        let unspaced_end = Point {
            x: scale(dx)?.checked_add(self.extra)?,
            y: scale(dy)?,
        };
        let mut after = unspaced_end;
        let mut extra = self.extra;
        if self
            .glyphs
            .get(self.index + 1)
            .is_none_or(|next| next.cluster != g.cluster)
        {
            extra = extra.checked_add(self.style.cluster_spacing)?;
            after.x = after.x.checked_add(self.style.cluster_spacing)?;
        }
        let out = GlyphPen {
            glyph: self.index as u32,
            origin,
            before: self.position,
            unspaced_end,
            after,
        };
        self.position = after;
        self.dx = dx;
        self.dy = dy;
        self.extra = extra;
        self.index += 1;
        Ok(Some(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mo_common::Emu;
    fn glyph(cluster: u32, advance: i32, offset: i32) -> ShapedGlyph {
        ShapedGlyph {
            glyph_id: 2,
            cluster,
            unsafe_to_break: false,
            unsafe_to_concat: false,
            safe_to_insert_tatweel: false,
            x_advance: advance,
            y_advance: 0,
            x_offset: offset,
            y_offset: 0,
        }
    }
    #[test]
    fn spacing_occurs_after_clusters_including_reversed_clusters_and_marks() {
        for clusters in [[0, 0, 2], [2, 2, 0]] {
            let glyphs = [
                glyph(clusters[0], 64, 0),
                glyph(clusters[1], 0, -32),
                glyph(clusters[2], 64, 0),
            ];
            let style = GeometryStyle {
                font_size: Emu::new(1000),
                baseline_shift: Emu::ZERO.into(),
                cluster_spacing: Fixed::emu(Emu::new(100)),
            };
            let mut pen = FragmentPen::new(&glyphs, style, 64);
            assert_eq!(
                pen.advance()
                    .unwrap()
                    .unwrap()
                    .after
                    .x
                    .wire()
                    .unwrap()
                    .get(),
                1000
            );
            let mark = pen.advance().unwrap().unwrap();
            assert_eq!(mark.origin.x.wire().unwrap().get(), 500);
            assert_eq!(mark.after.x.wire().unwrap().get(), 1100);
            let next = pen.advance().unwrap().unwrap();
            assert_eq!(next.origin.x.wire().unwrap().get(), 1100);
            assert_eq!(next.after.x.wire().unwrap().get(), 2200);
            assert!(pen.advance().unwrap().is_none());
        }
    }
    #[test]
    fn negative_tracking_keeps_unspaced_extent_and_overflow_is_atomic() {
        let glyphs = [glyph(0, 64, 0), glyph(1, 64, 0)];
        let mut style = GeometryStyle {
            font_size: Emu::new(1000),
            baseline_shift: Emu::ZERO.into(),
            cluster_spacing: Fixed::emu(Emu::new(-1500)),
        };
        let mut pen = FragmentPen::new(&glyphs, style, 64);
        let first = pen.advance().unwrap().unwrap();
        assert_eq!(first.unspaced_end.x.wire().unwrap().get(), 1000);
        assert_eq!(first.after.x.wire().unwrap().get(), -500);
        assert_eq!(
            pen.advance()
                .unwrap()
                .unwrap()
                .after
                .x
                .wire()
                .unwrap()
                .get(),
            -1000
        );
        style.cluster_spacing = Fixed::from_raw(i128::MAX);
        let mut pen = FragmentPen::new(&glyphs, style, 64);
        assert!(pen.advance().is_err());
        assert_eq!(pen.position().x, Fixed::ZERO);
        assert!(pen.advance().is_err());
    }
}
