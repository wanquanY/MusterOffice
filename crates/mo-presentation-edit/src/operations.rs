use mo_common::*;
use mo_presentation_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transaction {
    pub document_id: DocumentId,
    pub request_id: RequestId,
    pub base_revision: Digest,
    pub operations: Vec<OperationEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationEntry {
    pub operation_id: OperationId,
    pub operation: Operation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DeletePolicy {
    RejectDependencies,
    Cascade,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Operation {
    EditTable {
        object: ObjectId,
        operation: crate::TableOperation,
    },
    /// Compile native editorial groups and replace this slide's whole timeline.
    SetPresentationSequence {
        slide: SlideId,
        sequence: mo_timeline::PresentationSequence,
    },
    SetTimeline {
        slide: SlideId,
        timeline: Option<mo_timeline::Timeline>,
    },
    SetTitle {
        title: String,
    },
    SetSlideName {
        slide: SlideId,
        name: String,
    },
    SetSlideBackground {
        slide: SlideId,
        background: Inherited<Fill>,
    },
    InsertSlide {
        slide: Slide,
        index: u32,
    },
    /// Clone the owned object/text graph and its timeline with deterministic new
    /// identities. Shared masters, layouts, fonts and resources remain shared.
    DuplicateSlide {
        source: SlideId,
        slide: SlideId,
        index: u32,
    },
    DeleteSlide {
        slide: SlideId,
        policy: DeletePolicy,
    },
    MoveSlide {
        slide: SlideId,
        index: u32,
    },
    SetLayout {
        slide: SlideId,
        layout: Option<LayoutId>,
    },
    PutTheme {
        theme: Theme,
    },
    PutMaster {
        master: Master,
    },
    PutLayout {
        layout: Layout,
    },
    /// Bind a resource without changing pixels selected by existing objects.
    EnsureResource {
        resource: Resource,
    },
    AttachResource {
        resource: Resource,
    },
    DetachResource {
        resource: ResourceId,
    },
    PutFont {
        font: FontFace,
    },
    InsertObject {
        object: Object,
        index: u32,
    },
    DeleteObject {
        object: ObjectId,
        policy: DeletePolicy,
    },
    /// Caller supplies the intended local transform explicitly when changing coordinates.
    MoveObject {
        object: ObjectId,
        parent: ContainerId,
        index: u32,
        transform: Transform,
    },
    SetTransform {
        object: ObjectId,
        transform: Transform,
    },
    SetAppearance {
        object: ObjectId,
        appearance: Appearance,
    },
    SetFill {
        object: ObjectId,
        fill: Inherited<Fill>,
    },
    SetStroke {
        object: ObjectId,
        stroke: Inherited<Stroke>,
    },
    /// Replace source pixels while preserving identity, geometry and crop unless specified.
    SetPicture {
        object: ObjectId,
        resource: ResourceId,
        crop: Option<Crop>,
    },
    SetPictureCrop {
        object: ObjectId,
        crop: Crop,
    },
    SetGeometry {
        object: ObjectId,
        geometry: Geometry,
    },
    SetAccessibility {
        object: ObjectId,
        accessibility: Accessibility,
    },
    SetText {
        object: ObjectId,
        text: TextBody,
    },
    /// Splice within one text run using Unicode scalar offsets, never UTF-16 or bytes.
    SpliceText {
        object: ObjectId,
        paragraph: ParagraphId,
        run: RunId,
        start: u32,
        delete: u32,
        insert: String,
    },
}
