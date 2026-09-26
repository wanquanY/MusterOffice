use super::{NAMESPACE, Profile};
use crate::{Element, ExpandedName, XmlError, names};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

/// Shared, immutable lexical scopes. Only declarations allocate a new scope;
/// ordinary elements clone an Rc rather than the full inherited rule set.
#[derive(Debug, Default)]
pub(super) struct Rules {
    parent: Option<Rc<Rules>>,
    ignorable: BTreeSet<String>,
    process_content: BTreeMap<String, BTreeSet<String>>,
}
impl Rules {
    pub(super) fn ignorable(&self, ns: &str) -> bool {
        self.ignorable.contains(ns) || self.parent.as_ref().is_some_and(|p| p.ignorable(ns))
    }
    pub(super) fn process(&self, name: &ExpandedName) -> bool {
        self.process_content
            .get(&name.namespace)
            .is_some_and(|locals| locals.contains(&name.local) || locals.contains("*"))
            || self.parent.as_ref().is_some_and(|p| p.process(name))
    }
}

pub(super) struct Attributes {
    pub rules: Rc<Rules>,
    pub must_understand: Vec<String>,
    pub requires: Vec<String>,
}

fn tokens(value: &str) -> impl Iterator<Item = &str> {
    value
        .split([' ', '\t', '\r', '\n'])
        .filter(|s| !s.is_empty())
}
fn namespace(prefix: &str, bindings: &BTreeMap<String, String>) -> Result<String, XmlError> {
    if !names::ncname(prefix) {
        return Err(invalid(
            "invalid namespace prefix in compatibility attribute",
        ));
    }
    let value = bindings
        .get(prefix)
        .ok_or_else(|| invalid("unbound compatibility namespace prefix"))?;
    if value == NAMESPACE || value.is_empty() {
        return Err(invalid("invalid compatibility namespace reference"));
    }
    Ok(value.clone())
}
fn namespaces(value: &str, bindings: &BTreeMap<String, String>) -> Result<Vec<String>, XmlError> {
    tokens(value)
        .map(|prefix| namespace(prefix, bindings))
        .collect()
}

pub(super) fn attributes(
    element: &Element,
    bindings: &BTreeMap<String, String>,
    parent: &Rc<Rules>,
) -> Result<Attributes, XmlError> {
    let mut declared = Rules {
        parent: Some(parent.clone()),
        ..Default::default()
    };
    if let Some(attr) = element
        .attributes
        .iter()
        .find(|a| a.name.is(NAMESPACE, "Ignorable"))
    {
        declared.ignorable = namespaces(&attr.value, bindings)?.into_iter().collect();
    }
    if let Some(attr) = element
        .attributes
        .iter()
        .find(|a| a.name.is(NAMESPACE, "ProcessContent"))
    {
        for token in tokens(&attr.value) {
            let (prefix, local) = token
                .split_once(':')
                .ok_or_else(|| invalid("ProcessContent requires a prefixed name"))?;
            if local != "*" && !names::ncname(local) {
                return Err(invalid("invalid ProcessContent local name"));
            }
            let ns = namespace(prefix, bindings)?;
            if !declared.ignorable(&ns) {
                return Err(invalid("ProcessContent namespace is not ignorable"));
            }
            declared
                .process_content
                .entry(ns)
                .or_default()
                .insert(local.into());
        }
    }
    let rules = if declared.ignorable.is_empty() && declared.process_content.is_empty() {
        parent.clone()
    } else {
        Rc::new(declared)
    };
    let mut result = Attributes {
        rules,
        must_understand: Vec::new(),
        requires: Vec::new(),
    };
    let control = element.name.namespace == NAMESPACE;
    for attr in &element.attributes {
        if attr.name.namespace == NAMESPACE {
            match attr.name.local.as_str() {
                "Ignorable" | "ProcessContent" => {}
                "MustUnderstand" => result.must_understand = namespaces(&attr.value, bindings)?,
                _ => {
                    return Err(XmlError::Compatibility(format!(
                        "unimplemented MCE attribute {}",
                        attr.name.local
                    )));
                }
            }
        } else if control {
            if attr.name.namespace.is_empty() {
                if !element.name.is(NAMESPACE, "Choice") || attr.name.local != "Requires" {
                    return Err(invalid("invalid unqualified MCE control attribute"));
                }
                result.requires = namespaces(&attr.value, bindings)?;
            } else if attr.name.namespace == names::XML_NS
                || !result.rules.ignorable(&attr.name.namespace)
            {
                return Err(invalid("MCE control foreign attribute is not ignorable"));
            }
        }
    }
    if element.name.is(NAMESPACE, "Choice") && result.requires.is_empty() {
        return Err(invalid("Choice Requires must not be empty"));
    }
    Ok(result)
}

pub(super) fn require_understood(namespaces: &[String], profile: &Profile) -> Result<(), XmlError> {
    for ns in namespaces {
        if !profile.understood_namespaces.contains(ns) {
            return Err(XmlError::Compatibility(format!(
                "MustUnderstand namespace is not implemented: {ns}"
            )));
        }
    }
    Ok(())
}
pub(super) fn invalid(message: &str) -> XmlError {
    XmlError::Malformed(format!("MCE: {message}"))
}
