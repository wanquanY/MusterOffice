//! Explicit external fixtures, also used as the native oracle for public WASM.
use mo_media_isobmff::{IndexLimits, portable::PortableIndex};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};

#[test]
#[ignore = "requires MO_AVC_INPUT / MO_AVC_OUTPUT real fixture paths"]
fn index_real_avc_metadata() {
    let path = std::env::var("MO_AVC_INPUT").expect("fixture path");
    let output = std::env::var("MO_AVC_OUTPUT").expect("report path");
    let mut file = std::fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    let mut read_bytes = 0;
    let mut reader = |offset, out: &mut [u8]| {
        file.seek(SeekFrom::Start(offset))?;
        let size = out.len().min(17);
        let count = file.read(&mut out[..size])?;
        read_bytes += count;
        Ok(count)
    };
    let mut index =
        PortableIndex::open(&mut reader, length, IndexLimits::default(), &|| false).unwrap();
    let summary: Value = serde_json::from_str(&index.summary().unwrap()).unwrap();
    let mut tracks = Vec::new();
    for track in 0..summary["tracks"].as_u64().unwrap() {
        let info: Value = serde_json::from_str(
            &index
                .query(&json!({"type":"track","track":track}).to_string())
                .unwrap(),
        )
        .unwrap();
        if info["track"]["handler"] != json!([118, 105, 100, 101]) {
            continue;
        }
        let samples = info["track"]["samples"].as_u64().unwrap();
        if samples == 0 {
            continue;
        }
        let mut pages = 0;
        loop {
            let progress: Value = serde_json::from_str(
                &index
                    .scan_avc(
                        &json!({"track":track,"through":samples - 1,"count":11}).to_string(),
                        &mut reader,
                        &|| false,
                    )
                    .unwrap(),
            )
            .unwrap();
            pages += 1;
            if progress["ready"] == true {
                break;
            }
        }
        assert_eq!(pages, samples.div_ceil(11));
        let mut pictures = vec![];
        for sample in 0..samples {
            let request = json!({"track":track,"sample":sample}).to_string();
            let value: Value = serde_json::from_str(&index.avc_sample(&request).unwrap()).unwrap();
            let bootstrap = index.avc_bootstrap(&request).unwrap();
            pictures.push(json!({"metadata":value,"bootstrapSha256":format!("{:x}", Sha256::digest(bootstrap))}));
        }
        // A cached backwards query never rescans or rewrites a parameter set.
        for sample in [samples - 1, 0, samples / 2] {
            let request = json!({"track":track,"sample":sample}).to_string();
            let value: Value = serde_json::from_str(&index.avc_sample(&request).unwrap()).unwrap();
            assert_eq!(value, pictures[sample as usize]["metadata"]);
        }
        tracks.push(json!({"track":track,"pictures":pictures}));
    }
    assert!(!tracks.is_empty());
    std::fs::write(
        output,
        serde_json::to_vec_pretty(
            &json!({"tracks":tracks,"readBytes":read_bytes,"fileBytes":length}),
        )
        .unwrap(),
    )
    .unwrap();
}
