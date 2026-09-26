//! Derived source geometry. Never writes inherited values into author records.
use super::*;
use crate::{PptxError, cancelled};
use mo_presentation_model::{Point, Size};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum PlaceholderKind {
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "body")]
    Body,
    #[serde(rename = "ctrTitle")]
    CenterTitle,
    #[serde(rename = "subTitle")]
    Subtitle,
    #[serde(rename = "dt")]
    Date,
    #[serde(rename = "sldNum")]
    SlideNumber,
    #[serde(rename = "ftr")]
    Footer,
    #[serde(rename = "hdr")]
    Header,
    #[serde(rename = "obj")]
    Object,
    #[serde(rename = "chart")]
    Chart,
    #[serde(rename = "tbl")]
    Table,
    #[serde(rename = "clipArt")]
    ClipArt,
    #[serde(rename = "dgm")]
    Diagram,
    #[serde(rename = "media")]
    Media,
    #[serde(rename = "sldImg")]
    SlideImage,
    #[serde(rename = "pic")]
    Picture,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum PlaceholderOrientation {
    #[serde(rename = "horz")]
    Horizontal,
    #[serde(rename = "vert")]
    Vertical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PlaceholderSize {
    Full,
    Half,
    Quarter,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePlaceholder {
    /// None preserves absent author attributes, distinct from explicit defaults.
    pub kind: Option<PlaceholderKind>,
    pub index: Option<u32>,
    pub orientation: Option<PlaceholderOrientation>,
    pub size: Option<PlaceholderSize>,
    pub custom_prompt: Option<bool>,
}
impl SourcePlaceholder {
    pub fn effective_kind(&self) -> PlaceholderKind {
        self.kind.unwrap_or(PlaceholderKind::Object)
    }
    pub fn effective_index(&self) -> u32 {
        self.index.unwrap_or(0)
    }
    pub(super) fn read(e: &mo_xml::Element) -> Result<Self, mo_xml::XmlError> {
        fn enumeration<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, mo_xml::XmlError> {
            T::deserialize(
                serde::de::value::StrDeserializer::<serde::de::value::Error>::new(value.trim()),
            )
            .map_err(|_| malformed("invalid placeholder enumeration"))
        }
        Ok(Self {
            kind: e.attribute("type").map(enumeration).transpose()?,
            index: e
                .attribute("idx")
                .map(|v| integer(Some(v), "placeholder index"))
                .transpose()?,
            orientation: e.attribute("orient").map(enumeration).transpose()?,
            size: e.attribute("sz").map(enumeration).transpose()?,
            custom_prompt: e.attribute("hasCustomPrompt").map(boolean).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceObjectRef {
    pub part: String,
    pub native_id: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceResolvedValue<T> {
    pub value: T,
    /// Ultimate explicit declaration, not merely the next inheritance hop.
    pub declared_by: SourceObjectRef,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PlaceholderMatchRule {
    SlideIndex,
    MasterType,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourcePlaceholderMatch {
    #[default]
    NotPlaceholder,
    Master,
    Matched {
        target: SourceObjectRef,
        rule: PlaceholderMatchRule,
    },
    /// Missing relationship or matching placeholder; no synthetic geometry.
    Unmatched,
    /// Slide idx=0xffffffff is explicitly detached from layout matching.
    Detached,
    Ambiguous {
        part: String,
        candidates: u32,
    },
    UnsupportedContext,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceObjectResolution {
    pub placeholder_match: SourcePlaceholderMatch,
    pub origin: Option<SourceResolvedValue<Point>>,
    pub size: Option<SourceResolvedValue<Size>>,
}

#[derive(Default)]
struct Lookup {
    indices: BTreeMap<u32, Vec<usize>>,
    kinds: BTreeMap<PlaceholderKind, Vec<usize>>,
}
// PowerPoint's placeholder families as documented/tested by python-pptx.
// Header/slide-image belong to notes/handout contexts, not these slide surfaces.
pub(super) fn master_kind(kind: PlaceholderKind) -> Option<PlaceholderKind> {
    use PlaceholderKind::*;
    match kind {
        Title | CenterTitle => Some(Title),
        Body | Subtitle | Object | Chart | Table | ClipArt | Diagram | Media | Picture => {
            Some(Body)
        }
        Date | SlideNumber | Footer => Some(kind),
        Header | SlideImage => None,
    }
}

pub(super) fn resolve(
    surfaces: &mut BTreeMap<String, SourceSurface>,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    let mut lookups = BTreeMap::new();
    for (part, surface) in surfaces.iter() {
        let mut lookup = Lookup::default();
        for (i, object) in surface.objects.iter().enumerate() {
            cancelled(check)?;
            if object.parent_group.is_none()
                && let Some(ph) = &object.placeholder
            {
                lookup
                    .indices
                    .entry(ph.effective_index())
                    .or_insert_with(Vec::new)
                    .push(i);
                lookup
                    .kinds
                    .entry(ph.effective_kind())
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }
        lookups.insert(part.clone(), lookup);
    }
    // The three domain levels are acyclic even though OPC master/layout links
    // are reciprocal. No recursion, arbitrary graph walk or first-match guess.
    for kind in [SurfaceKind::Master, SurfaceKind::Layout, SurfaceKind::Slide] {
        let parts: Vec<_> = surfaces
            .iter()
            .filter(|(_, s)| s.kind == kind)
            .map(|(p, _)| p.clone())
            .collect();
        for part in parts {
            cancelled(check)?;
            let surface = &surfaces[&part];
            let parent = match kind {
                SurfaceKind::Master => None,
                SurfaceKind::Layout => surface.links.master.as_deref(),
                SurfaceKind::Slide => surface.links.layout.as_deref(),
            };
            let mut theme = parent
                .map(|p| surfaces[p].effective_theme.clone())
                .unwrap_or_default();
            // A slide inherits the layout's effective map; that layout in turn
            // inherits its master. Keep the explicit masterClrMapping marker in
            // author data even when it selects the same parent as omission.
            let color_mapping = match &surface.color_mapping {
                Some(SourceColorMapping::Explicit { source_ordinal, .. }) => {
                    Some(SourceColorMapRef {
                        part: part.clone(),
                        source_ordinal: *source_ordinal,
                    })
                }
                _ => parent.and_then(|p| surfaces[p].resolved_color_mapping.clone()),
            };
            if let Some(base) = &surface.links.theme {
                theme.base = Some(base.clone());
            }
            if let Some(over) = &surface.links.theme_override {
                theme.overrides.push(over.clone());
            }
            let mut resolutions = Vec::with_capacity(surface.objects.len());
            for object in &surface.objects {
                cancelled(check)?;
                let mut inherited = None;
                let placeholder_match = if let Some(ph) = &object.placeholder {
                    if object.parent_group.is_some()
                        || object.kind == SourceObjectKind::Group
                        || (kind != SurfaceKind::Slide && ph.effective_index() == u32::MAX)
                        || matches!(
                            ph.effective_kind(),
                            PlaceholderKind::Header | PlaceholderKind::SlideImage
                        )
                    {
                        SourcePlaceholderMatch::UnsupportedContext
                    } else if kind == SurfaceKind::Master {
                        SourcePlaceholderMatch::Master
                    } else if kind == SurfaceKind::Slide && ph.effective_index() == u32::MAX {
                        SourcePlaceholderMatch::Detached
                    } else if let Some(parent) = parent {
                        let lookup = &lookups[parent];
                        let candidates = if kind == SurfaceKind::Slide {
                            lookup.indices.get(&ph.effective_index())
                        } else {
                            master_kind(ph.effective_kind()).and_then(|k| lookup.kinds.get(&k))
                        };
                        match candidates.map(Vec::as_slice).unwrap_or(&[]) {
                            [] => SourcePlaceholderMatch::Unmatched,
                            [i] => {
                                let target = &surfaces[parent].objects[*i];
                                if target.resolution.placeholder_match
                                    == SourcePlaceholderMatch::UnsupportedContext
                                {
                                    SourcePlaceholderMatch::UnsupportedContext
                                } else {
                                    inherited = Some(&target.resolution);
                                    SourcePlaceholderMatch::Matched {
                                        target: SourceObjectRef {
                                            part: parent.into(),
                                            native_id: target.native_id,
                                        },
                                        rule: if kind == SurfaceKind::Slide {
                                            PlaceholderMatchRule::SlideIndex
                                        } else {
                                            PlaceholderMatchRule::MasterType
                                        },
                                    }
                                }
                            }
                            many => SourcePlaceholderMatch::Ambiguous {
                                part: parent.into(),
                                candidates: many
                                    .len()
                                    .try_into()
                                    .map_err(|_| PptxError::Limit("placeholder candidates"))?,
                            },
                        }
                    } else {
                        SourcePlaceholderMatch::Unmatched
                    }
                } else {
                    SourcePlaceholderMatch::NotPlaceholder
                };
                let declared_by = SourceObjectRef {
                    part: part.clone(),
                    native_id: object.native_id,
                };
                resolutions.push(SourceObjectResolution {
                    placeholder_match,
                    origin: object
                        .transform
                        .as_ref()
                        .and_then(|t| t.origin)
                        .map(|value| SourceResolvedValue {
                            value,
                            declared_by: declared_by.clone(),
                        })
                        .or_else(|| inherited.and_then(|r| r.origin.clone())),
                    size: object
                        .transform
                        .as_ref()
                        .and_then(|t| t.size)
                        .map(|value| SourceResolvedValue { value, declared_by })
                        .or_else(|| inherited.and_then(|r| r.size.clone())),
                });
            }
            let surface = surfaces.get_mut(&part).expect("existing surface");
            surface.effective_theme = theme;
            surface.resolved_color_mapping = color_mapping;
            for (object, resolution) in surface.objects.iter_mut().zip(resolutions) {
                object.resolution = resolution;
            }
        }
    }
    Ok(())
}
