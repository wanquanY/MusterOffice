use crate::{
    AuthorPlan, ExportDefaults, PptxError, PptxLimits, R, Resources, cancelled, definitions,
    drawing::Drawing, native, value, xml::Xml,
};
use mo_opc::{
    PackageBuilder, PartName, Relationship, RelationshipSource, ResultSink, VerifiedPackage,
};
use mo_presentation_model::*;

/// Generates a PPTX into bounded memory and verifies actual stored OPC bytes.
/// This covers authored plans and supported retained-field edits, not layout proof.
pub fn export(
    document: &Document,
    defaults: &ExportDefaults,
    resources: &(impl Resources + ?Sized),
    limits: PptxLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, PptxError> {
    Ok(export_to(document, defaults, resources, Vec::new(), limits, check)?.into_reader())
}

/// Streams the authored or retained package into an injected host sink, seals it, and
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
    let plan = crate::PresentationPlan::new(
        document,
        defaults,
        resources,
        limits.document,
        limits.package,
        check,
    )?;
    export_presentation_plan_to(&plan, resources, sink, limits.package, check)
}

/// Serialize the same input plan used for compilation. Retained documents
/// rewrite only supported fields and independently verify the stored result.
pub fn export_presentation_plan_to<S: ResultSink>(
    plan: &crate::PresentationPlan<'_>,
    resources: &(impl Resources + ?Sized),
    sink: S,
    limits: mo_opc::PackageLimits,
    check: &dyn Fn() -> bool,
) -> Result<VerifiedPackage<S::Reader>, PptxError> {
    match plan {
        crate::PresentationPlan::Author(plan) => {
            export_plan_to(plan, resources, sink, limits, check)
        }
        crate::PresentationPlan::Retained { plan, source } => plan.write_to(source, sink, check),
    }
}

/// Writes the immutable semantic plan also consumed by page compilation.
pub fn export_plan_to<S: ResultSink>(
    plan: &AuthorPlan<'_>,
    resources: &(impl Resources + ?Sized),
    sink: S,
    limits: mo_opc::PackageLimits,
    check: &dyn Fn() -> bool,
) -> Result<VerifiedPackage<S::Reader>, PptxError> {
    Ok(prepare_package(plan, resources, limits, check)?.write_sealed(sink, limits, check)?)
}

fn prepare_package<'a>(
    author: &AuthorPlan<'_>,
    resources: &'a (impl Resources + ?Sized),
    limits: mo_opc::PackageLimits,
    check: &dyn Fn() -> bool,
) -> Result<PackageBuilder<'a>, PptxError> {
    cancelled(check)?;
    let document = author.document();
    let plan = author.bindings();
    let max = limits.xml.max_bytes;
    let mut package = PackageBuilder::new();
    for part in plan.themes.values() {
        cancelled(check)?;
        package.add_part(
            part.clone(),
            "application/vnd.openxmlformats-officedocument.theme+xml".into(),
            definitions::theme(&author.declarations().themes[&part.to_string()], max)?,
        )?;
    }
    for master in &plan.masters {
        let mut ctx = context(author, &master.part, check);
        let layouts = plan
            .layouts
            .iter()
            .enumerate()
            .filter(|(_, l)| l.master == master.part)
            .map(|(i, l)| (2147483648_u32 + i as u32, l.part.clone()))
            .collect::<Vec<_>>();
        let bytes =
            definitions::master(&mut ctx, master.id.as_ref(), &master.theme, &layouts, max)?;
        add(
            &mut package,
            master.part.clone(),
            "slideMaster",
            bytes,
            ctx.relationships,
        )?;
    }
    for layout in &plan.layouts {
        let mut ctx = context(author, &layout.part, check);
        let bytes = definitions::layout(&mut ctx, layout.id.as_ref(), &layout.master, max)?;
        add(
            &mut package,
            layout.part.clone(),
            "slideLayout",
            bytes,
            ctx.relationships,
        )?;
    }
    for binding in &plan.slides {
        cancelled(check)?;
        let slide = &document.slides[&binding.id];
        let path = &binding.part;
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
        let mut ctx = context(author, path, check);
        let bytes = definitions::slide(&mut ctx, slide, layout, max)?;
        add(
            &mut package,
            path.clone(),
            "slide",
            bytes,
            ctx.relationships,
        )?;
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
    let mut ctx = context(author, &main, check);
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
    if !plan.slides.is_empty() {
        x.raw("<p:sldIdLst>")?;
        for slide in &plan.slides {
            let rel = ctx.relationship("slide", &slide.part)?;
            x.raw("<p:sldId")?;
            x.attr("id", slide.native_id)?;
            x.attr("r:id", rel)?;
            x.raw("/>")?;
        }
        x.raw("</p:sldIdLst>")?;
    }
    x.raw("<p:sldSz")?;
    x.attr("cx", document.page_size.width.get())?;
    x.attr("cy", document.page_size.height.get())?;
    x.raw("/><p:notesSz cx=\"6858000\" cy=\"9144000\"/>")?;
    let text = &author.declarations().text;
    native::text(&mut x, text, text.roots[0].source_ordinal, &[])?;
    x.raw("</p:presentation>")?;
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
    author: &'a AuthorPlan<'a>,
    path: &PartName,
    check: &'a dyn Fn() -> bool,
) -> Drawing<'a> {
    Drawing::new(author, path, check)
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
