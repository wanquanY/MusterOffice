use super::*;
use crate::{
    A, P,
    source::{
        boolean,
        drawingml::{enumeration, required},
        malformed,
    },
};
use NativeTextElement as N;
use mo_xml::{Element, XmlError};

pub(super) fn node(
    e: &Element,
    ordinal: u32,
    parent: Option<u32>,
) -> Result<SourceTextNode, XmlError> {
    let element: N = enumeration(&e.name.local)?;
    if e.name.namespace
        != if matches!(
            element,
            N::TxBody
                | N::TxStyles
                | N::DefaultTextStyle
                | N::TitleStyle
                | N::BodyStyle
                | N::OtherStyle
        ) {
            P
        } else {
            A
        }
    {
        return Err(malformed("native text namespace"));
    }
    let mut allowed: &[&str] = &[];
    let value = match element {
        N::BodyPr => {
            allowed = SourceTextBodyAttributes::NAMES;
            SourceTextValue::Body {
                attributes: SourceTextBodyAttributes::read(e)?.into(),
            }
        }
        N::PPr
        | N::DefPPr
        | N::Lvl1pPr
        | N::Lvl2pPr
        | N::Lvl3pPr
        | N::Lvl4pPr
        | N::Lvl5pPr
        | N::Lvl6pPr
        | N::Lvl7pPr
        | N::Lvl8pPr
        | N::Lvl9pPr => {
            allowed = SourceTextParagraphAttributes::NAMES;
            SourceTextValue::Paragraph {
                attributes: SourceTextParagraphAttributes::read(e)?.into(),
            }
        }
        N::RPr | N::DefRPr | N::EndParaRPr => {
            allowed = SourceTextCharacterAttributes::NAMES;
            SourceTextValue::Character {
                attributes: SourceTextCharacterAttributes::read(e)?.into(),
            }
        }
        N::Latin | N::Ea | N::Cs | N::Sym | N::BuFont => {
            allowed = &["typeface", "panose", "pitchFamily", "charset"];
            SourceTextValue::Font {
                font: super::super::drawingml::text_font(e)?.into(),
            }
        }
        N::FontRef => {
            allowed = &["idx"];
            SourceTextValue::FontReference {
                index: enumeration(required(e, "idx")?)?,
            }
        }
        N::NormAutofit => {
            allowed = &["fontScale", "lnSpcReduction"];
            SourceTextValue::Autofit {
                font_scale: e
                    .attribute("fontScale")
                    .map(|s| attributes::percentage_range(s, 1000, 100000))
                    .transpose()?,
                line_spacing_reduction: e
                    .attribute("lnSpcReduction")
                    .map(|s| attributes::percentage_range(s, 0, 13200000))
                    .transpose()?,
            }
        }
        N::SpcPct | N::BuSzPct => {
            allowed = &["val"];
            let s = required(e, "val")?;
            let value = if element == N::BuSzPct {
                attributes::percentage_range(s, 25000, 400000)?
            } else {
                attributes::percentage_range(s, 0, 13200000)?
            };
            SourceTextValue::Percentage { value }
        }
        N::SpcPts | N::BuSzPts => {
            allowed = &["val"];
            let (lo, hi) = if element == N::BuSzPts {
                (100, 400000)
            } else {
                (0, 158400)
            };
            SourceTextValue::Points {
                value: attributes::range(required(e, "val")?, lo, hi)?,
            }
        }
        N::BuAutoNum => {
            allowed = &["type", "startAt"];
            SourceTextValue::AutoNumber {
                scheme: enumeration(required(e, "type")?)?,
                start_at: e
                    .attribute("startAt")
                    .map(|s| attributes::range(s, 1, 32767).map(|n| n as u16))
                    .transpose()?,
            }
        }
        N::BuChar => {
            allowed = &["char"];
            SourceTextValue::BulletCharacter {
                character: required(e, "char")?.into(),
            }
        }
        N::Tab => {
            allowed = &["pos", "algn"];
            SourceTextValue::Tab {
                position: e
                    .attribute("pos")
                    .map(attributes::coordinate32)
                    .transpose()?,
                alignment: e.attribute("algn").map(enumeration).transpose()?,
            }
        }
        N::Fld => {
            allowed = &["id", "type"];
            SourceTextValue::Field {
                id: required(e, "id")?.into(),
                field_type: e.attribute("type").map(str::to_owned),
            }
        }
        N::HlinkClick | N::HlinkMouseOver => {
            allowed = SourceTextHyperlinkAttributes::NAMES;
            SourceTextValue::Hyperlink {
                attributes: SourceTextHyperlinkAttributes::read(e)?.into(),
            }
        }
        N::Rtl => {
            allowed = &["val"];
            SourceTextValue::RightToLeft {
                value: e
                    .attribute("val")
                    .map(|s| match s.trim() {
                        "on" => Ok(true),
                        "off" => Ok(false),
                        s => boolean(s),
                    })
                    .transpose()?,
            }
        }
        _ => SourceTextValue::Container {},
    };
    let mut retained_ordinals = vec![];
    retain_attributes(e, allowed, ordinal, &mut retained_ordinals);
    Ok(SourceTextNode {
        element,
        parent,
        children: vec![],
        value,
        retained_ordinals,
    })
}
pub(super) fn retain_attributes(
    e: &Element,
    allowed: &[&str],
    ordinal: u32,
    retained: &mut Vec<u32>,
) {
    if e.attributes.iter().any(|a| {
        if a.name.namespace == "http://schemas.openxmlformats.org/markup-compatibility/2006" {
            return false;
        }
        if a.name.namespace == crate::R {
            return !allowed
                .iter()
                .any(|n| n.strip_prefix("r:") == Some(a.name.local.as_str()));
        }
        !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str())
    }) {
        retain(retained, ordinal);
    }
}
pub(super) fn retain(values: &mut Vec<u32>, ordinal: u32) {
    if values.last() != Some(&ordinal) {
        values.push(ordinal);
    }
}

