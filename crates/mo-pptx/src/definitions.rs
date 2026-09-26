use crate::source::theme::*;
use crate::{A, P, PptxError, R, drawing::Drawing, native, xml::Xml};
use mo_common::{LayoutId, MasterId};
use mo_opc::PartName;
use mo_presentation_model::*;

pub(crate) fn root(x: &mut Xml, tag: &str) -> Result<(), PptxError> {
    x.raw("<p:")?;
    x.raw(tag)?;
    x.attr("xmlns:p", P)?;
    x.attr("xmlns:a", A)?;
    x.attr("xmlns:r", R)
}
pub(crate) fn theme(theme: &SourceThemePart, max: usize) -> Result<Vec<u8>, PptxError> {
    let mut x = Xml::new(max)?;
    x.raw("<a:theme")?;
    x.attr("xmlns:a", A)?;
    x.attr("name", theme.name.as_deref().unwrap_or(""))?;
    x.raw("><a:themeElements><a:clrScheme")?;
    let colors = theme.color_scheme.as_ref().expect("planned theme colors");
    x.attr("name", &colors.name)?;
    x.raw(">")?;
    for (_, name) in mo_presentation_source::author::COLOR_SLOTS {
        let slot: ColorSlot = serde::Deserialize::deserialize(serde::de::value::StrDeserializer::<
            serde::de::value::Error,
        >::new(name))
        .expect("native color slot");
        x.raw("<a:")?;
        x.raw(name)?;
        x.raw(">")?;
        native::color(&mut x, &colors.colors[&slot])?;
        x.raw("</a:")?;
        x.raw(name)?;
        x.raw(">")?;
    }
    x.raw("</a:clrScheme><a:fontScheme")?;
    let fonts = theme.font_scheme.as_ref().expect("planned theme fonts");
    x.attr("name", &fonts.name)?;
    x.raw(">")?;
    for (tag, fonts) in [("majorFont", &fonts.major), ("minorFont", &fonts.minor)] {
        x.raw("<a:")?;
        x.raw(tag)?;
        x.raw(">")?;
        for (script, font) in [
            ("latin", &fonts.latin),
            ("ea", &fonts.east_asian),
            ("cs", &fonts.complex_script),
        ] {
            if let Some(font) = font {
                native::font(&mut x, script, font)?;
            }
        }
        x.raw("</a:")?;
        x.raw(tag)?;
        x.raw(">")?;
    }
    let format = theme.format_scheme.as_ref().expect("planned theme format");
    x.raw("</a:fontScheme><a:fmtScheme")?;
    x.attr("name", format.name.as_deref().unwrap_or(""))?;
    x.raw(">")?;
    for (tag, styles) in [
        ("fillStyleLst", &format.fills),
        ("lnStyleLst", &format.lines),
        ("effectStyleLst", &format.effects),
        ("bgFillStyleLst", &format.background_fills),
    ] {
        x.raw("<a:")?;
        x.raw(tag)?;
        x.raw(">")?;
        for style in styles {
            if let Some(fill) = &style.fill {
                native::fill(&mut x, fill)?;
            }
            if let Some(line) = &style.line {
                native::line(&mut x, line)?;
            }
            if let Some(effect) = &style.effect_style {
                if !effect.effects.is_explicitly_empty_list() {
                    return Err(crate::value("author plan", "theme effects"));
                }
                x.raw("<a:effectStyle><a:effectLst/></a:effectStyle>")?;
            }
        }
        x.raw("</a:")?;
        x.raw(tag)?;
        x.raw(">")?;
    }
    x.raw("</a:fmtScheme></a:themeElements></a:theme>")?;
    Ok(x.finish())
}

pub(crate) fn master(
    ctx: &mut Drawing<'_>,
    id: Option<&MasterId>,
    theme: &PartName,
    layouts: &[(u32, PartName)],
    max: usize,
) -> Result<Vec<u8>, PptxError> {
    ctx.relationship("theme", theme)?;
    let mut x = Xml::new(max)?;
    root(&mut x, "sldMaster")?;
    x.raw("><p:cSld")?;
    x.attr("name", id.map_or("MusterOffice default", |id| id.as_str()))?;
    x.raw(">")?;
    let master = id.map(|id| &ctx.document.masters[id]);
    native::background(&mut x, &ctx.surface().background)?;
    ctx.tree(&mut x, master.map_or(&[], |m| m.objects.as_slice()))?;
    x.raw("</p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:sldLayoutIdLst>")?;
    for (native_id, path) in layouts {
        let rel = ctx.relationship("slideLayout", path)?;
        x.raw("<p:sldLayoutId")?;
        x.attr("id", native_id)?;
        x.attr("r:id", rel)?;
        x.raw("/>")?;
    }
    x.raw("</p:sldLayoutIdLst>")?;
    let catalog = &ctx.surface().text;
    for root in catalog.roots.iter().filter(|r| r.owner.is_none()) {
        native::text(&mut x, catalog, root.source_ordinal, &[])?;
    }
    x.raw("</p:sldMaster>")?;
    Ok(x.finish())
}

pub(crate) fn layout(
    ctx: &mut Drawing<'_>,
    id: Option<&LayoutId>,
    master: &PartName,
    max: usize,
) -> Result<Vec<u8>, PptxError> {
    ctx.relationship("slideMaster", master)?;
    let layout = id.map(|id| &ctx.document.layouts[id]);
    let mut x = Xml::new(max)?;
    root(&mut x, "sldLayout")?;
    x.raw(" type=\"blank\" preserve=\"1\"><p:cSld")?;
    x.attr("name", layout.map_or("Blank", |l| l.name.as_str()))?;
    x.raw(">")?;
    native::background(&mut x, &ctx.surface().background)?;
    ctx.tree(&mut x, layout.map_or(&[], |l| l.objects.as_slice()))?;
    x.raw("</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>")?;
    Ok(x.finish())
}

pub(crate) fn slide(
    ctx: &mut Drawing<'_>,
    slide: &Slide,
    layout: &PartName,
    max: usize,
) -> Result<Vec<u8>, PptxError> {
    ctx.relationship("slideLayout", layout)?;
    let mut x = Xml::new(max)?;
    root(&mut x, "sld")?;
    x.attr("show", u8::from(!slide.hidden))?;
    x.raw("><p:cSld")?;
    x.attr("name", &slide.name)?;
    x.raw(">")?;
    native::background(&mut x, &ctx.surface().background)?;
    ctx.tree(&mut x, &slide.objects)?;
    x.raw("</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>")?;
    if let Some(timeline) = ctx.document.timelines.get(&slide.id) {
        crate::timing::write(&mut x, timeline, ctx.object_ids, ctx.check)?;
    }
    x.raw("</p:sld>")?;
    Ok(x.finish())
}
