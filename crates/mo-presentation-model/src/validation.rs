use crate::*;
use mo_common::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationCode {
    IdentityMismatch,
    MissingReference,
    DuplicateIdentity,
    InvalidValue,
    OwnershipConflict,
    ReferenceCycle,
    LimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub truncated: bool,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty() && !self.truncated
    }
}

/// Host-selectable admission limits. These are not claims of supported product capacity.
#[derive(Debug, Clone, Copy)]
pub struct ValidationLimits {
    pub max_slides: usize,
    pub max_objects: usize,
    pub max_text_scalars: usize,
    pub max_group_depth: usize,
    pub max_timing_nodes: usize,
    pub max_issues: usize,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_slides: 10_000,
            max_objects: 1_000_000,
            max_text_scalars: 64_000_000,
            max_group_depth: 128,
            max_timing_nodes: 10_000,
            max_issues: 1_024,
        }
    }
}

pub fn validate(document: &Document, limits: ValidationLimits) -> ValidationReport {
    Validator {
        document,
        limits,
        report: ValidationReport::default(),
        listed: BTreeMap::new(),
        paragraphs: BTreeSet::new(),
        runs: BTreeSet::new(),
        text_scalars: 0,
    }
    .run()
}

struct Validator<'a> {
    document: &'a Document,
    limits: ValidationLimits,
    report: ValidationReport,
    listed: BTreeMap<ObjectId, usize>,
    paragraphs: BTreeSet<ParagraphId>,
    runs: BTreeSet<RunId>,
    text_scalars: usize,
}

