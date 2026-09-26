use crate::PptxError;
use mo_common::ResourceId;
use mo_opc::ReaderAt;

/// Host-authorized immutable bytes; computation never discovers file paths.
pub struct ResourceData<'a> {
    pub reader: &'a dyn ReaderAt,
    pub byte_length: u64,
}
pub trait Resources {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError>;
}
pub struct NoResources;
impl Resources for NoResources {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        Err(PptxError::ResourceRequired(id.clone()))
    }
}
