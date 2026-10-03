//! Resource-page capabilities distinguish native OPC packages from authored image graphs.
//! Author resource addresses never impersonate a package or expose chart bytes.
use super::images::{AuthorImages, ImageInput, PackageImages};
use mo_opc::PackageRead;
pub trait PageInput: ImageInput {
    fn native_package(&self) -> Option<&dyn PackageRead> {
        None
    }
}
impl PageInput for PackageImages<'_> {
    fn native_package(&self) -> Option<&dyn PackageRead> {
        Some(self.0)
    }
}
impl PageInput for AuthorImages<'_> {}
