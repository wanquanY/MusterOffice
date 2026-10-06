//! GPU pixel execution; the pure compositor owns authoring and resource semantics.
mod encode;
mod execution;
pub use encode::*;
