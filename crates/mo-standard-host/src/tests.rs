use super::*;

#[test]
fn actual_sqlite_build_and_connection_durability_configuration() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("mo-host-config-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    let host = StandardHost::open(
        path.join("host.sqlite"),
        Digest::from_sha256([0; 32]),
        HostLimits::default(),
    )
    .unwrap();
    let c = &host.connection;
    for (pragma, expected) in [
        ("synchronous", 2),
        ("fullfsync", 1),
        ("foreign_keys", 1),
        ("trusted_schema", 0),
    ] {
        assert_eq!(
            c.pragma_query_value(None, pragma, |r| r.get::<_, i64>(0))
                .unwrap(),
            expected,
            "{pragma}"
        );
    }
    assert_eq!(
        c.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    let version: String = c
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, "3.53.2");
    let source: String = c
        .query_row("SELECT sqlite_source_id()", [], |r| r.get(0))
        .unwrap();
    let options: Vec<String> = c
        .prepare("PRAGMA compile_options")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    println!(
        "SQLITE_BUILD_RECORD={}",
        serde_json::json!({"version":version,"sourceId":source,"compileOptions":options,"journalMode":"wal","synchronous":"FULL","fullFsync":true,"foreignKeys":true,"trustedSchema":false})
    );
    drop(host);
    std::fs::remove_dir_all(path).unwrap();
}
