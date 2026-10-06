use crate::wire::Wire;
use mo_media::{ErrorKind, MediaError};
use mo_media_isobmff::{IndexError, IndexLimits, IsoBmffIndex};
use std::{cell::RefCell, rc::Rc};
pub fn open(
    io: Rc<RefCell<Wire>>,
    length: u64,
    limits: IndexLimits,
) -> Result<IsoBmffIndex, MediaError> {
    let mut input =
        move |offset, destination: &mut [u8]| io.borrow_mut().read_source(offset, destination);
    IsoBmffIndex::read(&mut input, length, limits, &|| false).map_err(|error| {
        let kind = match &error {
            IndexError::Io(_) => ErrorKind::Io,
            IndexError::Invalid(_) => ErrorKind::Invalid,
            IndexError::Unsupported(_) => ErrorKind::Unsupported,
            IndexError::Budget => ErrorKind::Limit,
            IndexError::Cancelled => ErrorKind::Cancelled,
            IndexError::CallbackPanic => ErrorKind::CallbackPanic,
        };
        MediaError::new(kind, error.to_string())
    })
}
