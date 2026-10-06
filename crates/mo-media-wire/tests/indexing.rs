use mo_media_isobmff::{ByteRange, IndexLimits, Sample};
use mo_media_wire::{
    Event, IndexPage, IndexQuery, IndexSummary, MAX_HEADER_BYTES, Response, header_bytes,
};
fn summary() -> IndexSummary {
    IndexSummary {
        movie_timescale: 1000,
        declared_movie_duration: Some(1),
        tracks: 2,
        media_data: 1,
        bytes_read: 100,
    }
}
#[test]
fn page_budgets_and_response_kind_are_checked() {
    let limits = IndexLimits::default();
    let valid = IndexQuery::Samples {
        track: 1,
        start: 1,
        count: 2,
    };
    let sample = Sample {
        bytes: ByteRange {
            offset: 10,
            length: 2,
        },
        dts: 1,
        pts: 2,
        decode_duration: 1,
        description: 0,
        sync_sample: true,
    };
    let page = IndexPage::Samples {
        total: 3,
        items: vec![sample; 2],
    };
    assert!(page.validate(valid, summary(), limits, 100).is_ok());
    for query in [
        IndexQuery::Samples {
            track: 2,
            start: 1,
            count: 2,
        },
        IndexQuery::Samples {
            track: 1,
            start: 1,
            count: 0,
        },
        IndexQuery::Samples {
            track: 1,
            start: 1,
            count: 129,
        },
        IndexQuery::Samples {
            track: 1,
            start: 4,
            count: 2,
        },
        IndexQuery::Edits {
            track: 1,
            start: 1,
            count: 2,
        },
        IndexQuery::Description {
            track: 1,
            index: 0,
            offset: 0,
            length: 4097,
        },
    ] {
        assert!(page.validate(query, summary(), limits, 100).is_err());
    }
    for bad in [
        IndexPage::Samples {
            total: 4,
            items: vec![sample],
        },
        IndexPage::Samples {
            total: u32::MAX,
            items: vec![sample; 2],
        },
        IndexPage::Samples {
            total: 3,
            items: vec![
                Sample {
                    bytes: ByteRange {
                        offset: 99,
                        length: 2
                    },
                    ..sample
                };
                2
            ],
        },
        IndexPage::Samples {
            total: 3,
            items: vec![
                Sample {
                    decode_duration: 0,
                    ..sample
                };
                2
            ],
        },
    ] {
        assert!(bad.validate(valid, summary(), limits, 100).is_err());
    }
    let end = IndexPage::Samples {
        total: 3,
        items: vec![],
    };
    assert!(
        end.validate(
            IndexQuery::Samples {
                track: 0,
                start: 3,
                count: 128
            },
            summary(),
            limits,
            100
        )
        .is_ok()
    );
    let blob = IndexPage::Blob {
        kind: *b"avcC",
        total: 5,
        bytes: vec![1; 2],
    };
    assert!(
        blob.validate(
            IndexQuery::Description {
                track: 0,
                index: 0,
                offset: 3,
                length: 4096
            },
            summary(),
            limits,
            100
        )
        .is_ok()
    );
    assert!(
        blob.validate(
            IndexQuery::Description {
                track: 0,
                index: 0,
                offset: 4,
                length: 4096
            },
            summary(),
            limits,
            100
        )
        .is_err()
    );
}
#[test]
fn maximum_numeric_samples_and_blob_pages_fit_the_existing_header_budget() {
    let sample = Sample {
        bytes: ByteRange {
            offset: u64::MAX - 1,
            length: 1,
        },
        dts: i64::MIN,
        pts: i64::MAX,
        decode_duration: u32::MAX,
        description: 127,
        sync_sample: false,
    };
    for page in [
        IndexPage::Samples {
            total: 2_000_000,
            items: vec![sample; 128],
        },
        IndexPage::Blob {
            kind: [255; 4],
            total: 16 << 20,
            bytes: vec![255; 4096],
        },
    ] {
        let bytes = header_bytes(&Response {
            call_id: u64::MAX,
            event: Event::IndexPage { page },
        })
        .unwrap();
        assert!(bytes.len() < MAX_HEADER_BYTES);
    }
    assert!(
        IndexSummary {
            tracks: 0,
            ..summary()
        }
        .validate(IndexLimits::default(), 100)
        .is_err()
    );
    assert!(summary().validate(IndexLimits::default(), 99).is_err());
}
