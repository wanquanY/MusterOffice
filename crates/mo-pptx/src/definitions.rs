use crate::{
    A, COLOR_SLOTS, ExportDefaults, P, PptxError, R,
    drawing::{self, Drawing},
    text,
    xml::Xml,
};
use mo_common::{LayoutId, MasterId, ThemeId};
use mo_opc::PartName;
use mo_presentation_model::*;

pub(crate) fn root(x: &mut Xml, tag: &str) -> Result<(), PptxError> {
    x.raw("<p:")?;
    x.raw(tag)?;
    x.attr("xmlns:p", P)?;
    x.attr("xmlns:a", A)?;
    x.attr("xmlns:r", R)
}
fn background(x: &mut Xml, bg: &Inherited<Fill>) -> Result<(), PptxError> {
    if let Inherited::Value(f) = bg {
        x.raw("<p:bg><p:bgPr>")?;
        drawing::fill(x, f)?;
        x.raw("<a:effectLst/></p:bgPr></p:bg>")?;
    }
    Ok(())
}
pub(crate) fn theme(
    document: &Document,
    id: Option<&ThemeId>,
    defaults: &ExportDefaults,
    max: usize,
) -> Result<Vec<u8>, PptxError> {
    let mut x = Xml::new(max)?;
    let theme = id.map(|id| &document.themes[id]);
    let name = theme.map_or("MusterOffice", |t| t.name.as_str());
    let mut family = defaults.font_family.as_str();
    if let Some(theme) = theme {
        let mut unsupported = theme.default_text.clone();
        unsupported.font = Inherited::Inherit;
        if unsupported != CharacterStyle::default() {
            return Err(PptxError::Unsupported(
                "theme-wide character defaults other than font need native style binding".into(),
            ));
        }
        if let Inherited::Value(font) = &theme.default_text.font {
            family = &document.fonts[font].family;
        }
    }
    x.raw("<a:theme")?;
    x.attr("xmlns:a", A)?;
    x.attr("name", name)?;
    x.raw("><a:themeElements><a:clrScheme")?;
    x.attr("name", name)?;
    x.raw(">")?;
    for (slot, native) in COLOR_SLOTS {
        let rgba = theme
            .and_then(|t| t.colors.get(&slot))
            .unwrap_or(&defaults.theme_colors[&slot]);
        x.raw("<a:")?;
        x.raw(native)?;
        x.raw(">")?;
        drawing::color(&mut x, &Color::Srgb { rgba: *rgba })?;
        x.raw("</a:")?;
        x.raw(native)?;
        x.raw(">")?;
    }
    x.raw("</a:clrScheme><a:fontScheme")?;
    x.attr("name", name)?;
    x.raw(">")?;
    for font in ["majorFont", "minorFont"] {
        x.raw("<a:")?;
        x.raw(font)?;
        x.raw(">")?;
        text::fonts(&mut x, family)?;
        x.raw("</a:")?;
        x.raw(font)?;
        x.raw(">")?;
    }
    x.raw("</a:fontScheme><a:fmtScheme name=\"MusterOffice\"><a:fillStyleLst>")?;
    for _ in 0..3 {
        x.raw("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>")?;
    }
    x.raw("</a:fillStyleLst><a:lnStyleLst>")?;
    for width in [12700, 25400, 38100] {
        x.raw("<a:ln")?;
        x.attr("w", width)?;
        x.raw("><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/></a:ln>")?;
    }
    x.raw("</a:lnStyleLst><a:effectStyleLst>")?;
    for _ in 0..3 {
        x.raw("<a:effectStyle><a:effectLst/></a:effectStyle>")?;
    }
    x.raw("</a:effectStyleLst><a:bgFillStyleLst>")?;
    for _ in 0..3 {
        x.raw("<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>")?;
    }
    x.raw("</a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>")?;
    Ok(x.finish())
}

pub(crate) fn master(
    ctx: &mut Drawing<'_>,
    id: Option<&MasterId>,
    theme: &PartName,
    layouts: &[(u32, PartName)],
    defaults: &ExportDefaults,
    max: usize,
) -> Result<Vec<u8>, PptxError> {
    ctx.relationship("theme", theme)?;
    let mut x = Xml::new(max)?;
    root(&mut x, "sldMaster")?;
    x.raw("><p:cSld")?;
    x.attr("name", id.map_or("MusterOffice default", |id| id.as_str()))?;
    x.raw(">")?;
    let master = id.map(|id| &ctx.document.masters[id]);
    background(
        &mut x,
        &master.map_or(
            Inherited::Value(Fill::Solid {
                color: Color::Srgb {
                    rgba: defaults.page_background,
                },
            }),
            |m| m.background.clone(),
        ),
    )?;
    ctx.tree(&mut x, master.map_or(&[], |m| m.objects.as_slice()))?;
    x.raw("</p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/><p:sldLayoutIdLst>")?;
    for (native_id, path) in layouts {
        let rel = ctx.relationship("slideLayout", path)?;
        x.raw("<p:sldLayoutId")?;
        x.attr("id", native_id)?;
        x.attr("r:id", rel)?;
        x.raw("/>")?;
    }
    x.raw("</p:sldLayoutIdLst><p:txStyles>")?;
    for tag in ["titleStyle", "bodyStyle", "otherStyle"] {
        x.raw("<p:")?;
        x.raw(tag)?;
        x.raw(">")?;
        if let Some(master) = master {
            x.raw("<a:lvl1pPr>")?;
            text::properties(&mut x, "a:defRPr", &master.default_text, ctx.document)?;
            x.raw("</a:lvl1pPr>")?;
        }
        x.raw("</p:")?;
        x.raw(tag)?;
        x.raw(">")?;
    }
    x.raw("</p:txStyles></p:sldMaster>")?;
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
    if layout.is_some_and(|l| l.default_text != CharacterStyle::default()) {
        return Err(PptxError::Unsupported(
            "layout text defaults require native placeholder bindings".into(),
        ));
    }
    let mut x = Xml::new(max)?;
    root(&mut x, "sldLayout")?;
    x.raw(" type=\"blank\" preserve=\"1\"><p:cSld")?;
    x.attr("name", layout.map_or("Blank", |l| l.name.as_str()))?;
    x.raw(">")?;
    if let Some(layout) = layout {
        background(&mut x, &layout.background)?;
    }
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
    background(&mut x, &slide.background)?;
    ctx.tree(&mut x, &slide.objects)?;
    x.raw("</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>")?;
    if let Some(timeline) = ctx.document.timelines.get(&slide.id) {
        crate::timing::write(&mut x, timeline, ctx.object_ids, ctx.check)?;
    }
    x.raw("</p:sld>")?;
    Ok(x.finish())
}
