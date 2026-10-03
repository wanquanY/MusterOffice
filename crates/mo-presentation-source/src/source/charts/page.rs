//! Admission of whole-chart container semantics for the bounded 2D page profile.
//! Inspection remains lossless; unsupported visual declarations fail explicitly.
use super::*;
/// The plot-area paint owner after container admission. All ordinals are physical.
pub fn page_structure(
    package: &dyn PackageRead,
    part: &str,
    check: &dyn Fn() -> bool,
) -> Result<u32, PptxError> {
    let mut budget = Budget::new(SourceChartLimits::default());
    let bytes = budget.read(package, &mo_opc::PartName::new(part)?, check)?;
    let tree = tree::read(
        &bytes,
        SourceLimits::default().package.xml,
        &profile(),
        &mut budget,
        check,
    )?;
    let chart = tree.required(0, "chart")?;
    let area = tree.required(chart, "plotArea")?;
    for (parent, structural, scalars) in [
        (
            0,
            &["chart", "spPr", "externalData", "clrMapOvr"][..],
            &["lang", "date1904", "roundedCorners"][..],
        ),
        (
            chart,
            &["plotArea", "legend", "title"][..],
            &[
                "plotVisOnly",
                "dispBlanksAs",
                "showDLblsOverMax",
                "autoTitleDeleted",
            ][..],
        ),
        (
            area,
            &[
                "barChart",
                "lineChart",
                "pieChart",
                "doughnutChart",
                "catAx",
                "valAx",
                "spPr",
            ][..],
            &[][..],
        ),
    ] {
        cancelled(check)?;
        let root = &tree.nodes[parent];
        let mut attrs = vec![];
        layout::retain_attributes(root, &[], &mut attrs);
        if !attrs.is_empty() || !root.extensions.is_empty() || !root.text.trim().is_empty() {
            return Err(invalid("unsupported chart container content"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for &i in &root.children {
            let n = &tree.nodes[i];
            let name = n.element.name.local.as_str();
            if n.element.name.namespace != C
                || (!structural.contains(&name) && !scalars.contains(&name))
            {
                return Err(invalid(format!(
                    "unsupported page chart container {name} at {}",
                    n.ordinal
                )));
            }
            if name != "catAx" && name != "valAx" && !seen.insert(name) {
                return Err(invalid("duplicate chart container"));
            }
            if scalars.contains(&name) {
                layout::leaf(n)?;
                layout::retain_attributes(n, &["val"], &mut attrs);
                let val = n
                    .element
                    .attribute("val")
                    .ok_or_else(|| invalid("chart scalar value missing"))?;
                let supported = match name {
                    "lang" => !val.is_empty(),
                    "dispBlanksAs" => val == "gap",
                    "roundedCorners" | "showDLblsOverMax" => matches!(val, "0" | "false"),
                    "autoTitleDeleted" => matches!(val, "1" | "true"),
                    _ => matches!(val, "0" | "1" | "true" | "false"),
                };
                if !supported || !attrs.is_empty() {
                    return Err(invalid(format!("unsupported chart {name}={val}")));
                }
            }
        }
    }
    for &axis in &tree.nodes[area].children {
        if !matches!(
            tree.nodes[axis].element.name.local.as_str(),
            "catAx" | "valAx"
        ) {
            continue;
        }
        if let Some(scaling) = tree.optional(axis, "scaling")? {
            let l = layout::read(&tree, scaling, layout::Scope::Scaling, check)?;
            if !l.unrecognized_children.is_empty()
                || !l.retained_attribute_ordinals.is_empty()
                || !l.markup.is_empty()
            {
                return Err(invalid("unsupported axis scaling content"));
            }
        }
    }
    Ok(tree.nodes[area].ordinal)
}
