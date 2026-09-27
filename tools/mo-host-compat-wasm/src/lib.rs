//! Development compatibility binding for the old persistent-host schema API.
//! Deliberately separate from the production computation WASM/SDK dependency
//! graph. It owns no database and provides no alternate document computation.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn operation_schema_json(id_json: &str) -> Result<String, String> {
    mo_operation_service::operation_schema_json(id_json).map_err(|error| error.to_string())
}
