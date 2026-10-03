//! Presentation input is a kernel command, distinct from raw source events.
use crate::{ExactValue, NavigationDirection};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The last presentation step in the sampled, validated event prefix. The
/// enclosing frame binds document/session/generation and hashes this receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationStepReceipt {
    pub sequence: u32,
    pub at: ExactValue,
    pub direction: NavigationDirection,
    pub outcome: PresentationStepOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PresentationStepOutcome {
    /// An eligible page condition admitted this event, including delayed work.
    /// This remains consumed even if a later event cancels that scheduled work.
    Consumed,
    /// No eligible page condition admitted this event. The presentation host
    /// may cross the requested boundary, using this kernel-defined entry state.
    PageBoundary { entry: PresentationPageEntry },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PresentationPageEntry {
    /// A new page owner with time zero and an empty input prefix. Previous does
    /// not synthesize a terminal pose or guess clicks on an unbounded timeline.
    Initial,
}
