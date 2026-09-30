//! Shared native paragraph/run cascade. Charts supply their own declared layers
//! and defer chart defaults instead of inheriting shape defaults or fake objects.
use super::*;
pub(super) fn character(
    locations: &[Layer<'_>],
    defaults: bool,
    budget: &mut Budget<'_>,
) -> Result<CascadedCharacterStyle, Failure> {
    let mut style = CascadedCharacterStyle::default();
    for layer in locations {
        let Layer::Native(at) = layer else {
            if let Layer::Table(table) = layer {
                table.apply(&mut style, budget)?;
            }
            continue;
        };
        let n = at.checked(budget)?;
        let SourceTextValue::Character { attributes } = &n.value else {
            return Err(conflict().into());
        };
        properties::character_bytes(attributes, budget)?;
        properties::character(
            &mut style.attributes,
            attributes,
            &mut style.origins,
            &at.origin,
            budget,
        )?;
        declarations(
            at,
            &mut style.declarations,
            properties::character_slot,
            budget,
        )?;
    }
    if defaults {
        properties::character(
            &mut style.attributes,
            &properties::character_defaults(),
            &mut style.origins,
            &TextStyleOrigin::ProfileDefault {},
            budget,
        )?;
    }
    Ok(style)
}

pub(super) fn paragraph(
    location: &Location<'_>,
    parents: &[Layer<'_>],
    run_kind: &dyn Fn(usize) -> Option<SourceRunKind>,
    defaults: bool,
    budget: &mut Budget<'_>,
) -> Result<CascadedParagraph, Failure> {
    let mut attributes = SourceTextParagraphAttributes::default();
    let mut origins = BTreeMap::new();
    let mut selected = BTreeMap::new();
    let mut characters = vec![];
    for layer in parents {
        let Layer::Native(at) = layer else {
            characters.push(layer.clone());
            continue;
        };
        let n = at.checked(budget)?;
        let SourceTextValue::Paragraph { attributes: from } = &n.value else {
            return Err(conflict().into());
        };
        if let Some(c) = &from.default_tab_size {
            budget.bytes(c.lexical().len())?;
        }
        // lvl selects a local paragraph's style; lvl on lvlNpPr is ignored.
        let mut from = *from.clone();
        if !std::ptr::eq(at.catalog, location.catalog) || n.parent != Some(location.id) {
            from.level = None;
        }
        properties::paragraph(&mut attributes, &from, &mut origins, &at.origin, budget)?;
        declarations(at, &mut selected, properties::paragraph_slot, budget)?;
        if let Some(c) = at.child(NativeTextElement::DefRPr, budget)? {
            characters.push(Layer::Native(c));
        }
    }
    if defaults {
        properties::paragraph(
            &mut attributes,
            &properties::paragraph_defaults(),
            &mut origins,
            &TextStyleOrigin::ProfileDefault {},
            budget,
        )?;
    }
    let mut runs = vec![];
    for c in location.children(budget)? {
        let n = c.node()?;
        let kind = match n.element {
            NativeTextElement::R => SourceRunKind::Text,
            NativeTextElement::Br => SourceRunKind::Break,
            NativeTextElement::Fld => SourceRunKind::Field,
            NativeTextElement::PPr | NativeTextElement::EndParaRPr => continue,
            _ => return Err(conflict().into()),
        };
        c.checked(budget)?;
        budget.runs = budget
            .runs
            .checked_add(1)
            .ok_or(PptxError::Limit("text cascade runs"))?;
        if budget.runs > budget.limits.max_runs {
            return Err(PptxError::Limit("text cascade runs").into());
        }
        if run_kind(runs.len()) != Some(kind) {
            return Err(conflict().into());
        }
        if let Some(at) = c.child(NativeTextElement::PPr, budget)? {
            return Err(TextCascadeUnresolved::FieldParagraph { origin: at.origin }.into());
        }
        // Check unresolved text-leaf attributes before advertising a usable
        // source binding. Explicit xml:space preservation retains every scalar.
        if let Some(t) = c.child(NativeTextElement::T, budget)? {
            t.checked(budget)?;
        }
        let local = c.child(NativeTextElement::RPr, budget)?;
        let mut chain = local.into_iter().map(Layer::Native).collect::<Vec<_>>();
        chain.extend(characters.iter().cloned());
        let style = character(&chain, defaults, budget)?;
        runs.push(CascadedTextRun {
            source_ordinal: c.id,
            run: runs.len() as u32,
            kind,
            style,
        });
    }
    if run_kind(runs.len()).is_some() {
        return Err(conflict().into());
    }
    let mut end = location
        .child(NativeTextElement::EndParaRPr, budget)?
        .into_iter()
        .map(Layer::Native)
        .collect::<Vec<_>>();
    end.extend(characters);
    let end_style = character(&end, defaults, budget)?;
    Ok(CascadedParagraph {
        source_ordinal: location.id,
        attributes,
        origins,
        declarations: selected,
        runs,
        end_style,
    })
}
