//! Chart-local theme context shared by paint and text computations. The caller
//! accounts prior source reads; this reader charges the same remaining budget.
use super::*;
use crate::source::theme;

pub fn read_theme_override(
    package: &dyn PackageRead,
    chart: &SourceChartPart,
    source_limits: SourceLimits,
    limits: SourceChartLimits,
    bytes_left: &mut u64,
    check: &dyn Fn() -> bool,
) -> Result<Option<(String, theme::SourceThemePart)>, PptxError> {
    let part = PartName::new(&chart.part)?;
    let info = package
        .parts()
        .get(&part)
        .ok_or_else(|| invalid("chart context part missing"))?;
    if info.sha256 != chart.sha256 || info.content_type != CHART_TYPE {
        return Err(invalid("chart context differs from source"));
    }
    let mut target = None;
    for (i, rel) in package
        .relationships()
        .get(&RelationshipSource::Part(part))
        .into_iter()
        .flatten()
        .enumerate()
    {
        cancelled(check)?;
        if i >= limits.max_relationship_steps {
            return Err(PptxError::Limit("chart paint relationship steps"));
        }
        if rel.relationship_type != format!("{R}/themeOverride") {
            continue;
        }
        if target.is_some() {
            return Err(invalid("multiple chart theme overrides"));
        }
        let RelationshipTarget::Internal {
            part,
            fragment: None,
        } = &rel.resolved
        else {
            return Err(invalid("chart theme override must be an internal part"));
        };
        let info = package
            .parts()
            .get(part)
            .ok_or_else(|| invalid("missing chart theme override"))?;
        if info.content_type != "application/vnd.openxmlformats-officedocument.themeOverride+xml" {
            return Err(invalid("chart theme override content type"));
        }
        if source_limits.max_theme_parts == 0 {
            return Err(PptxError::Limit("chart theme parts"));
        }
        *bytes_left = bytes_left
            .checked_sub(info.byte_length)
            .ok_or(PptxError::Limit("chart paint total part bytes"))?;
        let bytes = package.read_part(
            part,
            limits
                .max_part_bytes
                .min(source_limits.package.xml.max_bytes) as u64,
            check,
        )?;
        target = Some((
            part.to_string(),
            theme::read_override(&bytes, info.sha256.clone(), source_limits, check)?,
        ));
    }
    Ok(target)
}
