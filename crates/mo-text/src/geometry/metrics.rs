use super::*;
use crate::metrics::{FontMetric, FontMetricsInstance, FontMetricsRequest, measure_verified};
use mo_font::VerifiedFont;
use std::collections::BTreeMap;
const METRICS: [FontMetric; 3] = [
    FontMetric::HorizontalAscender,
    FontMetric::HorizontalDescender,
    FontMetric::HorizontalLineGap,
];
pub(super) struct Measurements {
    pub instances: Vec<MetricInstance>,
    pub fragments: Vec<Vec<Option<usize>>>,
    pub strut: usize,
}
pub(super) fn measure(
    q: &LineGeometryRequest,
    shaped: &lines::LineShapeResult,
    fonts: &[VerifiedFont<'_>],
    bindings: &[usize],
    backend: &mut dyn backend::TextBackend,
    issues: &mut Vec<GeometryIssue>,
    check: &dyn Fn() -> bool,
) -> Result<Measurements, TextError> {
    let styles = &q.shaping.paragraph.styles;
    let mut unique = BTreeMap::new();
    let mut candidates = Vec::new();
    let mut register = |candidate: &cascade::FontCandidate| -> Result<usize, TextError> {
        let mut coords: Vec<_> = candidate
            .variations
            .iter()
            .map(|v| (v.tag.clone(), v.value_16_16))
            .collect();
        coords.sort();
        let key = (bindings[candidate.font as usize], coords);
        if let Some(&i) = unique.get(&key) {
            return Ok(i);
        }
        if candidates.len() >= 2048 {
            return Err(TextError::Limit("line metric instances"));
        }
        let i = candidates.len();
        unique.insert(key, i);
        candidates.push(candidate.clone());
        Ok(i)
    };
    let strut = register(&styles[q.strut_style as usize].candidates[0])?;
    let mut fragments = Vec::new();
    for (i, item) in shaped.fallback.items.iter().enumerate() {
        let style = &styles[shaped.items[shaped.shaped_item_indices[i] as usize].style as usize];
        let mut values = Vec::new();
        for fragment in &item.fragments {
            cancelled(check)?;
            values.push(match fragment {
                FontFragment::Selected { candidate, .. } => {
                    Some(register(&style.candidates[*candidate as usize])?)
                }
                FontFragment::Unresolved { start, end } => {
                    issues.push(GeometryIssue::UnresolvedFont {
                        start: *start,
                        end: *end,
                    });
                    None
                }
            });
        }
        fragments.push(values);
    }
    let mut by_font = BTreeMap::<usize, Vec<usize>>::new();
    for (i, c) in candidates.iter().enumerate() {
        by_font
            .entry(bindings[c.font as usize])
            .or_default()
            .push(i);
    }
    let mut measured = vec![None; candidates.len()];
    for (font, indices) in by_font {
        let font = &fonts[font];
        for batch in indices.chunks(256) {
            cancelled(check)?;
            let request = FontMetricsRequest {
                expected_sha256: font.metadata().sha256.clone(),
                face_index: font.metadata().face_index,
                instances: batch
                    .iter()
                    .map(|&i| FontMetricsInstance {
                        variations: candidates[i].variations.clone(),
                        metrics: METRICS.to_vec(),
                    })
                    .collect(),
            };
            let result = measure_verified(&request, font, backend, check)?;
            for (&index, value) in batch.iter().zip(result.instances) {
                for metric in &value.values {
                    if metric.position.is_none() {
                        issues.push(GeometryIssue::MissingMetric {
                            instance: index as u32,
                            metric: metric.metric,
                        });
                    }
                }
                measured[index] = Some(MetricInstance {
                    font: candidates[index].font,
                    variations: candidates[index].variations.clone(),
                    position_units_per_em: result.position_units_per_em,
                    measured: value,
                });
            }
        }
    }
    Ok(Measurements {
        instances: measured.into_iter().map(Option::unwrap).collect(),
        fragments,
        strut,
    })
}
