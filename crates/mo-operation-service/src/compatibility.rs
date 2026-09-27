//! Translation at the old host boundary. No domain algorithms live here.
use crate::{Failure, FailureCode, OperationRequest};
use mo_presentation_operations as computation;

pub(crate) fn input(request: &OperationRequest) -> computation::Computation<'_> {
    computation::Computation {
        request_id: &request.request_id,
        profile_id: request.profile_id,
        action: &request.action,
    }
}

impl From<computation::Failure> for Failure {
    fn from(error: computation::Failure) -> Self {
        use computation::FailureCode as C;
        let code = match error.code {
            C::InputInvalid => FailureCode::InputInvalid,
            C::NotFound => FailureCode::NotFound,
            C::RequestIdReused => FailureCode::RequestIdReused,
            C::RevisionConflict => FailureCode::RevisionConflict,
            C::ReferenceConflict => FailureCode::ReferenceConflict,
            C::DocumentExists => FailureCode::DocumentExists,
            C::LimitExceeded => FailureCode::LimitExceeded,
            C::Cancelled => FailureCode::Cancelled,
            C::ExecutionInterrupted => FailureCode::ExecutionInterrupted,
            C::ExecutorMismatch => FailureCode::ExecutorMismatch,
            C::IoFailure => FailureCode::StorageFailure,
            C::ResultMismatch => FailureCode::StaleExecution,
            C::ResourceConflict => FailureCode::ResourceConflict,
            C::ResourceIncomplete => FailureCode::ResourceIncomplete,
            C::MappingNotImplemented => FailureCode::MappingNotImplemented,
            C::RenderFailure => FailureCode::RenderFailure,
        };
        Self {
            code,
            message: error.message,
            detail: error.detail,
        }
    }
}

impl From<mo_presentation_delivery::DeliveryError> for Failure {
    fn from(error: mo_presentation_delivery::DeliveryError) -> Self {
        // Preserve old host errors embedded in I/O without making the pure
        // computation crate depend on identity, leases or storage policy.
        crate::delivery_failure(error)
    }
}
