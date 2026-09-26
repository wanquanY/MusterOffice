use crate::{ExportDefaults, PptxError, value, xml::Xml};
use mo_common::Emu;
use mo_presentation_model::*;

pub(crate) fn centipoints(value: Emu, path: &str, min: i64, max: i64) -> Result<i64, PptxError> {
    if value.get() % 127 != 0 || !(min..=max).contains(&(value.get() / 127)) {
        return Err(crate::value(
            path,
            "value is not exactly representable in native hundredths of a point",
        ));
    }
    Ok(value.get() / 127)
}

pub(crate) fn properties(
    x: &mut Xml,
    tag: &str,
    style: &CharacterStyle,
    document: &Document,
) -> Result<(), PptxError> {
    x.raw("<")?;
    x.raw(tag)?;
    if let Inherited::Value(size) = style.size {
        x.attr("sz", centipoints(size, "font size", 100, 400000)?)?;
    }
    if let Inherited::Value(b) = style.bold {
        x.attr("b", u8::from(b))?;
    }
    if let Inherited::Value(i) = style.italic {
        x.attr("i", u8::from(i))?;
    }
    if let Inherited::Value(u) = style.underline {
        x.attr("u", if u { "sng" } else { "none" })?;
    }
    if let Inherited::Value(lang) = &style.language {
        x.attr("lang", lang)?;
    }
    x.raw(">")?;
    if let Inherited::Value(color) = &style.color {
        crate::drawing::fill(
            x,
            &Fill::Solid {
                color: color.clone(),
            },
        )?;
    }
    if let Inherited::Value(font) = &style.font {
        fonts(x, &document.fonts[font].family)?;
    }
    x.raw("</")?;
    x.raw(tag)?;
    x.raw(">")
}

pub(crate) fn fonts(x: &mut Xml, family: &str) -> Result<(), PptxError> {
    if family.trim().is_empty() {
        return Err(value("fontFamily", "explicit font family is required"));
    }
    for script in ["latin", "ea", "cs"] {
        x.raw("<a:")?;
        x.raw(script)?;
        x.attr("typeface", family)?;
        x.raw("/>")?;
    }
    Ok(())
}

pub(crate) fn defaults(x: &mut Xml, defaults: &ExportDefaults) -> Result<(), PptxError> {
    // Author paragraphs currently have native level zero. Office ignores
    // defPPr, so defaults must participate through the matching level style.
    x.raw("<a:lvl1pPr><a:defRPr")?;
    x.attr(
        "sz",
        centipoints(defaults.text_size, "defaults.textSize", 100, 400000)?,
    )?;
    x.raw(">")?;
    crate::drawing::fill(
        x,
        &Fill::Solid {
            color: Color::Srgb {
                rgba: defaults.text_color,
            },
        },
    )?;
    fonts(x, &defaults.font_family)?;
    x.raw("</a:defRPr></a:lvl1pPr>")
}

pub(crate) fn body(x: &mut Xml, body: &TextBody, document: &Document) -> Result<(), PptxError> {
    x.raw("<p:txBody><a:bodyPr")?;
    for (name, emu) in [
        ("lIns", body.insets.left),
        ("tIns", body.insets.top),
        ("rIns", body.insets.right),
        ("bIns", body.insets.bottom),
    ] {
        if emu.get() > i32::MAX as i64 {
            return Err(value(name, "text inset exceeds native range"));
        }
        x.attr(name, emu.get())?;
    }
    x.attr("wrap", if body.wrap { "square" } else { "none" })?;
    x.attr(
        "vertOverflow",
        if body.overflow == OverflowPolicy::Clip {
            "clip"
        } else {
            "overflow"
        },
    )?;
    x.attr(
        "horzOverflow",
        if body.overflow == OverflowPolicy::Clip {
            "clip"
        } else {
            "overflow"
        },
    )?;
    let mut vertical = None;
    for p in &body.paragraphs {
        let direction = match p.style.direction {
            Inherited::Value(TextDirection::VerticalRightToLeft) => "eaVert",
            Inherited::Value(TextDirection::VerticalLeftToRight) => "mongolianVert",
            _ => "horz",
        };
        if vertical.is_some_and(|v| v != direction) {
            return Err(PptxError::Unsupported(
                "mixed paragraph writing modes in one native text body".into(),
            ));
        }
        vertical = Some(direction);
    }
    x.attr("vert", vertical.unwrap_or("horz"))?;
    x.raw(">")?;
    x.raw(if body.overflow == OverflowPolicy::GrowShape {
        "<a:spAutoFit/>"
    } else {
        "<a:noAutofit/>"
    })?;
    x.raw("</a:bodyPr><a:lstStyle><a:lvl1pPr>")?;
    properties(x, "a:defRPr", &body.style, document)?;
    x.raw("</a:lvl1pPr></a:lstStyle>")?;
    for paragraph in &body.paragraphs {
        x.raw("<a:p><a:pPr")?;
        let rtl = matches!(
            paragraph.style.direction,
            Inherited::Value(TextDirection::RightToLeft)
        );
        if let Inherited::Value(direction) = paragraph.style.direction
            && matches!(
                direction,
                TextDirection::LeftToRight | TextDirection::RightToLeft
            )
        {
            x.attr("rtl", u8::from(rtl))?;
        }
        if let Inherited::Value(alignment) = paragraph.style.alignment {
            x.attr(
                "algn",
                match alignment {
                    Alignment::Start => {
                        if rtl {
                            "r"
                        } else {
                            "l"
                        }
                    }
                    Alignment::End => {
                        if rtl {
                            "l"
                        } else {
                            "r"
                        }
                    }
                    Alignment::Center => "ctr",
                    Alignment::Justify => "just",
                },
            )?;
        }
        x.raw(">")?;
        for (tag, spacing) in [
            ("spcBef", &paragraph.style.space_before),
            ("spcAft", &paragraph.style.space_after),
        ] {
            if let Inherited::Value(emu) = spacing {
                x.raw("<a:")?;
                x.raw(tag)?;
                x.raw("><a:spcPts")?;
                x.attr("val", centipoints(*emu, tag, 0, 158400)?)?;
                x.raw("/></a:")?;
                x.raw(tag)?;
                x.raw(">")?;
            }
        }
        properties(x, "a:defRPr", &paragraph.default_run_style, document)?;
        x.raw("</a:pPr>")?;
        for run in &paragraph.runs {
            match &run.content {
                InlineContent::Break => {
                    x.raw("<a:br>")?;
                    properties(x, "a:rPr", &run.style, document)?;
                    x.raw("</a:br>")?;
                }
                InlineContent::Text { text } => {
                    x.raw("<a:r>")?;
                    properties(x, "a:rPr", &run.style, document)?;
                    x.raw("<a:t>")?;
                    x.text(text)?;
                    x.raw("</a:t></a:r>")?;
                }
                InlineContent::Tab => {
                    x.raw("<a:r>")?;
                    properties(x, "a:rPr", &run.style, document)?;
                    x.raw("<a:t>\t</a:t></a:r>")?;
                }
            }
        }
        x.raw("</a:p>")?;
    }
    x.raw("</p:txBody>")
}
