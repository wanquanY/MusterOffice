//! Connection-scoped protocol adapter over shared caller-file computation.
mod bridge;
mod config;
mod resources;
mod server;
pub use bridge::Bridge;
pub use config::Config;
pub use server::ComputeServer;

use mo_embedded_sdk::operation::Failure;
use mo_native_compute::SavedComputation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileArguments {
    pub invocation_file: String,
    pub inputs_file: String,
    pub output_directory: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "outcome", rename_all = "camelCase", deny_unknown_fields)]
pub enum Response {
    Computed { result: SavedComputation },
    Failed { error: Failure },
}
