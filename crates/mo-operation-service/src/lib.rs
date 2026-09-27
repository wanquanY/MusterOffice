//! Legacy persistent-host contract and compatibility adapters. Pure consumers
//! use mo-presentation-operations; domain algorithms are not implemented here.
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

pub use mo_presentation_operations::{
    AssetBinding, AssetDescriptor, AssetId, AssetInfo, AssetVerification, DocumentAction,
    ExportAsset, ExportReceipt, ExportSettings, MutationReceipt, OperationProfile,
};
mod compatibility;
