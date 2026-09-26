//! Model-independent operation authority is injected by the host. This crate has
//! no persistence, paths, network, process spawning or implicit credentials.
mod assets;
mod compute;
mod contract;
mod discovery;
mod export;
mod host;
mod identity;
mod schemas;
pub use assets::*;
pub use compute::*;
pub use contract::*;
pub use discovery::*;
pub use export::*;
pub use host::*;
pub use identity::*;
pub use schemas::*;
