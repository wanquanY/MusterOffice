//! Native chart source/cache inspection. No workbook recalculation, rendering,
//! external fetch, implicit refresh or editable-chart capability is asserted.
mod data;
mod layout;
mod layout_types;
mod tree;
mod types;
use super::{SourceIndex, SourceLimits, SourceObjectKind, SourceObjectRef};
use crate::{A, P, PptxError, R, cancelled};
pub use layout_types::*;
use mo_common::ByteLength;
use mo_opc::{PackageRead, PartName, Relationship, RelationshipSource, RelationshipTarget};
use mo_xml::{Element, ExpandedName, mce};
use std::collections::{BTreeMap, BTreeSet};
pub use types::*;
const C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CHART_TYPE: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
fn invalid(message: impl Into<String>) -> PptxError {
    PptxError::SourceConflict(message.into())
}
fn number(element: &Element, attribute: &str) -> Result<u32, PptxError> {
    element
        .attribute(attribute)
        .ok_or_else(|| invalid(format!("missing chart {attribute}")))?
        .trim()
        .parse()
        .map_err(|_| invalid(format!("invalid chart {attribute}")))
}
fn relationship_id(element: &Element) -> Result<String, PptxError> {
    element
        .attributes
        .iter()
        .find(|a| a.name.is(R, "id"))
        .map(|a| a.value.clone())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| invalid("missing chart relationship id"))
}
struct Budget {
    limits: SourceChartLimits,
    elements: usize,
    metadata_bytes: usize,
    part_bytes: usize,
    relationships: usize,
    series: usize,
    points: usize,
    axes: usize,
}
impl Budget {
    fn account(&mut self, bytes: usize) -> Result<(), mo_xml::XmlError> {
        self.metadata_bytes = self
            .metadata_bytes
            .checked_add(bytes)
            .filter(|v| *v <= self.limits.max_metadata_bytes)
            .ok_or(mo_xml::XmlError::Limit("chart retained XML metadata"))?;
        Ok(())
    }
    fn read(
        &mut self,
        package: &dyn PackageRead,
        part: &PartName,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, PptxError> {
        let info = package
            .parts()
            .get(part)
            .ok_or_else(|| invalid("missing chart source part"))?;
        let length: usize = info
            .byte_length
            .try_into()
            .map_err(|_| PptxError::Limit("chart part bytes"))?;
        self.part_bytes = self
            .part_bytes
            .checked_add(length)
            .filter(|v| *v <= self.limits.max_total_part_bytes)
            .ok_or(PptxError::Limit("chart total part bytes"))?;
        if length > self.limits.max_part_bytes {
            return Err(PptxError::Limit("chart part bytes"));
        }
        Ok(package.read_part(part, self.limits.max_part_bytes as u64, check)?)
    }
    fn relationship<'a>(
        &mut self,
        package: &'a dyn PackageRead,
        owner: &PartName,
        id: &str,
        kind: &str,
        check: &dyn Fn() -> bool,
    ) -> Result<&'a Relationship, PptxError> {
        for rel in package
            .relationships()
            .get(&RelationshipSource::Part(owner.clone()))
            .into_iter()
            .flatten()
        {
            cancelled(check)?;
            self.relationships += 1;
            if self.relationships > self.limits.max_relationship_steps {
                return Err(PptxError::Limit("chart relationship steps"));
            }
            if rel.id == id {
                if rel.relationship_type != format!("{R}/{kind}") {
                    return Err(invalid("wrong chart relationship type"));
                }
                return Ok(rel);
            }
        }
        Err(invalid("missing chart relationship"))
    }
}
fn profile() -> mce::Profile {
    mce::Profile {
        understood_namespaces: [C, A, R, "http://www.w3.org/XML/1998/namespace", ""]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        extension_elements: [C, A]
            .into_iter()
            .map(|namespace| ExpandedName {
                namespace: namespace.into(),
                local: "extLst".into(),
            })
            .collect(),
    }
}
/// Inspect direct chart references on one source surface, preserving source pins
/// and native relationship identity. Other graphics remain outside this query.
pub fn query(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceChartQuery,
    source_limits: SourceLimits,
    limits: SourceChartLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceCharts, PptxError> {
    cancelled(check)?;
    if package.sha256() != &request.expected_source_sha256
        || index.source_sha256 != request.expected_source_sha256
    {
        return Err(invalid("chart source digest mismatch"));
    }
    let surface = index
        .surfaces
        .get(&request.surface)
        .ok_or_else(|| invalid("chart surface not in source index"))?;
    let owner = PartName::new(&request.surface)?;
    let mut budget = Budget {
        limits,
        elements: 0,
        metadata_bytes: 0,
        part_bytes: 0,
        relationships: 0,
        series: 0,
        points: 0,
        axes: 0,
    };
    let bytes = budget.read(package, &owner, check)?;
    // Keep the exact presentation MCE selection used to build SourceIndex.
    let surface_tree = tree::read(
        &bytes,
        source_limits.package.xml,
        &super::compatibility::profile(),
        &mut budget,
        check,
    )?;
    let mut result = SourceCharts {
        source_sha256: index.source_sha256.clone(),
        surface: request.surface.clone(),
        bindings: vec![],
        charts: vec![],
    };
    let mut parts = BTreeMap::new();
    let mut object_ids = BTreeSet::new();
    for object in &surface.objects {
        cancelled(check)?;
        if object.kind == SourceObjectKind::GraphicFrame {
            object_ids.insert(object.native_id);
        }
    }
    for node in &surface_tree.nodes {
        cancelled(check)?;
        if !node.element.name.is(P, "graphicFrame") {
            continue;
        }
        let child = |parent: &tree::Node, namespace, local| -> Result<&tree::Node, PptxError> {
            let mut children = parent
                .children
                .iter()
                .map(|i| &surface_tree.nodes[*i])
                .filter(|n| n.element.name.is(namespace, local));
            let item = children
                .next()
                .ok_or_else(|| invalid("incomplete chart graphicFrame"))?;
            if children.next().is_some() {
                return Err(invalid("duplicate chart graphicFrame child"));
            }
            Ok(item)
        };
        let properties = child(node, P, "nvGraphicFramePr")?;
        let native_id = number(&child(properties, P, "cNvPr")?.element, "id")?;
        let graphic = child(node, A, "graphic")?;
        let data = child(graphic, A, "graphicData")?;
        if data.element.attribute("uri") != Some(C) {
            continue;
        }
        if !object_ids.contains(&native_id) {
            return Err(invalid("chart object not in source index"));
        }
        let reference = child(data, C, "chart")?;
        let id = relationship_id(&reference.element)?;
        let rel = budget.relationship(package, &owner, &id, "chart", check)?;
        let RelationshipTarget::Internal {
            part,
            fragment: None,
        } = &rel.resolved
        else {
            return Err(invalid("chart requires an internal part without fragment"));
        };
        if result.bindings.len() >= limits.max_bindings {
            return Err(PptxError::Limit("chart bindings"));
        }
        let chart = if let Some(i) = parts.get(part) {
            *i
        } else {
            if result.charts.len() >= limits.max_charts {
                return Err(PptxError::Limit("chart parts"));
            }
            let info = package
                .parts()
                .get(part)
                .ok_or_else(|| invalid("missing chart part"))?;
            if info.content_type != CHART_TYPE {
                return Err(invalid("chart content type mismatch"));
            }
            let bytes = budget.read(package, part, check)?;
            let tree = tree::read(
                &bytes,
                source_limits.package.xml,
                &profile(),
                &mut budget,
                check,
            )?;
            let (plots, external_data) = data::read(package, part, &tree, &mut budget, check)?;
            let axes = layout::axes(&tree, &mut budget, check)?;
            let i = result.charts.len() as u32;
            result.charts.push(SourceChartPart {
                axes,
                part: part.to_string(),
                sha256: info.sha256.clone(),
                byte_length: ByteLength::new(info.byte_length),
                compatibility: tree.compatibility,
                plots,
                external_data,
                data_authority: ChartDataAuthority::SourceCacheSnapshot,
            });
            parts.insert(part.clone(), i);
            i
        };
        result.bindings.push(SourceChartBinding {
            object: SourceObjectRef {
                part: request.surface.clone(),
                native_id,
            },
            source_ordinal: reference.ordinal,
            relationship_id: id,
            chart,
        });
    }
    Ok(result)
}
