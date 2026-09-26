//! Explicit, content-addressed font inspection. No paths, system font discovery or shaping.
mod cmap;
mod container;
mod metadata;
mod types;
use mo_common::{ByteLength, Digest};
use read_fonts::{FontRef, TableProvider, types::Tag};
use sha2::{Digest as _, Sha256};
use thiserror::Error;
pub use types::*;

#[derive(Debug, Error)]
pub enum FontError {
    #[error("invalid font request: {0}")]
    InvalidRequest(&'static str),
    #[error("invalid font or request: {0}")]
    Invalid(&'static str),
    #[error("unsupported font feature: {0}")]
    Unsupported(&'static str),
    #[error("font resource SHA-256 does not match request")]
    ResourceConflict,
    #[error("font resource limit exceeded: {0}")]
    Limit(&'static str),
    #[error("font inspection cancelled")]
    Cancelled,
    #[error("font table read: {0}")]
    Read(#[from] read_fonts::ReadError),
}
fn invalid(message: &'static str) -> FontError {
    FontError::Invalid(message)
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), FontError> {
    if check() {
        Err(FontError::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn valid_selector(cp: u32) -> bool {
    char::from_u32(cp).is_some_and(|c| mo_unicode::properties(c).variation_selector)
}
/// Immutable, borrowed font bytes bound to checked identity, metadata and cmap.
/// This value cannot be constructed from a serialized inspection. It does not
/// certify layout programs/outlines or confer embedding/distribution rights.
pub struct VerifiedFont<'a> {
    bytes: &'a [u8],
    metadata: FontInspection,
    cmap: cmap::CheckedCmap<'a>,
}
impl<'a> VerifiedFont<'a> {
    pub fn load(
        expected_sha256: &Digest,
        face_index: u32,
        bytes: &'a [u8],
        limits: FontLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, FontError> {
        load(expected_sha256, face_index, bytes, limits, check)
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    /// Contains no per-request coverage results. Call `query` separately.
    pub fn metadata(&self) -> &FontInspection {
        &self.metadata
    }
    /// Allocation-free lookup for a caller that already accounts for total work.
    pub fn query_one(
        &self,
        character: FontCharacter,
        check: &dyn Fn() -> bool,
    ) -> Result<CoverageOutcome, FontError> {
        check_characters(&[character], 1, check)?;
        Ok(self.cmap.query(character))
    }
    pub fn query(
        &self,
        characters: &[FontCharacter],
        max_queries: usize,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<FontCoverage>, FontError> {
        check_characters(characters, max_queries, check)?;
        let mut result = Vec::with_capacity(characters.len());
        for &character in characters {
            cancelled(check)?;
            result.push(FontCoverage {
                character,
                outcome: self.cmap.query(character),
            });
        }
        Ok(result)
    }
}

fn check_characters(
    characters: &[FontCharacter],
    max_queries: usize,
    check: &dyn Fn() -> bool,
) -> Result<(), FontError> {
    cancelled(check)?;
    if characters.len() > max_queries {
        return Err(FontError::Limit("character queries"));
    }
    for character in characters {
        cancelled(check)?;
        if char::from_u32(character.codepoint).is_none()
            || character
                .variation_selector
                .is_some_and(|v| !valid_selector(v))
        {
            return Err(FontError::InvalidRequest(
                "Unicode scalar or variation selector",
            ));
        }
    }
    Ok(())
}
/// Inspect a selected SFNT/TTC face, preserving source identities and exact integer metadata.
/// Directory/checksum, consumed metadata and selected Unicode cmap are checked. This is not
/// a complete font sanitizer, glyph/GSUB/GPOS validator, shaping or embedding authorization.
pub fn inspect(
    request: &FontRequest,
    bytes: &[u8],
    limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> Result<FontInspection, FontError> {
    cancelled(check)?;
    if bytes.len() > limits.max_bytes {
        return Err(FontError::Limit("font bytes"));
    }
    check_characters(&request.characters, limits.max_queries, check)?;
    let font = VerifiedFont::load(
        &request.expected_sha256,
        request.face_index,
        bytes,
        limits,
        check,
    )?;
    let coverage = font.query(&request.characters, limits.max_queries, check)?;
    Ok(FontInspection {
        coverage,
        ..font.metadata
    })
}
fn load<'a>(
    expected_sha256: &Digest,
    face_index: u32,
    bytes: &'a [u8],
    limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> Result<VerifiedFont<'a>, FontError> {
    cancelled(check)?;
    if bytes.len() > limits.max_bytes {
        return Err(FontError::Limit("font bytes"));
    }
    let mut hash = Sha256::new();
    for chunk in bytes.chunks(65536) {
        cancelled(check)?;
        hash.update(chunk);
    }
    let sha256 = Digest::from_sha256(hash.finalize().into());
    if &sha256 != expected_sha256 {
        return Err(FontError::ResourceConflict);
    }
    let container = container::inspect(bytes, face_index, limits, check)?;
    let font = FontRef::from_index(bytes, face_index)?;
    let head = font.head()?;
    if head.magic_number() != 0x5f0f3cf5 || !(16..=16384).contains(&head.units_per_em()) {
        return Err(invalid("head magic or units per em"));
    }
    let glyph_count = font.maxp()?.num_glyphs();
    if glyph_count == 0 {
        return Err(invalid("empty glyph set"));
    }
    let hhea = font.hhea()?;
    let vertical = if font.table_data(Tag::new(b"vhea")).is_some() {
        let v = font.vhea()?;
        Some(VerticalMetrics {
            ascender: v.ascender().into(),
            descender: v.descender().into(),
            line_gap: v.line_gap().into(),
        })
    } else {
        None
    };
    let os2 = metadata::os2(&font)?;
    let (names, language_tags) = metadata::names(&font, limits, check)?;
    let (axes, instances) = metadata::variations(&font, limits, check)?;
    let cmap = cmap::inspect(&font, glyph_count, check)?;
    let mut notices = vec!["Inspection does not validate glyph outlines, layout programs, variation deltas or renderability.".into()];
    if names.iter().any(|n| n.text.is_none()) {
        notices.push(
            "Some name records use an unsupported encoding; their text is unresolved.".into(),
        );
    }
    if cmap.info.is_none() {
        notices.push("No supported Unicode cmap selected; symbol and legacy encodings are not Unicode fallbacks.".into());
    }
    let metadata = FontInspection {
        sha256,
        byte_length: ByteLength::new(bytes.len() as u64),
        face_count: container.face_count,
        face_index,
        sfnt_version: container.version,
        units_per_em: head.units_per_em(),
        glyph_count,
        font_revision_16_16: head.font_revision().to_bits(),
        tables: container.tables,
        names,
        language_tags,
        metrics: FontMetrics {
            horizontal_ascender: hhea.ascender().into(),
            horizontal_descender: hhea.descender().into(),
            horizontal_line_gap: hhea.line_gap().into(),
            vertical,
        },
        os2,
        axes,
        instances,
        cmap: cmap.info.clone(),
        coverage: vec![],
        notices,
    };
    Ok(VerifiedFont {
        bytes,
        metadata,
        cmap,
    })
}

#[cfg(test)]
mod tests;
