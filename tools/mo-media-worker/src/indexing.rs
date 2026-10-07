use crate::wire::Wire;
use mo_media::MediaError;
use mo_media_isobmff::IndexLimits;
pub use mo_media_isobmff::{IndexedMovie as Indexed, timing_error};
use std::{cell::RefCell, rc::Rc};
pub fn open(
    io: Rc<RefCell<Wire>>,
    length: u64,
    limits: IndexLimits,
) -> Result<Indexed, MediaError> {
    let mut input =
        move |offset, destination: &mut [u8]| io.borrow_mut().read_source(offset, destination);
    Indexed::open(&mut input, length, limits, &|| false)
}
