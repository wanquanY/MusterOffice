use super::*;
use crate::{PptxError, R, value};
use mo_opc::{PartName, RelationshipSource};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePartRef {
    pub part: String,
    pub sha256: Digest,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSurfaceLinks {
    pub layout: Option<String>,
    pub master: Option<String>,
    pub theme: Option<SourcePartRef>,
    pub theme_override: Option<SourcePartRef>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceThemeStack {
    pub base: Option<SourcePartRef>,
    /// Ordered from master toward slide. Parts remain source-bound; colors,
    /// fonts and effects are not flattened or resolved by this stack alone.
    pub overrides: Vec<SourcePartRef>,
}

pub(super) fn read(
    source: &dyn PackageRead,
    part: &PartName,
    kind: SurfaceKind,
) -> Result<SourceSurfaceLinks, PptxError> {
    let mut links = SourceSurfaceLinks::default();
    for rel in source
        .relationships()
        .get(&RelationshipSource::Part(part.clone()))
        .into_iter()
        .flatten()
    {
        let Some(suffix) = rel.relationship_type.strip_prefix(&format!("{R}/")) else {
            continue;
        };
        let allowed = match suffix {
            "slideLayout" => matches!(kind, SurfaceKind::Slide | SurfaceKind::Master),
            "slideMaster" => kind == SurfaceKind::Layout,
            "theme" => kind == SurfaceKind::Master,
            "themeOverride" => matches!(kind, SurfaceKind::Slide | SurfaceKind::Layout),
            _ => continue,
        };
        if !allowed {
            return Err(value(
                part.to_string(),
                format!("invalid {suffix} relationship on {kind:?}"),
            ));
        }
        let target = presentation::internal(rel)?;
        match suffix {
            "slideLayout" if kind == SurfaceKind::Slide => {
                unique(&mut links.layout, target.to_string(), part)?
            }
            "slideMaster" => unique(&mut links.master, target.to_string(), part)?,
            "theme" | "themeOverride" => {
                let info = &source.parts()[&target];
                if info.content_type
                    != format!("application/vnd.openxmlformats-officedocument.{suffix}+xml")
                {
                    return Err(value(
                        target.to_string(),
                        "theme relationship content type mismatch",
                    ));
                }
                let slot = if suffix == "theme" {
                    &mut links.theme
                } else {
                    &mut links.theme_override
                };
                unique(
                    slot,
                    SourcePartRef {
                        part: target.to_string(),
                        sha256: info.sha256.clone(),
                    },
                    part,
                )?;
            }
            _ => {}
        }
    }
    Ok(links)
}

fn unique<T>(slot: &mut Option<T>, value_: T, part: &PartName) -> Result<(), PptxError> {
    if slot.is_some() {
        return Err(value(
            part.to_string(),
            "multiple inheritance relationships of the same kind",
        ));
    }
    *slot = Some(value_);
    Ok(())
}
