use super::*;
use crate::source::{SourceRun, SourceRunKind, drawingml::SourceTextFont, text::*};
use NativeTextElement as N;

pub(crate) fn font(x: &mut Xml, tag: &str, value: &SourceTextFont) -> Result<(), PptxError> {
    x.raw("<a:")?;
    x.raw(tag)?;
    font_attributes(x, value)?;
    x.raw("/>")
}
fn font_attributes(x: &mut Xml, value: &SourceTextFont) -> Result<(), PptxError> {
    x.attr("typeface", &value.typeface)?;
    attribute(x, "panose", &value.panose)?;
    attribute(x, "pitchFamily", &value.pitch_family)?;
    attribute(x, "charset", &value.charset)
}
pub(crate) fn text(
    x: &mut Xml,
    catalog: &SourceTextCatalog,
    root: u32,
    paragraphs: &[Vec<SourceRun>],
) -> Result<(), PptxError> {
    let mut leaves = paragraphs
        .iter()
        .flatten()
        .filter(|r| r.kind == SourceRunKind::Text);
    let mut pending = vec![(root, false)];
    while let Some((id, closing)) = pending.pop() {
        let node = catalog.nodes.get(&id).ok_or_else(unexpected)?;
        let namespace = if matches!(
            node.element,
            N::TxBody
                | N::TxStyles
                | N::DefaultTextStyle
                | N::TitleStyle
                | N::BodyStyle
                | N::OtherStyle
        ) {
            "p:"
        } else {
            "a:"
        };
        let tag = lexical(&node.element)?;
        if closing {
            x.raw("</")?;
            x.raw(namespace)?;
            x.raw(&tag)?;
            x.raw(">")?;
            continue;
        }
        if let SourceTextValue::Fill { fill: f } = &node.value {
            fill(x, f)?;
            continue;
        }
        x.raw("<")?;
        x.raw(namespace)?;
        x.raw(&tag)?;
        match &node.value {
            SourceTextValue::Container {} => (),
            SourceTextValue::Body { attributes: a } => {
                attribute(x, "lIns", &a.left_inset)?;
                attribute(x, "tIns", &a.top_inset)?;
                attribute(x, "rIns", &a.right_inset)?;
                attribute(x, "bIns", &a.bottom_inset)?;
                attribute(x, "wrap", &a.wrap)?;
                attribute(x, "vertOverflow", &a.vertical_overflow)?;
                attribute(x, "horzOverflow", &a.horizontal_overflow)?;
                attribute(x, "vert", &a.vertical)?;
            }
            SourceTextValue::Paragraph { attributes: a } => {
                attribute(x, "rtl", &a.right_to_left)?;
                attribute(x, "algn", &a.alignment)?;
            }
            SourceTextValue::Character { attributes: a } => {
                attribute(x, "sz", &a.size)?;
                attribute(x, "b", &a.bold)?;
                attribute(x, "i", &a.italic)?;
                attribute(x, "u", &a.underline)?;
                attribute(x, "lang", &a.language)?;
            }
            SourceTextValue::Font { font } => font_attributes(x, font)?,
            SourceTextValue::Points { value } => x.attr("val", value)?,
            _ => return Err(unexpected()),
        }
        if node.element == N::T {
            x.raw(">")?;
            x.text(&leaves.next().ok_or_else(unexpected)?.text)?;
            x.raw("</a:t>")?;
        } else if node.children.is_empty()
            && matches!(
                node.element,
                N::NoAutofit | N::SpAutoFit | N::Latin | N::Ea | N::Cs | N::SpcPts
            )
        {
            x.raw("/>")?;
        } else {
            x.raw(">")?;
            pending.push((id, true));
            pending.extend(node.children.iter().rev().map(|id| (*id, false)));
        }
    }
    if leaves.next().is_some() {
        return Err(unexpected());
    }
    Ok(())
}
