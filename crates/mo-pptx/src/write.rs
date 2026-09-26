use crate::{
    COLOR_SLOTS, ExportDefaults, PptxError, PptxLimits, R, Resources, cancelled, definitions,
    drawing::Drawing, text, value, xml::Xml,
};
use mo_common::{LayoutId, MasterId, ObjectId, ResourceId, ThemeId};
use mo_opc::{
    PackageBuilder, PartName, Relationship, RelationshipSource, ResultSink, VerifiedPackage,
};
use mo_presentation_model::*;
use std::collections::{BTreeMap, BTreeSet};

struct MasterPlan {
    id: Option<MasterId>,
    part: PartName,
    theme: PartName,
}
struct LayoutPlan {
    id: Option<LayoutId>,
    part: PartName,
    master: PartName,
}
struct Plan {
    themes: BTreeMap<Option<ThemeId>, PartName>,
    masters: Vec<MasterPlan>,
    layouts: Vec<LayoutPlan>,
    object_ids: BTreeMap<ObjectId, u32>,
    images: BTreeMap<ResourceId, PartName>,
}

/// Generates an authored PPTX into bounded memory, then reopens actual OPC bytes.
/// This is not yet an imported-document preservation exporter or layout proof.
pub fn export(
    document: &Document,
    defaults: &ExportDefaults,
    resources: &(impl Resources + ?Sized),
    limits: PptxLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, PptxError> {
    Ok(export_to(document, defaults, resources, Vec::new(), limits, check)?.into_reader())
}

/// Streams the same authored package into an injected host sink, seals it, and
/// verifies the actual stored OPC bytes. No output-sized Vec is required here.
/// XML plans remain bounded in memory. The returned proof is package integrity,
/// not full presentation quality or authorization to publish a product artifact.
pub fn export_to<S: ResultSink>(
    document: &Document,
    defaults: &ExportDefaults,
    resources: &(impl Resources + ?Sized),
    sink: S,
    limits: PptxLimits,
    check: &dyn Fn() -> bool,
) -> Result<VerifiedPackage<S::Reader>, PptxError> {
    Ok(
        prepare_package(document, defaults, resources, limits, check)?.write_sealed(
            sink,
            limits.package,
            check,
        )?,
    )
}

fn prepare_package<'a>(
    document: &Document,
    defaults: &ExportDefaults,
    resources: &'a (impl Resources + ?Sized),
    limits: PptxLimits,
    check: &dyn Fn() -> bool,
) -> Result<PackageBuilder<'a>, PptxError> {
    cancelled(check)?;
    let report = validate(document, limits.document);
    if !report.is_valid() {
        return Err(PptxError::InvalidDocument(report));
    }
    if document
        .resources
        .values()
        .any(|r| r.kind == ResourceKind::SourcePackage)
    {
        return Err(PptxError::Unsupported(
            "imported documents require source-preservation export planning".into(),
        ));
    }
    for length in [document.page_size.width, document.page_size.height] {
        if !(914400..=51206400).contains(&length.get()) {
            return Err(value(
                "pageSize",
                "native slide dimension must be within 1..=56 inches",
            ));
        }
    }
    if defaults.font_family.trim().is_empty() {
        return Err(value(
            "defaults.fontFamily",
            "explicit font family is required",
        ));
    }
    text::centipoints(defaults.text_size, "defaults.textSize", 100, 400000)?;
    for (slot, _) in COLOR_SLOTS {
        if !defaults.theme_colors.contains_key(&slot) {
            return Err(value("defaults.themeColors", format!("missing {slot:?}")));
        }
    }
    let plan = plan(document)?;
    let max = limits.package.xml.max_bytes;
    let mut package = PackageBuilder::new();
    for (id, part) in &plan.themes {
        cancelled(check)?;
        package.add_part(
            part.clone(),
            "application/vnd.openxmlformats-officedocument.theme+xml".into(),
            definitions::theme(document, id.as_ref(), defaults, max)?,
        )?;
    }
    for master in &plan.masters {
        let mut ctx = context(document, &plan, &master.part, check);
        let layouts = plan
            .layouts
            .iter()
            .enumerate()
            .filter(|(_, l)| l.master == master.part)
            .map(|(i, l)| (2147483648_u32 + i as u32, l.part.clone()))
            .collect::<Vec<_>>();
        let bytes = definitions::master(
            &mut ctx,
            master.id.as_ref(),
            &master.theme,
            &layouts,
            defaults,
            max,
        )?;
        add(
            &mut package,
            master.part.clone(),
            "slideMaster",
            bytes,
            ctx.relationships,
        )?;
    }
    for layout in &plan.layouts {
        let mut ctx = context(document, &plan, &layout.part, check);
        let bytes = definitions::layout(&mut ctx, layout.id.as_ref(), &layout.master, max)?;
        add(
            &mut package,
            layout.part.clone(),
            "slideLayout",
            bytes,
            ctx.relationships,
        )?;
    }
    let mut slide_parts = Vec::new();
    for (i, id) in document.slide_order.iter().enumerate() {
        cancelled(check)?;
        let slide = &document.slides[id];
        let path = part(format!("/ppt/slides/slide{}.xml", i + 1))?;
        let layout = if let Some(id) = &slide.layout {
            &plan
                .layouts
                .iter()
                .find(|l| l.id.as_ref() == Some(id))
                .expect("validated layout")
                .part
        } else {
            &plan.layouts[0].part
        };
        let mut ctx = context(document, &plan, &path, check);
        let bytes = definitions::slide(&mut ctx, slide, layout, max)?;
        add(
            &mut package,
            path.clone(),
            "slide",
            bytes,
            ctx.relationships,
        )?;
        slide_parts.push(path);
    }
    for (id, path) in &plan.images {
        cancelled(check)?;
        let resource = &document.resources[id];
        let data = resources.open(id)?;
        let mut signature = [0_u8; 8];
        if data.byte_length < 8 {
            return Err(value(
                format!("resources/{id}"),
                "truncated image signature",
            ));
        }
        data.reader
            .read_exact_at(&mut signature, 0)
            .map_err(mo_opc::OpcError::from)?;
        let valid = match resource.media_type.as_str() {
            "image/png" => signature == *b"\x89PNG\r\n\x1a\n",
            "image/jpeg" => signature.starts_with(&[0xFF, 0xD8, 0xFF]),
            _ => false,
        };
        if !valid {
            return Err(value(
                format!("resources/{id}"),
                "image bytes disagree with declared media type",
            ));
        }
        package.add_resource(
            path.clone(),
            resource.media_type.clone(),
            data.reader,
            data.byte_length,
            resource.sha256.clone(),
        )?;
    }
    let main = part("/ppt/presentation.xml")?;
    let mut ctx = context(document, &plan, &main, check);
    let mut x = Xml::new(max)?;
    definitions::root(&mut x, "presentation")?;
    x.raw(" autoCompressPictures=\"0\"><p:sldMasterIdLst>")?;
    for (i, master) in plan.masters.iter().enumerate() {
        let rel = ctx.relationship("slideMaster", &master.part)?;
        x.raw("<p:sldMasterId")?;
        x.attr("id", 2147483648_u32 + i as u32)?;
        x.attr("r:id", rel)?;
        x.raw("/>")?;
    }
    x.raw("</p:sldMasterIdLst>")?;
    if !slide_parts.is_empty() {
        x.raw("<p:sldIdLst>")?;
        for (i, slide) in slide_parts.iter().enumerate() {
            let rel = ctx.relationship("slide", slide)?;
            x.raw("<p:sldId")?;
            x.attr("id", 256 + i as u32)?;
            x.attr("r:id", rel)?;
            x.raw("/>")?;
        }
        x.raw("</p:sldIdLst>")?;
    }
    x.raw("<p:sldSz")?;
    x.attr("cx", document.page_size.width.get())?;
    x.attr("cy", document.page_size.height.get())?;
    x.raw("/><p:notesSz cx=\"6858000\" cy=\"9144000\"/><p:defaultTextStyle>")?;
    text::defaults(&mut x, defaults)?;
    x.raw("</p:defaultTextStyle></p:presentation>")?;
    let properties = part("/ppt/presProps.xml")?;
    ctx.relationship("presProps", &properties)?;
    add(
        &mut package,
        main.clone(),
        "presentation.main",
        x.finish(),
        ctx.relationships,
    )?;
    let mut x = Xml::new(max)?;
    definitions::root(&mut x, "presentationPr")?;
    x.raw("/>")?;
    package.add_part(
        properties,
        "application/vnd.openxmlformats-officedocument.presentationml.presProps+xml".into(),
        x.finish(),
    )?;
    let core = part("/docProps/core.xml")?;
    let mut x = Xml::new(max)?;
    x.raw("<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>")?;
    x.text(&document.title)?;
    x.raw("</dc:title></cp:coreProperties>")?;
    package.add_part(
        core.clone(),
        "application/vnd.openxmlformats-package.core-properties+xml".into(),
        x.finish(),
    )?;
    package.set_relationships(RelationshipSource::Package,vec![Relationship::new(&RelationshipSource::Package,"rId1".into(),format!("{R}/officeDocument"),main.to_string(),false)?,Relationship::new(&RelationshipSource::Package,"rId2".into(),"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties".into(),core.to_string(),false)?])?;
    Ok(package)
}

