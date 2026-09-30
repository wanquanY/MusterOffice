use super::*;
use mo_charts::number_format::{FormatError, Formatter};

pub(super) fn components(
    components: &[ChartLabelComponent],
    normalizations: &[ChartLabelNormalization],
    formatter: &mut Formatter<'_>,
    budget: &mut Budget<'_>,
) -> Result<Vec<ChartLabelComponentDisplay>, ChartLabelError> {
    let mut result = Vec::with_capacity(components.len());
    for component in components {
        budget.check()?;
        budget.bytes(64)?;
        let computed = match component {
            ChartLabelComponent::Text { value, .. } => {
                budget.bytes(value.len())?;
                result.push(ChartLabelComponentDisplay::Text {
                    value: value.clone(),
                });
                continue;
            }
            ChartLabelComponent::Number {
                value,
                format: Some(format),
                ..
            } => formatter.decimal(value, &format.code),
            ChartLabelComponent::Percent {
                normalization,
                point_index,
                format: Some(format),
            } => {
                let ratios = &normalizations
                    .get(*normalization as usize)
                    .ok_or_else(|| PptxError::SourceConflict("label normalization missing".into()))?
                    .ratios;
                // normalize() consumes the ordered point index exactly once.
                // Point lookup must not rescan a complete series for every label.
                let position = ratios
                    .numerators
                    .binary_search_by_key(point_index, |n| n.point_index)
                    .map_err(|_| PptxError::SourceConflict("label ratio point missing".into()))?;
                let numerator = &ratios.numerators[position];
                formatter.ratio(&numerator.numerator, &ratios.denominator, &format.code)
            }
            _ => {
                result.push(ChartLabelComponentDisplay::MissingFormat {});
                continue;
            }
        };
        result.push(match computed {
            Ok(display) => ChartLabelComponentDisplay::Number { display },
            Err(FormatError::Unresolved(issue)) => ChartLabelComponentDisplay::Unresolved { issue },
            Err(e) => return Err(e.into()),
        });
    }
    Ok(result)
}
