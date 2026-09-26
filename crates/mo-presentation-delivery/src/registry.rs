//! Compiled delivery composition, not the complete native-format catalogue.
//! Keeping renderer instance identity in the context makes this registry stable
//! across deployments of the same semantic profiles.
pub(crate) const MIME: &str = "application/vnd.musteroffice.feature-registry+json";

pub(crate) fn snapshot() -> serde_json::Value {
    serde_json::json!({
        "format": "musteroffice.feature-registry/1-draft",
        "scope": "delivery-composition",
        "status": "development",
        "completeNativeCatalogue": false,
        "fullPresentationCapability": false,
        "targetApplicationValidated": false,
        "entries": [
            {
                "namespace": "musteroffice.delivery",
                "kind": "pipeline",
                "nativeId": crate::PROFILE,
                "parameters": {
                    "settingsSchema": "presentation-delivery-settings",
                    "formats": ["pptx"],
                    "previewSample": "editor",
                    "fontPolicy": "explicit-resources-only",
                    "publication": "host-required"
                },
                "semanticVersion": "1-draft",
                "capabilityId": "F13.04",
                "targets": ["native"],
                "fixtureIds": ["presentations/delivery"],
                "implementationStatus": "partial",
                "acceptanceStatus": "not_accepted",
                "portableCoreStatus": "wasm_compile_only"
            },
            {
                "namespace": "musteroffice.presentation",
                "kind": "page-preview",
                "nativeId": mo_presentation_compile::source_resource_page::PROFILE,
                "parameters": {
                    "settingsSchema": "pptx-resource-page-request",
                    "sample": "editor",
                    "unsupportedContent": "reject",
                    "fonts": "explicit-resources-only"
                },
                "semanticVersion": "1-draft",
                "capabilityId": "F14.01",
                "targets": ["native", "wasm"],
                "fixtureIds": ["presentations/delivery"],
                "implementationStatus": "partial",
                "acceptanceStatus": "not_accepted"
            }
        ]
    })
}