fn context<'a>(
    document: &'a Document,
    plan: &'a Plan,
    path: &PartName,
    check: &'a dyn Fn() -> bool,
) -> Drawing<'a> {
    Drawing {
        document,
        object_ids: &plan.object_ids,
        images: &plan.images,
        source: path.clone(),
        relationships: vec![],
        check,
    }
}
fn part(path: impl Into<String>) -> Result<PartName, PptxError> {
    Ok(PartName::new(path)?)
}
fn add(
    package: &mut PackageBuilder<'_>,
    path: PartName,
    kind: &str,
    bytes: Vec<u8>,
    rels: Vec<Relationship>,
) -> Result<(), PptxError> {
    package.add_part(
        path.clone(),
        format!("application/vnd.openxmlformats-officedocument.presentationml.{kind}+xml"),
        bytes,
    )?;
    if !rels.is_empty() {
        package.set_relationships(RelationshipSource::Part(path), rels)?;
    }
    Ok(())
}
fn plan(document: &Document) -> Result<Plan, PptxError> {
    let mut themes = BTreeMap::new();
    for (i, id) in std::iter::once(None)
        .chain(document.themes.keys().cloned().map(Some))
        .enumerate()
    {
        themes.insert(id, part(format!("/ppt/theme/theme{}.xml", i + 1))?);
    }
    let mut masters = vec![MasterPlan {
        id: None,
        part: part("/ppt/slideMasters/slideMaster1.xml")?,
        theme: themes[&None].clone(),
    }];
    for (i, (id, m)) in document.masters.iter().enumerate() {
        masters.push(MasterPlan {
            id: Some(id.clone()),
            part: part(format!("/ppt/slideMasters/slideMaster{}.xml", i + 2))?,
            theme: themes[&Some(m.theme.clone())].clone(),
        });
    }
    let mut layouts = vec![LayoutPlan {
        id: None,
        part: part("/ppt/slideLayouts/slideLayout1.xml")?,
        master: masters[0].part.clone(),
    }];
    for (i, (id, l)) in document.layouts.iter().enumerate() {
        layouts.push(LayoutPlan {
            id: Some(id.clone()),
            part: part(format!("/ppt/slideLayouts/slideLayout{}.xml", i + 2))?,
            master: masters
                .iter()
                .find(|m| m.id.as_ref() == Some(&l.master))
                .expect("validated master")
                .part
                .clone(),
        });
    }
    for master in &masters {
        if !layouts.iter().any(|l| l.master == master.part) {
            layouts.push(LayoutPlan {
                id: None,
                part: part(format!(
                    "/ppt/slideLayouts/slideLayout{}.xml",
                    layouts.len() + 1
                ))?,
                master: master.part.clone(),
            });
        }
    }
    let mut images = BTreeMap::new();
    let ids = document
        .objects
        .values()
        .filter_map(|o| match &o.content {
            ObjectContent::Picture { resource, .. } => Some(resource.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for (i, id) in ids.into_iter().enumerate() {
        let extension = match document.resources[&id].media_type.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            _ => {
                return Err(PptxError::Unsupported(
                    "image format export beyond PNG/JPEG".into(),
                ));
            }
        };
        images.insert(
            id,
            part(format!("/ppt/media/image{}.{}", i + 1, extension))?,
        );
    }
    let object_ids = document
        .objects
        .keys()
        .enumerate()
        .map(|(i, id)| (id.clone(), i as u32 + 2))
        .collect();
    Ok(Plan {
        themes,
        masters,
        layouts,
        object_ids,
        images,
    })
}
