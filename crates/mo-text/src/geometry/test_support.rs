use super::*;
use crate::{cascade::*, itemize::StyleSpan, paragraph::*};
use mo_common::{ByteLength, Digest, Emu};
use mo_unicode::bidi::ParagraphDirection;
use sha2::{Digest as _, Sha256};
pub(crate) const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
pub(crate) fn request(text: &str) -> LineGeometryRequest {
    LineGeometryRequest {
        tabs: None,
        shaping: lines::LineShapeRequest {
            paragraph: ParagraphShapeRequest {
                text: text.into(),
                direction: ParagraphDirection::LeftToRight,
                spans: if text.is_empty() {
                    vec![]
                } else {
                    vec![StyleSpan {
                        end: text.chars().count() as u32,
                        style: 0,
                    }]
                },
                styles: vec![ParagraphTextStyle {
                    language: "und".into(),
                    features: vec![],
                    candidates: vec![FontCandidate {
                        font: 0,
                        variations: vec![],
                    }],
                    suppress_dotted_circle: false,
                    max_glyphs: 100,
                }],
                fonts: vec![CascadeFont {
                    expected_sha256: Digest::from_sha256(Sha256::digest(FONT).into()),
                    face_index: 0,
                    offset: ByteLength::new(0),
                    byte_length: ByteLength::new(FONT.len() as u64),
                }],
            },
            line_ends: vec![text.chars().count() as u32],
        },
        styles: vec![GeometryStyle {
            cluster_spacing: mo_geometry::Fixed::ZERO,
            font_size: Emu::new(254000),
            baseline_shift: Emu::ZERO.into(),
        }],
        strut_style: 0,
        spacing: LineSpacing::Natural,
    }
}
#[derive(Default)]
pub(crate) struct Backend {
    pub(crate) shapes: usize,
    pub(crate) metrics: usize,
    pub(crate) invalid: bool,
    pub(crate) metric_missing: bool,
    pub(crate) metric_fail: bool,
    pub(crate) glyph_position: Option<[i32; 4]>,
    pub(crate) zero_metrics: bool,
}
impl backend::TextBackend for Backend {
    fn shape_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        let runs = backend::decode_requests(frame)?;
        let mut out = vec![0, runs.len() as u32];
        for run in runs {
            self.shapes += 1;
            let r = run.words;
            out.extend([
                8 + r[8] * 7,
                backend::COMPONENT_MAGIC,
                1,
                1000,
                64000,
                r[8],
                r[4],
                r[5],
                0,
            ]);
            let mut indices: Vec<_> = (r[7]..r[7] + r[8]).collect();
            if r[2] == 5 {
                indices.reverse();
            }
            for i in indices {
                let p = self.glyph_position.unwrap_or([38400, 0, 0, 0]);
                out.extend([2, i, 0]);
                out.extend(p.map(|v| v as u32));
            }
        }
        Ok(out)
    }
    fn measure_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.metrics += 1;
        if self.metric_fail {
            return Ok(vec![2, 0]);
        }
        let inputs = backend::decode_metric_requests(frame)?;
        let mut out = vec![0, inputs.len() as u32];
        for input in inputs {
            assert_eq!(input[4], 3);
            out.extend([15, backend::METRICS_MAGIC, 1, 1000, 64000, 3, 0]);
            for (&tag, &value) in input[6 + input[3] as usize * 2..]
                .iter()
                .zip(&[51200i32, -12800, 1600])
            {
                out.extend([
                    tag,
                    u32::from(!self.metric_missing),
                    if self.metric_missing || self.zero_metrics {
                        0
                    } else {
                        value as u32
                    },
                ]);
            }
        }
        Ok(out)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
