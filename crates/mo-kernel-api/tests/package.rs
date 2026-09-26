use mo_kernel_api::{PackageErrorCode, PackageInspectionResponse, inspect_package};
use mo_opc::PackageLimits;

#[test]
fn inspection_preserves_budget_and_cancellation_error_categories() {
    let limits = PackageLimits {
        max_package_bytes: 0,
        ..PackageLimits::default()
    };
    let result = inspect_package(b"x".as_slice(), 1, limits, &|| false);
    assert!(
        matches!(result, PackageInspectionResponse::Error { error } if matches!(error.code, PackageErrorCode::LimitExceeded))
    );
    let result = inspect_package(b"x".as_slice(), 1, PackageLimits::default(), &|| true);
    assert!(
        matches!(result, PackageInspectionResponse::Error { error } if matches!(error.code, PackageErrorCode::Cancelled))
    );
}
