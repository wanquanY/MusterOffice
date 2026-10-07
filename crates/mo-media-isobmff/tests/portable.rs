mod support;
use mo_media::{ErrorKind, MediaError};
use mo_media_isobmff::{IndexLimits, IndexedMovie, portable::PortableIndex};
use serde_json::{Value, json};
use support::*;

fn open(bytes: &[u8]) -> PortableIndex {
    PortableIndex::open(
        &mut |offset, out: &mut [u8]| {
            let offset = offset as usize;
            let n = out.len().min(7).min(bytes.len().saturating_sub(offset));
            out[..n].copy_from_slice(&bytes[offset..offset + n]);
            Ok(n)
        },
        bytes.len() as u64,
        IndexLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn value(result: Result<String, MediaError>) -> Value {
    serde_json::from_str(&result.unwrap()).unwrap()
}
#[test]
fn wide_integer_edits_are_not_rounded_through_json_numbers() {
    let mut f = signed_fixture();
    f.version = 1;
    let wide = (1_u64 << 54) + 3;
    f.edits = table(
        b"elst",
        1,
        &[
            edit(1, wide, -1, 65536),
            edit(1, 400, (wide + 2) as i64, 65536),
        ],
    );
    let index = open(&f.file());
    let page = value(index.query(r#"{"type":"edits","track":0,"start":0,"count":128}"#));
    assert_eq!(page["items"][0]["movieDuration"], wide.to_string());
    assert_eq!(page["items"][1]["movieStart"], wide.to_string());
    assert_eq!(page["items"][1]["mediaTime"], (wide + 2).to_string());
    assert_eq!(page["total"], 2);
    let samples = value(index.query(r#"{"type":"samples","track":0,"start":2,"count":1}"#));
    assert_eq!(samples["items"][0]["pts"], "10");
    assert_eq!(samples["items"][0]["decodeDuration"], 10);
}
#[test]
fn portable_selection_keeps_exact_direction_empty_edits_and_dwell() {
    let mut index = open(&signed_fixture().file());
    let request = |n, d, side| {
        json!({"track":0,"time":{"numerator":n,"denominator":d},"side":side}).to_string()
    };
    let before = value(index.select_video(&request("1", "10", "before"), &|| false));
    assert_eq!(before["type"], "gap");
    let after = value(index.select_video(&request("1", "10", "after"), &|| false));
    assert_eq!(after["sample"], 0);
    assert_eq!(after["rawStart"], "0");
    let dwell = value(index.select_video(&request("3", "5", "after"), &|| false));
    assert_eq!(dwell["sample"], 3);
    assert_eq!(
        dwell["mediaTime"],
        json!({"numerator":"1","denominator":"5"})
    );
    assert_eq!(
        value(index.select_video(&request("7", "10", "after"), &|| false))["type"],
        "outside"
    );
    let movie = value(index.movie(
        r#"{"track":0,"uneditedMediaEnd":{"numerator":"2","denominator":"5"},"start":1,"count":1}"#,
        &|| false,
    ));
    assert_eq!(movie["total"], 3);
    assert_eq!(movie["segments"].as_array().unwrap().len(), 1);
    assert_eq!(movie["segments"][0]["editIndex"], 1);
}
#[test]
fn invalid_pages_duplicate_fields_and_cancelled_cached_timing_fail_closed() {
    let bytes = signed_fixture().file();
    let index = open(&bytes);
    for query in [
        r#"{"type":"track","track":0,"track":1}"#,
        r#"{"type":"samples","track":0,"start":0,"count":129}"#,
        r#"{"type":"track","track":0.5}"#,
        r#"{"type":"track","track":0,"unknown":true}"#,
    ] {
        assert_eq!(index.query(query).unwrap_err().kind, ErrorKind::Invalid);
    }
    let mut session = IndexedMovie::open(
        &mut |offset, out: &mut [u8]| {
            let start = offset as usize;
            out.copy_from_slice(&bytes[start..start + out.len()]);
            Ok(out.len())
        },
        bytes.len() as u64,
        IndexLimits::default(),
        &|| false,
    )
    .unwrap();
    session.timing(0, &|| false).unwrap();
    assert_eq!(
        session.timing(0, &|| true).err().unwrap().kind,
        ErrorKind::Cancelled
    );
}
