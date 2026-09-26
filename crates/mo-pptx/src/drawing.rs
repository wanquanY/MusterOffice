use crate::{
    AuthorPlan, PptxError, R, cancelled, native,
    source::{SourceObject, SourceSurface},
    value,
    xml::Xml,
};
use mo_common::{ObjectId, ResourceId};
use mo_opc::{PartName, Relationship, RelationshipSource};
use mo_presentation_model::*;
use std::collections::BTreeMap;

pub(crate) struct Drawing<'a> {
    pub document: &'a Document,
    pub object_ids: &'a BTreeMap<ObjectId, u32>,
    pub images: &'a BTreeMap<ResourceId, PartName>,
    pub source: PartName,
    pub relationships: Vec<Relationship>,
    pub check: &'a dyn Fn() -> bool,
    surface: Option<&'a SourceSurface>,
    objects: BTreeMap<u32, &'a SourceObject>,
}
impl<'a> Drawing<'a> {
    pub fn new(plan: &'a AuthorPlan<'a>, source: &PartName, check: &'a dyn Fn() -> bool) -> Self {
        let surface = plan.declarations().surfaces.get(&source.to_string());
        Self {
            document: plan.document(),
            object_ids: &plan.bindings().object_ids,
            images: &plan.bindings().images,
            source: source.clone(),
            relationships: vec![],
            check,
            surface,
            objects: surface
                .into_iter()
                .flat_map(|s| s.objects.iter())
                .map(|o| (o.native_id, o))
                .collect(),
        }
    }
    pub fn surface(&self) -> &'a SourceSurface {
        self.surface.expect("surface projection for drawing part")
    }
    pub fn relationship(&mut self, kind: &str, target: &PartName) -> Result<String, PptxError> {
        let id = format!("rId{}", self.relationships.len() + 1);
        self.relationships.push(Relationship::new(
            &RelationshipSource::Part(self.source.clone()),
            id.clone(),
            format!("{R}/{kind}"),
            target.to_string(),
            false,
        )?);
        Ok(id)
    }
    pub fn tree(&mut self, x: &mut Xml, objects: &[ObjectId]) -> Result<(), PptxError> {
        x.raw("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>")?;
        for id in objects {
            self.object(x, id)?;
        }
        x.raw("</p:spTree>")
    }
    fn nonvisual(&self, x: &mut Xml, object: &Object) -> Result<(), PptxError> {
        x.raw("<p:cNvPr")?;
        x.attr("id", self.object_ids[&object.id])?;
        x.attr("name", object.id.as_str())?;
        x.attr("title", &object.accessibility.title)?;
        x.attr("descr", &object.accessibility.description)?;
        x.raw("/>")
    }
    fn properties(&self, x: &mut Xml, object: &SourceObject) -> Result<(), PptxError> {
        if let Some(t) = &object.transform {
            native::transform(x, t)?;
        }
        if let Some(g) = &object.geometry {
            native::geometry(x, g)?;
        }
        if let Some(f) = &object.fill {
            native::fill(x, f)?;
        }
        if let Some(l) = &object.line {
            native::line(x, l)?;
        }
        Ok(())
    }
    fn object(&mut self, x: &mut Xml, id: &ObjectId) -> Result<(), PptxError> {
        cancelled(self.check)?;
        let o = &self.document.objects[id];
        let native = self.objects[&self.object_ids[id]];
        match &o.content {
            ObjectContent::Shape { .. } => {
                x.raw("<p:sp><p:nvSpPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>")?;
                self.properties(x, native)?;
                x.raw("</p:spPr>")?;
                if let Some(root) = native.text_body_ordinal {
                    native::text(x, &self.surface().text, root, &native.paragraphs)?;
                }
                x.raw("</p:sp>")?;
            }
            ObjectContent::Picture { resource, .. } => {
                let rel = self.relationship("image", &self.images[resource].clone())?;
                let crate::source::fill::SourceFillDefinition::Image(image) = &native
                    .picture_fill
                    .as_ref()
                    .expect("planned picture fill")
                    .definition
                else {
                    return Err(value("author plan", "picture fill"));
                };
                if image.blip.as_ref().and_then(|b| b.embed.as_ref()) != Some(&rel) {
                    return Err(value("author plan", "image relationship binding"));
                }
                x.raw("<p:pic><p:nvPicPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip")?;
                x.attr("r:embed", rel)?;
                x.raw("/><a:srcRect")?;
                if let Some(crop) = &image.source_rect {
                    for (name, amount) in [
                        ("l", &crop.left),
                        ("t", &crop.top),
                        ("r", &crop.right),
                        ("b", &crop.bottom),
                    ] {
                        if let Some(amount) = amount {
                            x.attr(name, native::lexical(amount)?)?;
                        }
                    }
                }
                x.raw("/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>")?;
                self.properties(x, native)?;
                x.raw("</p:spPr></p:pic>")?;
            }
            ObjectContent::Group { children, .. } => {
                x.raw("<p:grpSp><p:nvGrpSpPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>")?;
                self.properties(x, native)?;
                x.raw("</p:grpSpPr>")?;
                for child in children {
                    self.object(x, child)?;
                }
                x.raw("</p:grpSp>")?;
            }
            ObjectContent::Connector { start, end } => {
                x.raw("<p:cxnSp><p:nvCxnSpPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvCxnSpPr>")?;
                for (tag, endpoint) in [("stCxn", start), ("endCxn", end)] {
                    if let ConnectorEndpoint::Attached { object, site } = endpoint {
                        x.raw("<a:")?;
                        x.raw(tag)?;
                        x.attr("id", self.object_ids[object])?;
                        x.attr("idx", site)?;
                        x.raw("/>")?;
                    }
                }
                x.raw("</p:cNvCxnSpPr><p:nvPr/></p:nvCxnSpPr><p:spPr>")?;
                self.properties(x, native)?;
                x.raw("</p:spPr></p:cxnSp>")?;
            }
        }
        Ok(())
    }
}
