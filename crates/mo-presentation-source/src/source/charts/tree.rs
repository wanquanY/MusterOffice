//! Bounded transient projected XML; original part bytes remain the authority.
use super::*;
use mo_xml::{Element, XmlEvent, mce};
#[derive(Debug)]
pub(super) struct Node {
    pub element: Element,
    pub ordinal: u32,
    pub children: Vec<usize>,
    pub text: String,
    pub extensions: Vec<u32>,
}
pub(super) struct Tree {
    pub nodes: Vec<Node>,
    pub compatibility: super::super::SourceCompatibility,
    pub extension_ordinals: Vec<u32>,
}
impl Tree {
    pub fn children<'a>(&'a self, node: usize, local: &'a str) -> impl Iterator<Item = usize> + 'a {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .filter(move |i| self.nodes[*i].element.name.is(C, local))
    }
    pub fn optional(&self, node: usize, local: &str) -> Result<Option<usize>, PptxError> {
        let mut found = self.children(node, local);
        let result = found.next();
        if found.next().is_some() {
            return Err(invalid(format!("duplicate chart {local}")));
        }
        Ok(result)
    }
    pub fn required(&self, node: usize, local: &str) -> Result<usize, PptxError> {
        self.optional(node, local)?
            .ok_or_else(|| invalid(format!("missing chart {local}")))
    }
    pub fn value(&self, node: usize, local: &str) -> Result<Option<String>, PptxError> {
        self.optional(node, local)?
            .map(|i| {
                if !self.nodes[i].children.is_empty() {
                    return Err(invalid("chart scalar has child elements"));
                }
                Ok(self.nodes[i].text.clone())
            })
            .transpose()
    }
    pub fn number(&self, node: usize, local: &str) -> Result<u32, PptxError> {
        number(&self.nodes[self.required(node, local)?].element, "val")
    }
}
pub(super) fn read(
    bytes: &[u8],
    mut xml: mo_xml::XmlLimits,
    profile: &mce::Profile,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<Tree, PptxError> {
    xml.max_bytes = xml.max_bytes.min(budget.limits.max_part_bytes);
    let mut nodes: Vec<Node> = vec![];
    let mut stack: Vec<usize> = vec![];
    let mut skipped = 0;
    let mut extension_ordinals = vec![];
    let summary = mce::scan(bytes, xml, profile, check, |event| {
        match event {
            mce::Event::SourceElement { .. } => {
                budget.elements += 1;
                if budget.elements > budget.limits.max_elements {
                    return Err(mo_xml::XmlError::Limit("chart XML elements"));
                }
            }
            mce::Event::Content {
                event,
                source_ordinal,
                extension_content,
                ..
            } => match event {
                XmlEvent::Start { element, .. } => {
                    if extension_content || skipped > 0 {
                        if skipped == 0 {
                            budget.account(16)?;
                            let ordinal =
                                u32::try_from(source_ordinal.expect("extension root ordinal"))
                                    .map_err(|_| mo_xml::XmlError::Limit("chart ordinal"))?;
                            extension_ordinals.push(ordinal);
                            if let Some(&parent) = stack.last() {
                                nodes[parent].extensions.push(ordinal);
                            }
                        }
                        skipped += 1;
                        return Ok(());
                    }
                    // Account expanded names too: a long namespace URI repeated by
                    // tiny prefixed tags must not amplify retained memory unchecked.
                    budget.account(
                        128 + element.name.namespace.len()
                            + element.name.local.len()
                            + element.qualified_name.len(),
                    )?;
                    for attr in &element.attributes {
                        budget.account(
                            64 + attr.name.namespace.len()
                                + attr.name.local.len()
                                + attr.qualified_name.len()
                                + attr.value.len(),
                        )?;
                    }
                    let i = nodes.len();
                    if let Some(parent) = stack.last().copied() {
                        let parent: usize = parent;
                        nodes[parent].children.push(i);
                    } else if !nodes.is_empty() {
                        return Err(mo_xml::XmlError::Malformed("multiple chart roots".into()));
                    }
                    nodes.push(Node {
                        element: element.clone(),
                        ordinal: source_ordinal
                            .expect("start ordinal")
                            .try_into()
                            .map_err(|_| mo_xml::XmlError::Limit("chart ordinal"))?,
                        children: vec![],
                        text: String::new(),
                        extensions: vec![],
                    });
                    stack.push(i);
                }
                XmlEvent::End { .. } => {
                    if skipped > 0 {
                        skipped -= 1;
                    } else {
                        stack.pop();
                    }
                }
                XmlEvent::Text { text, .. } if skipped == 0 => {
                    if let Some(&i) = stack.last() {
                        budget.account(text.len())?;
                        nodes[i].text.push_str(text);
                    }
                }
                _ => {}
            },
        }
        Ok(())
    })?;
    if nodes.is_empty() {
        return Err(invalid("missing chart XML root"));
    }
    Ok(Tree {
        nodes,
        compatibility: super::super::compatibility::record(summary)?,
        extension_ordinals,
    })
}
