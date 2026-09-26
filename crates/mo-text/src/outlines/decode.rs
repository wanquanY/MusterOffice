use super::*;
use mo_font::FontInspection;
fn invalid(reason: &'static str) -> TextError {
    TextError::BackendInvalid(reason)
}
struct Reader<'a> {
    words: &'a [u32],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u32], TextError> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or(invalid("outline output overflow"))?;
        let data = self
            .words
            .get(self.offset..end)
            .ok_or(invalid("outline output range"))?;
        self.offset = end;
        Ok(data)
    }
    fn done(&self) -> Result<(), TextError> {
        if self.offset == self.words.len() {
            Ok(())
        } else {
            Err(invalid("trailing outline output"))
        }
    }
}
pub(super) fn decode(
    q: &FontOutlinesRequest,
    metadata: &FontInspection,
    axes: Vec<Vec<EffectiveVariation>>,
    raw: &[u32],
    check: &dyn Fn() -> bool,
) -> Result<FontOutlinesResult, TextError> {
    cancelled(check)?;
    if raw.len() < 2 || raw.len() > MAX_OUTLINES_RESULT_WORDS {
        return Err(invalid("outline batch length"));
    }
    if raw[0] != 0 {
        if raw.len() != 2 || raw[0] > 6 || raw[1] as usize >= q.instances.len() {
            return Err(invalid("outline failure frame"));
        }
        return Err(TextError::BackendFailure {
            status: raw[0],
            run: raw[1],
        });
    }
    if raw[1] as usize != q.instances.len() {
        return Err(invalid("outline instance count"));
    }
    let mut batch = Reader {
        words: raw,
        offset: 2,
    };
    let mut instances = Vec::new();
    for (input, effective_variations) in q.instances.iter().zip(axes) {
        cancelled(check)?;
        let length = batch.take(1)?[0] as usize;
        let maximum = 6 + input.glyph_ids.len() * 3 + input.max_commands as usize * 7;
        if length < 6 || length > maximum {
            return Err(invalid("outline instance length"));
        }
        let mut part = Reader {
            words: batch.take(length)?,
            offset: 0,
        };
        if part.take(6)?
            != [
                OUTLINES_MAGIC,
                1,
                u32::from(metadata.units_per_em),
                u32::from(metadata.units_per_em) * 64,
                input.glyph_ids.len() as u32,
                0,
            ]
        {
            return Err(invalid("outline header"));
        }
        let mut glyphs = Vec::new();
        let mut remaining = input.max_commands as usize;
        for &glyph_id in &input.glyph_ids {
            cancelled(check)?;
            let h = part.take(3)?;
            let count = h[2] as usize;
            if h[0] != glyph_id || h[1] > 1 || (h[1] == 0 && count != 0) || count > remaining {
                return Err(invalid("outline glyph identity, availability or budget"));
            }
            remaining -= count;
            let mut path = Vec::new();
            let mut open = false;
            for _ in 0..count {
                cancelled(check)?;
                let w = part.take(7)?;
                let point = |i| OutlinePoint {
                    x: w[i] as i32,
                    y: w[i + 1] as i32,
                };
                let (command, used) = match w[0] {
                    1 if !open => {
                        open = true;
                        (OutlineCommand::Move { to: point(1) }, 3)
                    }
                    2 if open => (OutlineCommand::Line { to: point(1) }, 3),
                    3 if open => (
                        OutlineCommand::Quadratic {
                            control: point(1),
                            to: point(3),
                        },
                        5,
                    ),
                    4 if open => (
                        OutlineCommand::Cubic {
                            control1: point(1),
                            control2: point(3),
                            to: point(5),
                        },
                        7,
                    ),
                    5 if open => {
                        open = false;
                        (OutlineCommand::Close, 1)
                    }
                    _ => return Err(invalid("outline command or contour order")),
                };
                if w[used..].iter().any(|&v| v != 0) {
                    return Err(invalid("outline reserved coordinates"));
                }
                path.push(command);
            }
            if open {
                return Err(invalid("unclosed outline contour"));
            }
            glyphs.push(GlyphOutline {
                glyph_id,
                path: if h[1] == 1 { Some(path) } else { None },
            });
        }
        part.done()?;
        instances.push(OutlinedInstance {
            effective_variations,
            glyphs,
        });
    }
    batch.done()?;
    cancelled(check)?;
    Ok(FontOutlinesResult {
        font_sha256: metadata.sha256.clone(),
        face_index: metadata.face_index,
        units_per_em: metadata.units_per_em,
        position_units_per_em: u32::from(metadata.units_per_em) * 64,
        profile: "harfbuzz-14.5.0-monochrome-unhinted-design64-nearest-away-v1".into(),
        color_tables: metadata
            .tables
            .iter()
            .filter(|t| {
                [
                    "COLR", "CPAL", "SVG ", "CBDT", "CBLC", "sbix", "EBDT", "EBLC",
                ]
                .contains(&t.tag.as_str())
            })
            .map(|t| t.tag.clone())
            .collect(),
        instances,
    })
}
