use super::*;
use crate::{Failure, FailureCode, MAX_OPERATION_BYTES};
use mo_common::{ParagraphId, RunId};
use mo_presentation_model::*;

fn cancelled(check: &dyn Fn() -> bool) -> Result<(), Failure> {
    if check() {
        Err(Failure::new(
            FailureCode::Cancelled,
            "composition cancelled",
        ))
    } else {
        Ok(())
    }
}
fn invalid(message: &'static str) -> Failure {
    Failure::new(FailureCode::InputInvalid, message)
}
fn inherit<T: Clone>(value: &Option<T>) -> Inherited<T> {
    value.clone().map_or(Inherited::Inherit, Inherited::Value)
}
fn size(
    value: &impl Serialize,
    remaining: usize,
    check: &dyn Fn() -> bool,
) -> Result<usize, Failure> {
    mo_common::check_json_size(value, remaining, check).map_err(|e| match e {
        mo_common::JsonBudgetError::Cancelled => {
            Failure::new(FailureCode::Cancelled, "composition cancelled")
        }
        mo_common::JsonBudgetError::Limit => {
            Failure::new(FailureCode::LimitExceeded, "expanded composition bytes")
        }
        mo_common::JsonBudgetError::Serialization => invalid("composition serialization"),
    })
}

impl PresentationContent {
    /// Materializes native declarations only. Snapshot admission remains the
    /// authority for references, geometry and model validity. No font, renderer,
    /// resource, theme or host environment is guessed during expansion.
    pub fn to_document(&self, check: &dyn Fn() -> bool) -> Result<Document, Failure> {
        cancelled(check)?;
        size(self, MAX_OPERATION_BYTES, check)?;
        let limits = ValidationLimits::default();
        if self.slides.is_empty() {
            return Err(invalid("composition requires at least one slide"));
        }
        if self.slides.len() > limits.max_slides {
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "composition slides",
            ));
        }
        let mut document = Document::empty(self.id.clone(), self.page_size);
        document.title = self.title.clone();
        for resource in &self.resources {
            if document
                .resources
                .insert(resource.id.clone(), resource.clone())
                .is_some()
            {
                return Err(invalid("duplicate composition resource id"));
            }
        }
        let mut used = size(&document, MAX_OPERATION_BYTES, check)?;
        for source in &self.slides {
            cancelled(check)?;
            if document.slides.contains_key(&source.id) {
                return Err(invalid("duplicate composition slide id"));
            }
            let mut slide = Slide {
                id: source.id.clone(),
                name: source.name.clone(),
                layout: None,
                objects: vec![],
                background: inherit(&source.background),
                hidden: false,
            };
            for element in &source.elements {
                cancelled(check)?;
                if document.objects.len() >= limits.max_objects {
                    return Err(Failure::new(
                        FailureCode::LimitExceeded,
                        "composition objects",
                    ));
                }
                if document.objects.contains_key(element.id()) {
                    return Err(invalid("duplicate composition object id"));
                }
                let f = element.frame();
                let (appearance, content) = match element {
                    ElementContent::Shape(shape) => {
                        let text = shape
                            .text
                            .as_ref()
                            .map(|text| {
                                lower_text(
                                    text,
                                    &self.id,
                                    &shape.id,
                                    MAX_OPERATION_BYTES - used,
                                    check,
                                )
                            })
                            .transpose()?;
                        (
                            Appearance {
                                fill: Inherited::Value(shape.fill.clone().unwrap_or(Fill::None)),
                                stroke: Inherited::Value(
                                    shape.stroke.clone().unwrap_or(Stroke::None {}),
                                ),
                            },
                            ObjectContent::Shape {
                                geometry: shape.geometry.clone(),
                                text,
                            },
                        )
                    }
                    ElementContent::Picture(picture) => (
                        Appearance {
                            fill: Inherited::Value(Fill::None),
                            stroke: Inherited::Value(Stroke::None {}),
                        },
                        ObjectContent::Picture {
                            resource: picture.picture.resource.clone(),
                            crop: picture.picture.crop,
                        },
                    ),
                };
                let object = Object {
                    id: element.id().clone(),
                    parent: ContainerId::Slide(source.id.clone()),
                    transform: Some(Transform {
                        origin: Point { x: f.x, y: f.y },
                        size: Size {
                            width: f.width,
                            height: f.height,
                        },
                        rotation: f.rotation,
                        flip_horizontal: f.flip_horizontal,
                        flip_vertical: f.flip_vertical,
                    }),
                    appearance,
                    accessibility: element.accessibility().clone(),
                    content,
                };
                used += size(&object, MAX_OPERATION_BYTES - used, check)?;
                slide.objects.push(element.id().clone());
                document.objects.insert(element.id().clone(), object);
            }
            used += size(&slide, MAX_OPERATION_BYTES - used, check)?;
            document.slide_order.push(source.id.clone());
            document.slides.insert(source.id.clone(), slide);
        }
        size(&document, MAX_OPERATION_BYTES, check)?;
        cancelled(check)?;
        Ok(document)
    }
}

