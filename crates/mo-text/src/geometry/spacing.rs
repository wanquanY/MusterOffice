use crate::{TextError, itemize::TextItemKind, lines::LineShapeResult};
use mo_geometry::Fixed;

/// Shared logical-line style selection, independent of glyph visibility, font
/// outlines and paragraph boundary placement. Non-text controls do not enlarge
/// a line's text size; tabs do participate; empty/control-only lines use the strut.
pub fn line_style_maximum(
    shaped: &LineShapeResult,
    line: u32,
    values: &[Fixed],
    strut_style: u32,
) -> Result<Fixed, TextError> {
    let invalid = || TextError::Invalid("line style spacing binding");
    let line = shaped.lines.get(line as usize).ok_or_else(invalid)?;
    let items = shaped
        .items
        .get(line.item_start as usize..line.item_end as usize)
        .ok_or_else(invalid)?;
    let mut selected = None;
    for item in items
        .iter()
        .filter(|i| matches!(i.kind, TextItemKind::Text | TextItemKind::Tab))
    {
        let value = *values.get(item.style as usize).ok_or_else(invalid)?;
        selected = Some(selected.map_or(value, |v: Fixed| v.max(value)));
    }
    selected.map_or_else(
        || {
            values
                .get(strut_style as usize)
                .copied()
                .ok_or_else(invalid)
        },
        Ok,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{test_support::*, *};
    use crate::itemize::StyleSpan;
    use mo_common::Emu;
    fn emu(n: i64) -> Fixed {
        Fixed::emu(Emu::new(n))
    }
    #[test]
    fn style_heights_follow_each_line_and_do_not_merge_different_height_styles() {
        let mut q = request("AA");
        let style = q.shaping.paragraph.styles[0].clone();
        q.shaping.paragraph.styles.extend([style.clone(), style]);
        q.shaping.paragraph.spans = vec![
            StyleSpan { end: 1, style: 0 },
            StyleSpan { end: 2, style: 1 },
        ];
        q.styles.extend([q.styles[0]; 2]);
        q.strut_style = 2;
        q.spacing = LineSpacing::StyleMaximum {
            heights: vec![emu(100), emu(200), emu(900)],
        };
        let mut backend = Backend::default();
        let one = layout_lines(&q, FONT, &mut backend, &|| false).unwrap();
        assert_eq!(one.layout.unwrap().height.get(), 200);
        assert_eq!(backend.shapes, 2);
        q.shaping.line_ends = vec![1, 2];
        let two = layout_lines(&q, FONT, &mut Backend::default(), &|| false).unwrap();
        let lines = two.layout.unwrap().lines;
        assert_eq!((lines[0].height.get(), lines[1].height.get()), (100, 200));
        assert_eq!(lines[1].top.get(), 100);
    }
    #[test]
    fn zero_and_empty_heights_are_explicit_and_invalid_vectors_precede_components() {
        let mut q = request("");
        q.spacing = LineSpacing::StyleMaximum {
            heights: vec![emu(123)],
        };
        let r = layout_lines(&q, FONT, &mut Backend::default(), &|| false).unwrap();
        assert_eq!(r.layout.unwrap().height.get(), 123);
        q.spacing = LineSpacing::StyleMaximum {
            heights: vec![Fixed::ZERO],
        };
        assert_eq!(
            layout_lines(&q, FONT, &mut Backend::default(), &|| false)
                .unwrap()
                .layout
                .unwrap()
                .height
                .get(),
            0
        );
        for heights in [vec![], vec![emu(-1)], vec![emu(1), emu(2)]] {
            q.spacing = LineSpacing::StyleMaximum { heights };
            let mut b = Backend::default();
            assert!(layout_lines(&q, FONT, &mut b, &|| false).is_err());
            assert_eq!((b.shapes, b.metrics), (0, 0));
        }
    }
}
