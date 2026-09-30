//! One bounded pass over selected real chart text roots. Uses the same native
//! DrawingML grammar/content readers as shape and table text, including MCE.
use super::*;
use crate::source::{line, paint, text};
use mo_xml::{XmlError, XmlEvent, mce};

#[derive(Default)]
pub(super) struct Budget {
    bodies: usize,
    text_bytes: usize,
    styles: text::Budget,
    lines: line::Budget,
    paints: paint::Budget,
}
struct Active {
    ordinal: u32,
    parent: u32,
    styles: text::Reader,
    content: text::ContentReader,
}
impl Active {
    fn finish(self) -> Result<SourceChartTextBody, XmlError> {
        use text::NativeTextElement as N;
        let mut styles = text::SourceTextCatalog::default();
        self.styles.finish(&mut styles)?;
        let content = self.content.finish()?;
        let paragraphs: Vec<_> = styles.nodes[&self.ordinal]
            .children
            .iter()
            .copied()
            .filter(|id| styles.nodes[id].element == N::P)
            .collect();
        if paragraphs.len() != content.paragraphs.len() {
            return Err(XmlError::Malformed("chart text paragraph binding".into()));
        }
        let paragraphs = paragraphs
            .into_iter()
            .zip(content.paragraphs)
            .map(|(id, content)| {
                let runs: Vec<_> = styles.nodes[&id]
                    .children
                    .iter()
                    .copied()
                    .filter(|id| matches!(styles.nodes[id].element, N::R | N::Br | N::Fld))
                    .collect();
                if runs.len() != content.len() {
                    return Err(XmlError::Malformed("chart text run binding".into()));
                }
                Ok(SourceChartTextParagraph {
                    source_ordinal: id,
                    runs: runs
                        .into_iter()
                        .zip(content)
                        .map(|(id, run)| SourceChartTextRun {
                            source_ordinal: id,
                            text_source_ordinal: styles.nodes[&id]
                                .children
                                .iter()
                                .copied()
                                .find(|id| styles.nodes[id].element == N::T),
                            kind: run.kind,
                            text: run.text,
                        })
                        .collect(),
                })
            })
            .collect::<Result<_, XmlError>>()?;
        Ok(SourceChartTextBody {
            source_ordinal: self.ordinal,
            parent_ordinal: self.parent,
            styles,
            paragraphs,
        })
    }
}
pub(super) fn read(
    bytes: &[u8],
    roots: &BTreeMap<u32, u32>,
    limits: SourceLimits,
    max_bodies: usize,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<Vec<SourceChartTextBody>, PptxError> {
    if roots.is_empty() {
        return Ok(vec![]);
    }
    budget.bodies = budget
        .bodies
        .checked_add(roots.len())
        .filter(|n| *n <= max_bodies)
        .ok_or(PptxError::Limit("chart text bodies"))?;
    let mut bodies = vec![];
    let mut depth = 0usize;
    let mut active: Option<Active> = None;
    mce::scan(bytes, limits.package.xml, &profile(), check, |event| {
        let (event, ordinal, extension, alternate) = match event {
            mce::Event::SourceElement { .. } => {
                if let Some(a) = &mut active {
                    a.content.physical_element();
                }
                return Ok(());
            }
            mce::Event::Content {
                event,
                source_ordinal,
                extension_content,
                alternate_ancestors,
                ..
            } => (
                event,
                source_ordinal,
                extension_content,
                !alternate_ancestors.is_empty(),
            ),
        };
        match event {
            XmlEvent::Start { element, .. } => {
                let ordinal = u32::try_from(ordinal.expect("chart text physical ordinal"))
                    .map_err(|_| XmlError::Limit("chart text ordinal"))?;
                if let Some(a) = &mut active {
                    a.styles.start(
                        element,
                        depth,
                        ordinal,
                        extension,
                        &mut budget.styles,
                        &mut budget.lines,
                        &mut budget.paints,
                        limits,
                    )?;
                    a.content
                        .start(element, depth, ordinal as usize, alternate)?;
                } else if let Some(parent) = roots.get(&ordinal) {
                    if extension || !(element.name.is(C, "txPr") || element.name.is(C, "rich")) {
                        return Err(XmlError::Malformed(
                            "chart text source root mismatch".into(),
                        ));
                    }
                    active = Some(Active {
                        ordinal,
                        parent: *parent,
                        styles: text::Reader::new(
                            element,
                            depth,
                            ordinal,
                            None,
                            &mut budget.styles,
                            limits,
                        )?,
                        content: text::ContentReader::new(depth),
                    });
                }
                depth += 1;
            }
            XmlEvent::End { .. } => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| XmlError::Malformed("chart text depth".into()))?;
                if let Some(a) = &mut active {
                    a.styles.end(depth)?;
                    a.content.end(depth);
                    if a.styles.depth == depth {
                        bodies.push(active.take().expect("active chart text").finish()?);
                    }
                }
            }
            XmlEvent::Text { text, .. } => {
                if let Some(a) = &mut active {
                    a.styles.text(text)?;
                    a.content.text(text, &mut budget.text_bytes, limits)?;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if active.is_some() || depth != 0 || bodies.len() != roots.len() {
        return Err(invalid("unclosed or missing chart text body"));
    }
    Ok(bodies)
}