pub(super) struct Child {
    pub rank: u8,
    pub repeated: bool,
    pub opaque: bool,
}
pub(super) fn child(parent: N, e: &Element) -> Result<Child, XmlError> {
    let local = e.name.local.as_str();
    let ns = if matches!(parent, N::TxStyles) { P } else { A };
    if e.name.namespace != ns {
        return Ok(Child {
            rank: 0,
            repeated: true,
            opaque: true,
        });
    }
    let (rank, repeated, opaque) = match parent {
        N::TxBody => match local {
            "bodyPr" => (1, false, false),
            "lstStyle" => (2, false, false),
            "p" => (3, true, false),
            _ => (0, true, true),
        },
        N::TxStyles => match local {
            "titleStyle" => (1, false, false),
            "bodyStyle" => (2, false, false),
            "otherStyle" => (3, false, false),
            "extLst" => (4, false, true),
            _ => (0, true, true),
        },
        N::DefaultTextStyle | N::TitleStyle | N::BodyStyle | N::OtherStyle | N::LstStyle => {
            match local {
                "defPPr" => (1, false, false),
                "lvl1pPr" => (2, false, false),
                "lvl2pPr" => (3, false, false),
                "lvl3pPr" => (4, false, false),
                "lvl4pPr" => (5, false, false),
                "lvl5pPr" => (6, false, false),
                "lvl6pPr" => (7, false, false),
                "lvl7pPr" => (8, false, false),
                "lvl8pPr" => (9, false, false),
                "lvl9pPr" => (10, false, false),
                "extLst" => (11, false, true),
                _ => (0, true, true),
            }
        }
        N::BodyPr => match local {
            "prstTxWarp" => (1, false, true),
            "noAutofit" | "normAutofit" | "spAutoFit" => (2, false, false),
            "scene3d" => (3, false, true),
            "sp3d" | "flatTx" => (4, false, true),
            "extLst" => (5, false, true),
            _ => (0, true, true),
        },
        N::P => match local {
            "pPr" => (1, false, false),
            "r" | "br" | "fld" => (2, true, false),
            "endParaRPr" => (3, false, false),
            _ => (0, true, true),
        },
        N::PPr
        | N::DefPPr
        | N::Lvl1pPr
        | N::Lvl2pPr
        | N::Lvl3pPr
        | N::Lvl4pPr
        | N::Lvl5pPr
        | N::Lvl6pPr
        | N::Lvl7pPr
        | N::Lvl8pPr
        | N::Lvl9pPr => match local {
            "lnSpc" => (1, false, false),
            "spcBef" => (2, false, false),
            "spcAft" => (3, false, false),
            "buClrTx" | "buClr" => (4, false, false),
            "buSzTx" | "buSzPct" | "buSzPts" => (5, false, false),
            "buFontTx" | "buFont" => (6, false, false),
            "buNone" | "buAutoNum" | "buChar" => (7, false, false),
            "buBlip" => (7, false, true),
            "tabLst" => (8, false, false),
            "defRPr" => (9, false, false),
            "extLst" => (10, false, true),
            _ => (0, true, true),
        },
        N::R | N::Br => match local {
            "rPr" => (1, false, false),
            "t" if parent == N::R => (2, false, false),
            _ => (0, true, true),
        },
        N::Fld => match local {
            "rPr" => (1, false, false),
            "pPr" => (2, false, false),
            "t" => (3, false, false),
            _ => (0, true, true),
        },
        N::RPr | N::DefRPr | N::EndParaRPr => match local {
            "ln" => (1, false, false),
            "noFill" | "solidFill" | "gradFill" | "blipFill" | "pattFill" | "grpFill" => {
                (2, false, false)
            }
            "effectLst" | "effectDag" => (3, false, false),
            "highlight" => (4, false, false),
            "uLnTx" | "uLn" => (5, false, false),
            "uFillTx" | "uFill" => (6, false, false),
            "latin" => (7, false, false),
            "ea" => (8, false, false),
            "cs" => (9, false, false),
            "sym" => (10, false, false),
            "hlinkClick" => (11, false, false),
            "hlinkMouseOver" => (12, false, false),
            "rtl" => (13, false, false),
            "extLst" => (14, false, true),
            _ => (0, true, true),
        },
        N::LnSpc | N::SpcBef | N::SpcAft => match local {
            "spcPct" | "spcPts" => (1, false, false),
            _ => (0, true, true),
        },
        N::TabLst => match local {
            "tab" => (1, true, false),
            _ => (0, true, true),
        },
        N::BuClr | N::Highlight | N::FontRef => match local {
            "srgbClr" | "scrgbClr" | "hslClr" | "sysClr" | "schemeClr" | "prstClr" => {
                (1, false, false)
            }
            _ => (0, true, true),
        },
        N::UFill => {
            if super::super::fill::is_fill(&e.name) {
                (1, false, false)
            } else {
                (0, true, true)
            }
        }
        N::HlinkClick | N::HlinkMouseOver => match local {
            "snd" => (1, false, true),
            "extLst" => (2, false, true),
            _ => (0, true, true),
        },
        _ => (0, true, true),
    };
    Ok(Child {
        rank,
        repeated,
        opaque,
    })
}
pub(super) fn close(
    node: &SourceTextNode,
    nodes: &BTreeMap<u32, SourceTextNode>,
) -> Result<(), XmlError> {
    let children: Vec<_> = node.children.iter().map(|id| nodes[id].element).collect();
    let missing = match node.element {
        N::TxBody => !children.contains(&N::BodyPr) || !children.contains(&N::P),
        N::R => !children.contains(&N::T),
        N::LnSpc | N::SpcBef | N::SpcAft | N::BuClr | N::Highlight | N::UFill => {
            children.is_empty() && node.retained_ordinals.is_empty()
        }
        _ => false,
    };
    if missing {
        return Err(malformed("missing required native text child"));
    }
    if node.element == N::TabLst && children.len() > 32 {
        return Err(malformed("too many native tab stops"));
    }
    Ok(())
}
