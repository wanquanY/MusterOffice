use crate::{Failure, FailureCode};

pub(crate) fn check_size(
    value: &impl serde::Serialize,
    limit: usize,
    label: &'static str,
    check: &dyn Fn() -> bool,
) -> Result<(), Failure> {
    mo_common::check_json_size(value, limit, check)
        .map(|_| ())
        .map_err(|error| match error {
            mo_common::JsonBudgetError::Cancelled => {
                Failure::new(FailureCode::Cancelled, "computation cancelled")
            }
            mo_common::JsonBudgetError::Limit => Failure::new(FailureCode::LimitExceeded, label),
            mo_common::JsonBudgetError::Serialization => {
                Failure::new(FailureCode::InputInvalid, "computation input serialization")
            }
        })
}
