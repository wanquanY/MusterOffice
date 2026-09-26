use super::{read::retain, *};
use crate::source::effects::{PendingEffect, Slots};
use crate::{
    A, P, R,
    source::{boolean, drawingml::*, integer, malformed},
};
use mo_xml::{Element, ExpandedName, XmlError};

pub(super) enum Node {
    EffectDraft {
        pending: PendingEffect,
        slots: Slots,
    },
    EffectList(Vec<u32>),
    EffectStyle {
        effects: Option<SourceEffectProperties>,
    },
    EffectLink(u32),
    Effects(SourceEffectProperties),
    FinishedEffectStyle(SourceEffectStyle),
    Fill(SourceFill),
    Stops(SourceGradientStops),
    Stop {
        position: NativePercentage,
        color: Option<SourceColor>,
    },
    Color(SourceColor),
    Transform(SourceColorTransform),
    Shade(SourceGradientShade),
    Rect(SourceFillRect),
    ColorWrapper(Option<SourceColor>),
    Blip(SourceFillBlip),
    Mode(SourceImageFillMode),
    Reference(SourceFillReference),
    Background {
        black_white_mode: Option<NativeBlackWhiteMode>,
        definition: Option<SourceBackgroundDefinition>,
    },
    BackgroundProperties {
        shade_to_title: Option<bool>,
        fill: Option<SourceFill>,
        effects: Option<SourceEffectProperties>,
        retained_ordinals: Vec<u32>,
    },
}
pub(super) struct Frame {
    pub ordinal: u32,
    pub value: Node,
    pub(super) name: String,
    rank: u8,
    retained_start: usize,
}
#[derive(Clone, Copy)]
pub(super) enum Child {
    Ordinary,
    Opaque,
    Effect,
}
fn attributes(element: &Element, allowed: &[&str], ordinal: u32, retained: &mut Vec<u32>) {
    if element.attributes.iter().any(|a| {
        if element.name.is(A, "blip")
            && a.name.namespace == R
            && ["embed", "link"].contains(&a.name.local.as_str())
        {
            return false;
        }
        !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str())
    }) {
        retain(retained, ordinal);
    }
}
fn coord(element: &Element, name: &str) -> Result<Option<NativeCoordinate>, XmlError> {
    element
        .attribute(name)
        .map(|s| s.trim().to_owned().try_into().map_err(malformed))
        .transpose()
}
impl Frame {
    pub fn new(
        e: &Element,
        ordinal: u32,
        retained: &mut Vec<u32>,
        is_effect: bool,
    ) -> Result<Self, XmlError> {
        let retained_start = retained.len();
        let name = e.name.local.as_str();
        let mut attrs: &[&str] = &[];
        let fill = |definition| {
            Node::Fill(SourceFill {
                source_ordinal: ordinal,
                definition,
                retained_ordinals: Vec::new(),
            })
        };
        let value = if is_effect {
            let (value, allowed) = super::effect::new(e)?;
            attrs = allowed;
            value
        } else {
            match name {
                "effectLst" => Node::EffectList(Vec::new()),
                "effectStyle" => Node::EffectStyle { effects: None },
                "noFill" => fill(SourceFillDefinition::None {}),
                "solidFill" => fill(SourceFillDefinition::Solid { color: None }),
                "grpFill" => fill(SourceFillDefinition::Group {}),
                "gradFill" => {
                    attrs = &["flip", "rotWithShape"];
                    fill(SourceFillDefinition::Gradient(SourceGradientFill {
                        stops: None,
                        shade: None,
                        tile_rect: None,
                        flip: e.attribute("flip").map(enumeration).transpose()?,
                        rotate_with_shape: e.attribute("rotWithShape").map(boolean).transpose()?,
                    }))
                }
                "pattFill" => {
                    attrs = &["prst"];
                    fill(SourceFillDefinition::Pattern(SourcePatternFill {
                        preset: e.attribute("prst").map(enumeration).transpose()?,
                        foreground: None,
                        background: None,
                    }))
                }
                "blipFill" => {
                    attrs = &["dpi", "rotWithShape"];
                    fill(SourceFillDefinition::Image(SourceImageFill {
                        blip: None,
                        source_rect: None,
                        mode: None,
                        dpi: e
                            .attribute("dpi")
                            .map(|s| integer(Some(s), "fill dpi"))
                            .transpose()?,
                        rotate_with_shape: e.attribute("rotWithShape").map(boolean).transpose()?,
                    }))
                }
                "gsLst" => Node::Stops(SourceGradientStops {
                    source_ordinal: ordinal,
                    entries: Vec::new(),
                }),
                "gs" => {
                    attrs = &["pos"];
                    Node::Stop {
                        position: positive_fixed_percentage(required(e, "pos")?)?,
                        color: None,
                    }
                }
                "lin" => {
                    attrs = &["ang", "scaled"];
                    let angle: Option<u32> = e
                        .attribute("ang")
                        .map(|s| integer(Some(s), "gradient angle"))
                        .transpose()?;
                    if angle.is_some_and(|a| a >= 21_600_000) {
                        return Err(malformed("gradient angle outside range"));
                    }
                    Node::Shade(SourceGradientShade::Linear {
                        source_ordinal: ordinal,
                        angle,
                        scaled: e.attribute("scaled").map(boolean).transpose()?,
                    })
                }
                "path" => {
                    attrs = &["path"];
                    Node::Shade(SourceGradientShade::Path {
                        source_ordinal: ordinal,
                        path: e.attribute("path").map(enumeration).transpose()?,
                        fill_to_rect: None,
                    })
                }
                "tileRect" | "fillToRect" | "srcRect" | "fillRect" => {
                    attrs = &["l", "t", "r", "b"];
                    Node::Rect(SourceFillRect {
                        source_ordinal: ordinal,
                        left: e.attribute("l").map(any_percentage).transpose()?,
                        top: e.attribute("t").map(any_percentage).transpose()?,
                        right: e.attribute("r").map(any_percentage).transpose()?,
                        bottom: e.attribute("b").map(any_percentage).transpose()?,
                    })
                }
                "fgClr" | "bgClr" | "clrFrom" | "clrTo" => Node::ColorWrapper(None),
                "blip" => {
                    attrs = &["cstate"];
                    Node::Blip(SourceFillBlip {
                        effect_nodes: Vec::new(),
                        source_ordinal: ordinal,
                        embed: e
                            .attributes
                            .iter()
                            .find(|a| a.name.is(R, "embed"))
                            .map(|a| a.value.as_str())
                            .map(str::to_owned),
                        link: e
                            .attributes
                            .iter()
                            .find(|a| a.name.is(R, "link"))
                            .map(|a| a.value.as_str())
                            .map(str::to_owned),
                        compression: e.attribute("cstate").map(enumeration).transpose()?,
                        retained_ordinals: Vec::new(),
                    })
                }
                "tile" => {
                    attrs = &["tx", "ty", "sx", "sy", "flip", "algn"];
                    Node::Mode(SourceImageFillMode::Tile(SourceFillTile {
                        source_ordinal: ordinal,
                        translate_x: coord(e, "tx")?,
                        translate_y: coord(e, "ty")?,
                        scale_x: e.attribute("sx").map(any_percentage).transpose()?,
                        scale_y: e.attribute("sy").map(any_percentage).transpose()?,
                        flip: e.attribute("flip").map(enumeration).transpose()?,
                        alignment: e.attribute("algn").map(enumeration).transpose()?,
                    }))
                }
                "stretch" => Node::Mode(SourceImageFillMode::Stretch {
                    source_ordinal: ordinal,
                    fill_rect: None,
                }),
                "fillRef" | "bgRef" | "effectRef" => {
                    attrs = &["idx"];
                    Node::Reference(SourceFillReference {
                        source_ordinal: ordinal,
                        index: integer(e.attribute("idx"), "fill style index")?,
                        color: None,
                        retained_ordinals: Vec::new(),
                    })
                }
                "bg" => {
                    attrs = &["bwMode"];
                    Node::Background {
                        black_white_mode: e.attribute("bwMode").map(enumeration).transpose()?,
                        definition: None,
                    }
                }
                "bgPr" => {
                    attrs = &["shadeToTitle"];
                    Node::BackgroundProperties {
                        shade_to_title: e.attribute("shadeToTitle").map(boolean).transpose()?,
                        fill: None,
                        effects: None,
                        retained_ordinals: Vec::new(),
                    }
                }
                "srgbClr" | "scrgbClr" | "hslClr" | "sysClr" | "schemeClr" | "prstClr" => {
                    attrs = match name {
                        "scrgbClr" => &["r", "g", "b"],
                        "hslClr" => &["hue", "sat", "lum"],
                        "sysClr" => &["val", "lastClr"],
                        _ => &["val"],
                    };
                    Node::Color(color(e, ordinal)?)
                }
                _ => {
                    if !["comp", "inv", "gray", "gamma", "invGamma"].contains(&name) {
                        attrs = &["val"];
                    }
                    Node::Transform(transform(e)?)
                }
            }
        };
        attributes(e, attrs, ordinal, retained);
        Ok(Self {
            ordinal,
            value,
            name: name.into(),
            rank: 0,
            retained_start,
        })
    }
    pub fn recognized_opaque(&self, name: &ExpandedName) -> bool {
        match &self.value {
            Node::Blip(_) => {
                name.namespace == A
                    && (name.local == "extLst" || BLIP_EFFECTS.contains(&name.local.as_str()))
            }
            Node::BackgroundProperties { .. } => {
                name.is(P, "extLst") || name.is(A, "effectLst") || name.is(A, "effectDag")
            }
            Node::EffectStyle { .. } => name.is(A, "scene3d") || name.is(A, "sp3d"),
            _ => false,
        }
    }
    /// Native sequence rank and repetition rules are checked before retaining
    /// opaque effects, so source retention does not hide malformed containers.
    pub fn child(&mut self, name: &ExpandedName) -> Result<Child, XmlError> {
        let local = name.local.as_str();
        let color = name.namespace == A
            && [
                "srgbClr",
                "scrgbClr",
                "hslClr",
                "sysClr",
                "schemeClr",
                "prstClr",
            ]
            .contains(&local);
        let special = super::effect::child(&self.value, name)?;
        let (rank, repeat, mode) = if let Some(s) = special {
            s
        } else {
            match &self.value {
                Node::Background { .. } if name.is(P, "bgPr") || name.is(P, "bgRef") => {
                    (1, false, Child::Ordinary)
                }
                Node::BackgroundProperties { .. } if is_fill(name) => (1, false, Child::Ordinary),
                Node::BackgroundProperties { .. }
                    if name.is(A, "effectLst") || name.is(A, "effectDag") =>
                {
                    (
                        2,
                        false,
                        if name.is(A, "effectDag") {
                            Child::Effect
                        } else {
                            Child::Ordinary
                        },
                    )
                }
                Node::BackgroundProperties { .. } if name.is(P, "extLst") => {
                    (3, false, Child::Opaque)
                }
                Node::Reference(_) | Node::ColorWrapper(_) | Node::Stop { .. } if color => {
                    (1, false, Child::Ordinary)
                }
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Solid { .. },
                    ..
                }) if color => (1, false, Child::Ordinary),
                Node::Color(_) if name.namespace == A => (1, true, Child::Ordinary),
                Node::Stops(_) if name.is(A, "gs") => (1, true, Child::Ordinary),
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Gradient(_),
                    ..
                }) if name.namespace == A => match local {
                    "gsLst" => (1, false, Child::Ordinary),
                    "lin" | "path" => (2, false, Child::Ordinary),
                    "tileRect" => (3, false, Child::Ordinary),
                    _ => return Err(malformed("invalid gradient child")),
                },
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Pattern(_),
                    ..
                }) if name.namespace == A => match local {
                    "fgClr" => (1, false, Child::Ordinary),
                    "bgClr" => (2, false, Child::Ordinary),
                    _ => return Err(malformed("invalid pattern child")),
                },
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Image(_),
                    ..
                }) if name.namespace == A => match local {
                    "blip" => (1, false, Child::Ordinary),
                    "srcRect" => (2, false, Child::Ordinary),
                    "tile" | "stretch" => (3, false, Child::Ordinary),
                    _ => return Err(malformed("invalid image fill child")),
                },
                Node::Shade(SourceGradientShade::Path { .. }) if name.is(A, "fillToRect") => {
                    (1, false, Child::Ordinary)
                }
                Node::Mode(SourceImageFillMode::Stretch { .. }) if name.is(A, "fillRect") => {
                    (1, false, Child::Ordinary)
                }
                Node::Blip(_) if name.namespace == A && BLIP_EFFECTS.contains(&local) => {
                    (1, true, Child::Effect)
                }
                Node::Blip(_) if name.is(A, "extLst") => (2, false, Child::Opaque),
                _ => return Err(malformed("unexpected fill child or namespace")),
            }
        };
        if rank < self.rank || rank == self.rank && !repeat {
            return Err(malformed("duplicate or out of order fill property"));
        }
        self.rank = rank;
        Ok(mode)
    }
    pub fn close(
        mut self,
        retained: &mut Vec<u32>,
        nodes: &mut BTreeMap<u32, SourceEffectNode>,
    ) -> Result<Self, XmlError> {
        if matches!(
            self.value,
            Node::EffectDraft { .. } | Node::EffectList(_) | Node::EffectStyle { .. }
        ) {
            self.value = super::effect::finish(
                self.value,
                self.ordinal,
                &self.name,
                retained[self.retained_start..].to_vec(),
                nodes,
            )?;
            retained.truncate(self.retained_start);
        }
        match &mut self.value {
            Node::Fill(v) => v.retained_ordinals = retained[self.retained_start..].to_vec(),
            Node::Reference(v) => v.retained_ordinals = retained[self.retained_start..].to_vec(),
            Node::Blip(v) => v.retained_ordinals = retained[self.retained_start..].to_vec(),
            Node::BackgroundProperties {
                fill,
                retained_ordinals,
                ..
            } => {
                if fill.is_none() {
                    return Err(malformed("background properties missing fill"));
                }
                *retained_ordinals = retained[self.retained_start..].to_vec();
            }
            Node::Background {
                definition: None, ..
            } => return Err(malformed("background missing declaration")),
            Node::Stops(v) if v.entries.len() < 2 => {
                return Err(malformed("gradient requires at least two stops"));
            }
            Node::Stop { color: None, .. } | Node::ColorWrapper(None) => {
                return Err(malformed("missing fill color"));
            }
            _ => {}
        }
        Ok(self)
    }
    pub fn attach(&mut self, child: Frame) -> Result<(), XmlError> {
        let ordinal = child.ordinal;
        match (&mut self.value, child.value) {
            (Node::EffectDraft { slots, .. }, Node::Color(c))
            | (Node::EffectDraft { slots, .. }, Node::ColorWrapper(Some(c))) => {
                slots.colors.push(c)
            }
            (Node::EffectDraft { slots, .. }, Node::Fill(f)) => slots.fill = Some(Box::new(f)),
            (Node::EffectDraft { slots, .. }, Node::EffectLink(id)) => slots.nodes.push(id),
            (Node::EffectList(ids), Node::EffectLink(id)) => ids.push(id),
            (Node::Blip(blip), Node::EffectLink(id)) => blip.effect_nodes.push(id),
            (Node::BackgroundProperties { effects, .. }, Node::Effects(v))
            | (Node::EffectStyle { effects }, Node::Effects(v)) => *effects = Some(v),
            (Node::Reference(v), Node::Color(c)) => v.color = Some(c),
            (Node::ColorWrapper(v), Node::Color(c))
            | (Node::Stop { color: v, .. }, Node::Color(c)) => *v = Some(c),
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Solid { color },
                    ..
                }),
                Node::Color(c),
            ) => *color = Some(c),
            (Node::Color(c), Node::Transform(t)) => c.transforms.push(t),
            (
                Node::Stops(stops),
                Node::Stop {
                    position,
                    color: Some(color),
                },
            ) => stops.entries.push(SourceGradientStop {
                source_ordinal: ordinal,
                position,
                color,
            }),
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Gradient(g),
                    ..
                }),
                Node::Stops(stops),
            ) => g.stops = Some(stops),
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Gradient(g),
                    ..
                }),
                Node::Shade(shade),
            ) => g.shade = Some(shade),
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Gradient(g),
                    ..
                }),
                Node::Rect(rect),
            ) => g.tile_rect = Some(rect),
            (Node::Shade(SourceGradientShade::Path { fill_to_rect, .. }), Node::Rect(rect)) => {
                *fill_to_rect = Some(rect)
            }
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Pattern(p),
                    ..
                }),
                Node::ColorWrapper(Some(color)),
            ) => {
                let value = Some(SourceFillColor {
                    source_ordinal: ordinal,
                    color,
                });
                if child.name == "fgClr" {
                    p.foreground = value;
                } else {
                    p.background = value;
                }
            }
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Image(i),
                    ..
                }),
                Node::Blip(blip),
            ) => i.blip = Some(blip),
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Image(i),
                    ..
                }),
                Node::Rect(rect),
            ) => i.source_rect = Some(rect),
            (
                Node::Fill(SourceFill {
                    definition: SourceFillDefinition::Image(i),
                    ..
                }),
                Node::Mode(mode),
            ) => i.mode = Some(mode),
            (Node::Mode(SourceImageFillMode::Stretch { fill_rect, .. }), Node::Rect(rect)) => {
                *fill_rect = Some(rect)
            }
            (Node::Background { definition, .. }, Node::Reference(reference)) => {
                *definition = Some(SourceBackgroundDefinition::Reference(reference))
            }
            (
                Node::Background { definition, .. },
                Node::BackgroundProperties {
                    shade_to_title,
                    fill: Some(fill),
                    effects,
                    retained_ordinals,
                },
            ) => {
                *definition = Some(SourceBackgroundDefinition::Properties {
                    source_ordinal: ordinal,
                    shade_to_title,
                    fill: Box::new(fill),
                    effects,
                    retained_ordinals,
                })
            }
            (Node::BackgroundProperties { fill, .. }, Node::Fill(value)) => *fill = Some(value),
            _ => return Err(malformed("invalid completed fill child")),
        }
        Ok(())
    }
}
const BLIP_EFFECTS: &[&str] = &[
    "alphaBiLevel",
    "alphaCeiling",
    "alphaFloor",
    "alphaInv",
    "alphaMod",
    "alphaModFix",
    "alphaRepl",
    "biLevel",
    "blur",
    "clrChange",
    "clrRepl",
    "duotone",
    "fillOverlay",
    "grayscl",
    "hsl",
    "lum",
    "tint",
];
