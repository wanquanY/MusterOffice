//! Deterministic presentation time and property evaluation. No clocks or host state.
mod clock;
mod evaluate;
mod events;
mod exact;
mod generation;
mod model;
mod plan;
mod sampler;
mod tree;
pub use generation::*;

pub use evaluate::*;
use mo_common::TimingNodeId;
pub use model::*;
pub use plan::*;
pub use sampler::*;

#[derive(Debug, thiserror::Error)]
pub enum TimelineError {
    #[error("invalid timing node {node:?}: {message}")]
    Invalid {
        node: Option<TimingNodeId>,
        message: String,
    },
    #[error("timeline limit exceeded: {0}")]
    Limit(&'static str),
    #[error("timeline computation cancelled")]
    Cancelled,
    #[error("interactive sampling requires a complete event history through the sample time")]
    MissingEventHistory,
    #[error("invalid playback event history: {0}")]
    EventHistory(&'static str),
    #[error(transparent)]
    Canonical(#[from] mo_common::CanonicalError),
}
pub(crate) fn invalid(node: Option<&TimingNodeId>, message: impl Into<String>) -> TimelineError {
    TimelineError::Invalid {
        node: node.cloned(),
        message: message.into(),
    }
}
pub(crate) fn cancel(check: &dyn Fn() -> bool) -> Result<(), TimelineError> {
    if check() {
        Err(TimelineError::Cancelled)
    } else {
        Ok(())
    }
}
