//! Owned native cell text fixture; ordinary and cell tests use the same font.
#[allow(dead_code)]
#[path = "source_glyphs.rs"]
mod glyphs;
pub use glyphs::*;
#[path = "table_text_fixture.rs"]
mod fixture;
pub use fixture::fixture;
