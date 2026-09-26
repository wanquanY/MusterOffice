use mo_operation_service::*;

#[test]
fn every_exposed_schema_matches_codegen_and_has_a_canonical_identity() {
    for id in SchemaId::ALL {
        let identifier = serde_json::to_string(&id).unwrap();
        let name = identifier.trim_matches('"');
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../contracts/generated/{name}.schema.json"));
        let file: serde_json::Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        let document = id.document().unwrap();
        assert_eq!(document.schema, file);
        assert_eq!(document.id, id);
        assert_eq!(
            document.digest,
            mo_common::digest("musteroffice.operation-schema/1", &(id, &file)).unwrap()
        );
        let portable: SchemaDocument =
            serde_json::from_str(&operation_schema_json(&identifier).unwrap()).unwrap();
        assert_eq!(portable.digest, document.digest);
        assert_eq!(portable.schema, file);
    }
    assert_eq!(
        operation_schema_json("\"../../private\"").unwrap_err().code,
        FailureCode::InputInvalid
    );
    assert_eq!(
        operation_schema_json("{\"id\":\"document\"}")
            .unwrap_err()
            .code,
        FailureCode::InputInvalid
    );
    assert_eq!(
        operation_schema_json(&" ".repeat(129)).unwrap_err().code,
        FailureCode::LimitExceeded
    );
}

#[test]
fn catalogue_uses_the_export_authority_intersection() {
    let mut context = CallContext {
        principal: PrincipalId::new("agent").unwrap(),
        scope: ScopeId::new("scope").unwrap(),
        permissions: Default::default(),
    };
    assert!(ServiceOperation::Schema.authorize(&context).is_ok());
    for permission in [
        Permission::Export,
        Permission::ReadDocument,
        Permission::ReadAssets,
    ] {
        assert_eq!(
            ServiceOperation::Export
                .authorize(&context)
                .unwrap_err()
                .code,
            FailureCode::NotAuthorized
        );
        context.permissions.insert(permission);
    }
    assert!(ServiceOperation::Export.authorize(&context).is_ok());
    assert_eq!(
        ServiceOperation::Export.profile(),
        Some(OperationProfile::ResourceDelivery)
    );
    assert_eq!(ServiceOperation::ReadDocument.profile(), None);
    let identifiers = ServiceOperation::ALL
        .iter()
        .map(|v| serde_json::to_string(v).unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(identifiers.len(), ServiceOperation::ALL.len());
}