pub(super) fn lower_text(
    input: &PlainText,
    document: &DocumentId,
    object: &ObjectId,
    remaining: usize,
    check: &dyn Fn() -> bool,
) -> Result<TextBody, Failure> {
    let s = &input.style;
    let mut body = TextBody {
        paragraphs: vec![],
        style: CharacterStyle {
            font: Inherited::Inherit,
            size: inherit(&s.size),
            color: inherit(&s.color),
            bold: inherit(&s.bold),
            italic: inherit(&s.italic),
            underline: inherit(&s.underline),
            language: inherit(&s.language),
        },
        insets: input.insets,
        wrap: input.wrap,
        overflow: OverflowPolicy::Report,
    };
    let mut used = size(&body, remaining, check)?;
    // Split CR and LF without copying the whole input or dropping trailing
    // empty paragraphs. CRLF is one separator, never two blank paragraphs.
    let mut start = 0;
    let mut chars = input.text.char_indices().peekable();
    loop {
        cancelled(check)?;
        let mut end = input.text.len();
        let mut next = None;
        for (at, c) in chars.by_ref() {
            cancelled(check)?;
            if c == '\r' || c == '\n' {
                end = at;
                next = Some(at + 1);
                break;
            }
        }
        if next.is_some()
            && input.text.as_bytes()[end] == b'\r'
            && chars.peek().is_some_and(|(_, c)| *c == '\n')
        {
            next = chars.next().map(|(at, _)| at + 1);
        }
        let ordinal = body.paragraphs.len();
        let identity = mo_common::digest(
            "musteroffice.compose.paragraph/1",
            &(document, object, ordinal),
        )
        .map_err(|_| invalid("composition paragraph identity"))?;
        let mut paragraph = Paragraph {
            id: ParagraphId::new(format!("paragraph:{}", identity.as_str()))
                .map_err(|_| invalid("composition paragraph id"))?,
            style: ParagraphStyle {
                alignment: inherit(&s.alignment),
                direction: inherit(&s.direction),
                space_before: inherit(&s.space_before),
                space_after: inherit(&s.space_after),
                line_spacing: s.line_spacing,
                left_margin: s.left_margin,
                right_margin: s.right_margin,
                indent: s.indent,
            },
            default_run_style: CharacterStyle::default(),
            runs: vec![],
        };
        let mut paragraph_bytes = size(&paragraph, remaining - used, check)?;
        for (segment, text) in input.text[start..end].split('\t').enumerate() {
            cancelled(check)?;
            if segment > 0 {
                push_run(
                    &mut paragraph,
                    &identity,
                    InlineContent::Tab,
                    &mut paragraph_bytes,
                    remaining - used,
                    check,
                )?;
            }
            // Empty paragraph still has a real editable empty text run.
            push_run(
                &mut paragraph,
                &identity,
                InlineContent::Text { text: text.into() },
                &mut paragraph_bytes,
                remaining - used,
                check,
            )?;
        }
        used += size(&paragraph, remaining - used, check)?;
        body.paragraphs.push(paragraph);
        match next {
            Some(at) => start = at,
            None => break,
        }
    }
    size(&body, remaining, check)?;
    Ok(body)
}

fn push_run(
    paragraph: &mut Paragraph,
    identity: &mo_common::Digest,
    content: InlineContent,
    used: &mut usize,
    limit: usize,
    check: &dyn Fn() -> bool,
) -> Result<(), Failure> {
    cancelled(check)?;
    let run = TextRun {
        id: RunId::new(format!(
            "run:{}:{}",
            identity.as_str(),
            paragraph.runs.len()
        ))
        .map_err(|_| invalid("composition run id"))?,
        style: CharacterStyle::default(),
        content,
    };
    *used += size(&run, limit - *used, check)?;
    paragraph.runs.push(run);
    Ok(())
}
