//! Bounded streaming grammar shared by native fills and effect graphs.
mod effect;
mod frame;
mod read;
use super::{effects::*, fill::*};
pub(super) use read::{Budget, Declaration, Reader};
use std::collections::BTreeMap;
pub(super) fn is_effect_properties(name: &mo_xml::ExpandedName) -> bool {
    name.is(crate::A, "effectLst") || name.is(crate::A, "effectDag")
}
