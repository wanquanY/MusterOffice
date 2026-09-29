use crate::{BindingValue, ParameterTarget, Template, TemplateError as Error};
use mo_common::*;
use mo_presentation_edit::{Operation, OperationEntry};
use mo_presentation_model::{
    Document, InlineContent, ObjectContent, ResourceKind, RetainedRunKind,
};
use std::collections::BTreeMap;

pub(crate) fn target_key(target: &ParameterTarget) -> Result<Digest, Error> {
    let key = match target {
        ParameterTarget::TextRun {
            object,
            paragraph,
            run,
            ..
        } => serde_json::json!(["text", object, paragraph, run]),
        ParameterTarget::Resource { resource, .. } => serde_json::json!(["resource", resource]),
        ParameterTarget::ThemeColor { theme, slot } => {
            serde_json::json!(["themeColor", theme, slot])
        }
        ParameterTarget::Transform { object } => serde_json::json!(["transform", object]),
    };
    Ok(digest("musteroffice.template-target/1", &key)?)
}

fn text<'a>(document: &'a Document, target: &ParameterTarget) -> Option<&'a str> {
    let ParameterTarget::TextRun {
        object,
        paragraph,
        run,
        ..
    } = target
    else {
        return None;
    };
    match &document.objects.get(object)?.content {
        ObjectContent::Shape {
            text: Some(body), ..
        } => {
            let run = body
                .paragraphs
                .iter()
                .find(|p| &p.id == paragraph)?
                .runs
                .iter()
                .find(|r| &r.id == run)?;
            match &run.content {
                InlineContent::Text { text } => Some(text),
                _ => None,
            }
        }
        ObjectContent::RetainedSource { paragraphs, .. } => {
            let run = paragraphs
                .iter()
                .find(|p| &p.id == paragraph)?
                .runs
                .iter()
                .find(|r| &r.id == run)?;
            (run.kind == RetainedRunKind::Text).then_some(run.text.as_str())
        }
        _ => None,
    }
}

pub(crate) fn example(document: &Document, target: &ParameterTarget) -> Option<BindingValue> {
    match target {
        ParameterTarget::TextRun { .. } => {
            text(document, target).map(|v| BindingValue::Text(v.into()))
        }
        ParameterTarget::Resource { resource, .. } => document
            .resources
            .get(resource)
            .cloned()
            .map(BindingValue::Resource),
        ParameterTarget::ThemeColor { theme, slot } => document
            .themes
            .get(theme)?
            .colors
            .get(slot)
            .copied()
            .map(BindingValue::Color),
        ParameterTarget::Transform { object } => document
            .objects
            .get(object)?
            .transform
            .map(BindingValue::Transform),
    }
}

fn operation_id(kind: &str, id: &str) -> Result<OperationId, Error> {
    let digest = digest("musteroffice.template-parameter-operation/1", &(kind, id))?;
    Ok(OperationId::new(format!("template:{digest}")).expect("bounded digest identifier"))
}

impl Template {
    pub(crate) fn operations(
        &self,
        values: &BTreeMap<TemplateParameterId, BindingValue>,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<OperationEntry>, Error> {
        for id in values.keys() {
            if !self.definition.parameters.contains_key(id) {
                return Err(Error::parameter(
                    id,
                    "parameter is not declared by this template",
                ));
            }
        }
        let document = self.source.document();
        let mut operations = Vec::new();
        let mut themes = BTreeMap::new();
        for (id, parameter) in &self.definition.parameters {
            crate::cancelled(check)?;
            let Some(value) = values.get(id) else {
                if parameter.required {
                    return Err(Error::parameter(id, "required binding is missing"));
                }
                continue;
            };
            let operation = match (&parameter.target, value) {
                (
                    ParameterTarget::TextRun {
                        object,
                        paragraph,
                        run,
                        min_scalars,
                        max_scalars,
                    },
                    BindingValue::Text(insert),
                ) => {
                    let size = insert.chars().count();
                    if size < *min_scalars as usize || size > *max_scalars as usize {
                        return Err(Error::parameter(
                            id,
                            "text exceeds its Unicode scalar constraint",
                        ));
                    }
                    let original = text(document, &parameter.target)
                        .ok_or_else(|| Error::parameter(id, "text target is unavailable"))?;
                    Operation::SpliceText {
                        object: object.clone(),
                        paragraph: paragraph.clone(),
                        run: run.clone(),
                        start: 0,
                        delete: u32::try_from(original.chars().count()).map_err(|_| {
                            Error::parameter(id, "text exceeds the editable scalar range")
                        })?,
                        insert: insert.clone(),
                    }
                }
                (
                    ParameterTarget::Resource {
                        resource,
                        media_types,
                    },
                    BindingValue::Resource(replacement),
                ) => {
                    let original = document
                        .resources
                        .get(resource)
                        .ok_or_else(|| Error::parameter(id, "resource is unavailable"))?;
                    if replacement.id != *resource || replacement.kind != original.kind {
                        return Err(Error::parameter(
                            id,
                            "resource identity and kind must match the declared target",
                        ));
                    }
                    if replacement.kind == ResourceKind::SourcePackage
                        || !media_types.contains(&replacement.media_type)
                    {
                        return Err(Error::parameter(
                            id,
                            "resource media type or immutable source provenance cannot be replaced",
                        ));
                    }
                    Operation::AttachResource {
                        resource: replacement.clone(),
                    }
                }
                (ParameterTarget::ThemeColor { theme, slot }, BindingValue::Color(color)) => {
                    // Coalesce all bindings to one theme; separate baseline
                    // replacements would overwrite earlier color bindings.
                    let theme_value = themes
                        .entry(theme.clone())
                        .or_insert_with(|| document.themes[theme].clone());
                    theme_value.colors.insert(*slot, *color);
                    continue;
                }
                (ParameterTarget::Transform { object }, BindingValue::Transform(transform)) => {
                    Operation::SetTransform {
                        object: object.clone(),
                        transform: *transform,
                    }
                }
                _ => {
                    return Err(Error::parameter(
                        id,
                        "binding type differs from its declared target",
                    ));
                }
            };
            operations.push(OperationEntry {
                operation_id: operation_id("parameter", id.as_str())?,
                operation,
            });
        }
        for (id, theme) in themes {
            crate::cancelled(check)?;
            operations.push(OperationEntry {
                operation_id: operation_id("theme", id.as_str())?,
                operation: Operation::PutTheme { theme },
            });
        }
        Ok(operations)
    }
}