impl Validator<'_> {
    fn issue(&mut self, code: ValidationCode, path: impl Into<String>, message: impl Into<String>) {
        if self.report.issues.len() >= self.limits.max_issues {
            self.report.truncated = true;
            return;
        }
        self.report.issues.push(ValidationIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }
    fn value(&mut self, condition: bool, path: &str, message: &str) {
        if !condition {
            self.issue(ValidationCode::InvalidValue, path, message);
        }
    }
    fn reference(&mut self, exists: bool, path: &str) {
        if !exists {
            self.issue(
                ValidationCode::MissingReference,
                path,
                "referenced record does not exist",
            );
        }
    }
    fn size(&mut self, size: Size, positive: bool, path: &str) {
        let minimum = if positive { 1 } else { 0 };
        self.value(
            size.width.get() >= minimum && size.height.get() >= minimum,
            path,
            "invalid dimensions",
        );
    }
    fn run(mut self) -> ValidationReport {
        let d = self.document;
        if d.slides.len() > self.limits.max_slides
            || d.objects.len() > self.limits.max_objects
            || d.timelines.len() > self.limits.max_slides
        {
            self.issue(
                ValidationCode::LimitExceeded,
                "/",
                "document exceeds admission count limits",
            );
            return self.report;
        }
        self.size(d.page_size, true, "/pageSize");
        for issue in
            crate::timing::validate(d, self.limits.max_timing_nodes, self.limits.max_group_depth)
        {
            self.issue(issue.code, issue.path, issue.message);
        }
        let mut ordered = BTreeSet::new();
        for (i, id) in d.slide_order.iter().enumerate() {
            self.reference(d.slides.contains_key(id), &format!("/slideOrder/{i}"));
            if !ordered.insert(id) {
                self.issue(
                    ValidationCode::DuplicateIdentity,
                    format!("/slideOrder/{i}"),
                    "slide appears more than once",
                );
            }
        }
        for (key, slide) in &d.slides {
            let path = format!("/slides/{key}");
            if key != &slide.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "map key differs from record identity",
                );
            }
            if !ordered.contains(key) {
                self.issue(
                    ValidationCode::OwnershipConflict,
                    &path,
                    "slide is absent from slideOrder",
                );
            }
            if let Some(layout) = &slide.layout {
                self.reference(d.layouts.contains_key(layout), &format!("{path}/layout"));
            }
            self.list(
                &slide.objects,
                &ContainerId::Slide(key.clone()),
                &format!("{path}/objects"),
            );
        }
        for (key, theme) in &d.themes {
            let path = format!("/themes/{key}");
            if key != &theme.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "theme key differs from identity",
                );
            }
            self.character_style(&theme.default_text, &format!("{path}/defaultText"));
        }
        for (key, master) in &d.masters {
            let path = format!("/masters/{key}");
            if key != &master.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "master key differs from identity",
                );
            }
            self.reference(
                d.themes.contains_key(&master.theme),
                &format!("{path}/theme"),
            );
            self.list(
                &master.objects,
                &ContainerId::Master(key.clone()),
                &format!("{path}/objects"),
            );
            self.character_style(&master.default_text, &format!("{path}/defaultText"));
        }
        for (key, layout) in &d.layouts {
            let path = format!("/layouts/{key}");
            if key != &layout.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "layout key differs from identity",
                );
            }
            self.reference(
                d.masters.contains_key(&layout.master),
                &format!("{path}/master"),
            );
            self.list(
                &layout.objects,
                &ContainerId::Layout(key.clone()),
                &format!("{path}/objects"),
            );
            self.character_style(&layout.default_text, &format!("{path}/defaultText"));
        }
        for (key, font) in &d.fonts {
            let path = format!("/fonts/{key}");
            if key != &font.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "font key differs from identity",
                );
            }
            self.resource(
                &font.resource,
                ResourceKind::Font,
                &format!("{path}/resource"),
            );
            self.value(
                (1..=1000).contains(&font.weight),
                &format!("{path}/weight"),
                "font weight must be 1..=1000",
            );
            self.value(
                !font.family.trim().is_empty(),
                &format!("{path}/family"),
                "font family must not be empty",
            );
        }
        for (key, resource) in &d.resources {
            let path = format!("/resources/{key}");
            if key != &resource.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "resource key differs from identity",
                );
            }
            self.value(
                resource.media_type.contains('/')
                    && resource.media_type.is_ascii()
                    && !resource.media_type.chars().any(char::is_whitespace),
                &format!("{path}/mediaType"),
                "expected an ASCII media type without whitespace",
            );
        }
        for (key, object) in &d.objects {
            let path = format!("/objects/{key}");
            if key != &object.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "object key differs from identity",
                );
            }
            if let Some(t) = object.transform {
                self.size(t.size, false, &format!("{path}/transform/size"));
                self.value(
                    t.origin.x.checked_add(t.size.width).is_ok()
                        && t.origin.y.checked_add(t.size.height).is_ok(),
                    &format!("{path}/transform"),
                    "bounds exceed coordinate range",
                );
            } else {
                self.value(
                    matches!(object.content, ObjectContent::RetainedSource { .. }),
                    &format!("{path}/transform"),
                    "authored object requires a direct transform",
                );
            }
            if let Inherited::Value(Stroke::Solid { width, .. }) = object.appearance.stroke {
                self.value(
                    width.get() >= 0,
                    &format!("{path}/appearance/stroke"),
                    "stroke width is negative",
                );
            }
            match &object.content {
                ObjectContent::RetainedSource {
                    native_kind,
                    children,
                    paragraphs,
                } => {
                    self.value(
                        d.source_bindings
                            .as_ref()
                            .is_some_and(|b| b.objects.contains_key(key)),
                        &path,
                        "retained object requires immutable source binding",
                    );
                    self.value(
                        *native_kind == RetainedObjectKind::Group || children.is_empty(),
                        &path,
                        "only a native group can own children",
                    );
                    self.list(
                        children,
                        &ContainerId::Group(key.clone()),
                        &format!("{path}/content/children"),
                    );
                    for p in paragraphs {
                        if !self.paragraphs.insert(p.id.clone()) {
                            self.issue(
                                ValidationCode::DuplicateIdentity,
                                &path,
                                "duplicate retained paragraph ID",
                            );
                        }
                        for r in &p.runs {
                            if !self.runs.insert(r.id.clone()) {
                                self.issue(
                                    ValidationCode::DuplicateIdentity,
                                    &path,
                                    "duplicate retained run ID",
                                );
                            }
                            self.text_scalars = self.text_scalars.saturating_add(r.scalar_len());
                        }
                    }
                }
                ObjectContent::Shape { geometry, text } => {
                    self.geometry(geometry, &format!("{path}/content/geometry"));
                    if let Some(text) = text {
                        self.text(text, &format!("{path}/content/text"));
                    }
                }
                ObjectContent::Group { children, viewport } => {
                    self.size(*viewport, true, &format!("{path}/content/viewport"));
                    self.list(
                        children,
                        &ContainerId::Group(key.clone()),
                        &format!("{path}/content/children"),
                    );
                }
                ObjectContent::Picture { resource, crop } => {
                    self.resource(
                        resource,
                        ResourceKind::Picture,
                        &format!("{path}/content/resource"),
                    );
                    self.value(
                        i64::from(crop.left) + i64::from(crop.right) < 1_000_000
                            && i64::from(crop.top) + i64::from(crop.bottom) < 1_000_000,
                        &format!("{path}/content/crop"),
                        "crop leaves an empty source region",
                    );
                }
                ObjectContent::Connector { start, end } => {
                    self.endpoint(start, key, &format!("{path}/content/start"));
                    self.endpoint(end, key, &format!("{path}/content/end"));
                }
            }
            self.ancestry(key, &path);
        }
        for key in d.objects.keys() {
            if self.listed.get(key) != Some(&1) {
                self.issue(
                    ValidationCode::OwnershipConflict,
                    format!("/objects/{key}/parent"),
                    "object must occur exactly once in its owning container",
                );
            }
        }
        if self.text_scalars > self.limits.max_text_scalars {
            self.issue(
                ValidationCode::LimitExceeded,
                "/objects",
                "text exceeds admission scalar limit",
            );
        }
        let source_report = crate::source_validation::validate(d, self.limits.max_issues);
        self.report.truncated |= source_report.truncated;
        for issue in source_report.issues {
            self.issue(issue.code, issue.path, issue.message);
        }
        self.report
    }

    fn list(&mut self, ids: &[ObjectId], owner: &ContainerId, path: &str) {
        for (i, id) in ids.iter().enumerate() {
            *self.listed.entry(id.clone()).or_default() += 1;
            match self.document.objects.get(id) {
                None => self.issue(
                    ValidationCode::MissingReference,
                    format!("{path}/{i}"),
                    "object does not exist",
                ),
                Some(object) if &object.parent != owner => self.issue(
                    ValidationCode::OwnershipConflict,
                    format!("{path}/{i}"),
                    "object parent disagrees with container",
                ),
                _ => (),
            }
        }
    }
    fn ancestry(&mut self, id: &ObjectId, path: &str) {
        let mut current = id;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(current) {
                self.issue(
                    ValidationCode::ReferenceCycle,
                    format!("{path}/parent"),
                    "group ownership is cyclic",
                );
                break;
            }
            if seen.len() > self.limits.max_group_depth.saturating_add(1) {
                self.issue(
                    ValidationCode::LimitExceeded,
                    format!("{path}/parent"),
                    "group depth exceeds admission limit",
                );
                break;
            }
            let Some(object) = self.document.objects.get(current) else {
                break;
            };
            match &object.parent {
                ContainerId::Group(parent) => {
                    self.reference(
                        self.document
                            .objects
                            .get(parent)
                            .is_some_and(|p| p.content.children().is_some()),
                        &format!("{path}/parent"),
                    );
                    current = parent;
                }
                ContainerId::Slide(parent) => {
                    self.reference(
                        self.document.slides.contains_key(parent),
                        &format!("{path}/parent"),
                    );
                    break;
                }
                ContainerId::Master(parent) => {
                    self.reference(
                        self.document.masters.contains_key(parent),
                        &format!("{path}/parent"),
                    );
                    break;
                }
                ContainerId::Layout(parent) => {
                    self.reference(
                        self.document.layouts.contains_key(parent),
                        &format!("{path}/parent"),
                    );
                    break;
                }
            }
        }
    }
    fn root(&self, id: &ObjectId) -> Option<ContainerId> {
        let mut id = id;
        let mut visited = BTreeSet::new();
        for _ in 0..=self.limits.max_group_depth {
            if !visited.insert(id) {
                return None;
            }
            match &self.document.objects.get(id)?.parent {
                ContainerId::Group(parent) => id = parent,
                root => return Some(root.clone()),
            }
        }
        None
    }
    fn endpoint(&mut self, endpoint: &ConnectorEndpoint, owner: &ObjectId, path: &str) {
        if let ConnectorEndpoint::Attached { object, .. } = endpoint {
            self.reference(self.document.objects.contains_key(object), path);
            self.value(object != owner, path, "connector cannot attach to itself");
            if self.document.objects.contains_key(object) {
                self.value(
                    self.root(owner).is_some() && self.root(owner) == self.root(object),
                    path,
                    "connector target must share the same slide or definition",
                );
            }
        }
    }
    fn resource(&mut self, id: &ResourceId, kind: ResourceKind, path: &str) {
        match self.document.resources.get(id) {
            None => self.issue(ValidationCode::MissingReference, path, "resource is absent"),
            Some(resource) => {
                self.value(resource.kind == kind, path, "resource kind is incompatible")
            }
        }
    }
    fn character_style(&mut self, style: &CharacterStyle, path: &str) {
        if let Inherited::Value(font) = &style.font {
            self.reference(
                self.document.fonts.contains_key(font),
                &format!("{path}/font"),
            );
        }
        if let Inherited::Value(size) = style.size {
            self.value(
                size.get() > 0,
                &format!("{path}/size"),
                "font size must be positive",
            );
        }
    }
    fn text(&mut self, body: &TextBody, path: &str) {
        self.value(
            !body.paragraphs.is_empty(),
            path,
            "text body must retain at least one paragraph",
        );
        self.value(
            [
                body.insets.left,
                body.insets.top,
                body.insets.right,
                body.insets.bottom,
            ]
            .iter()
            .all(|v| v.get() >= 0),
            &format!("{path}/insets"),
            "text insets must be nonnegative",
        );
        self.character_style(&body.style, &format!("{path}/style"));
        for (i, paragraph) in body.paragraphs.iter().enumerate() {
            let path = format!("{path}/paragraphs/{i}");
            if !self.paragraphs.insert(paragraph.id.clone()) {
                self.issue(
                    ValidationCode::DuplicateIdentity,
                    &path,
                    "paragraph identity is reused",
                );
            }
            self.character_style(
                &paragraph.default_run_style,
                &format!("{path}/defaultRunStyle"),
            );
            let mut count = 0_usize;
            for (j, run) in paragraph.runs.iter().enumerate() {
                let path = format!("{path}/runs/{j}");
                if !self.runs.insert(run.id.clone()) {
                    self.issue(
                        ValidationCode::DuplicateIdentity,
                        &path,
                        "run identity is reused",
                    );
                }
                self.character_style(&run.style, &format!("{path}/style"));
                if let InlineContent::Text { text } = &run.content {
                    self.value(
                        !text.contains(['\n', '\r', '\t']),
                        &format!("{path}/content"),
                        "use typed breaks and tabs or separate paragraphs",
                    );
                }
                count = count.saturating_add(run.content.scalar_len());
            }
            self.value(
                u32::try_from(count).is_ok(),
                &path,
                "paragraph exceeds scalar anchor range",
            );
            self.text_scalars = self.text_scalars.saturating_add(count);
        }
    }
    fn geometry(&mut self, geometry: &Geometry, path: &str) {
        match geometry {
            Geometry::RoundRectangle { radius } => {
                self.value(radius.get() >= 0, path, "corner radius is negative")
            }
            Geometry::Path { commands, viewport } => {
                self.size(*viewport, true, &format!("{path}/viewport"));
                let mut open = false;
                for (i, command) in commands.iter().enumerate() {
                    match command {
                        PathCommand::Move { .. } => open = true,
                        PathCommand::Close => {
                            self.value(
                                open,
                                &format!("{path}/commands/{i}"),
                                "close requires an open subpath",
                            );
                            open = false;
                        }
                        _ => self.value(
                            open,
                            &format!("{path}/commands/{i}"),
                            "drawing command requires a preceding move",
                        ),
                    }
                }
            }
            _ => (),
        }
    }
}
