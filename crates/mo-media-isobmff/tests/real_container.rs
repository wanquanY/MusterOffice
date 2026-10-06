//! Explicit real-media evidence; no fixture path is compiled into the library.
use mo_media_isobmff::{IndexLimits, IsoBmffIndex};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};

#[test]
#[ignore = "requires real input and output paths for independent packet comparison"]
fn index_real_container() {
    let input = std::env::var("MO_BMFF_INPUT").expect("explicit real input");
    let output = std::env::var("MO_BMFF_OUTPUT").expect("explicit output");
    let mut file = std::fs::File::open(&input).expect("open input");
    let length = file.metadata().expect("input length").len();
    let index = IsoBmffIndex::read(
        &mut |offset, bytes: &mut [u8]| {
            file.seek(SeekFrom::Start(offset))?;
            let limit = bytes.len().min(13);
            file.read(&mut bytes[..limit])
        },
        length,
        IndexLimits::default(),
        &|| false,
    )
    .expect("index actual container");
    let mut tracks = Vec::new();
    for track in index.tracks {
        let mut samples = Vec::new();
        for sample in track.samples {
            file.seek(SeekFrom::Start(sample.bytes.offset))
                .expect("read indexed sample");
            let mut bytes = vec![0; sample.bytes.length as usize];
            file.read_exact(&mut bytes)
                .expect("complete indexed packet");
            samples.push(json!({"dts":sample.dts,"pts":sample.pts,"duration":sample.decode_duration,
                "offset":sample.bytes.offset,"byteLength":sample.bytes.length,"sync":sample.sync_sample,
                "description":sample.description,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
        }
        tracks.push(json!({"id":track.id,"handler":String::from_utf8_lossy(&track.handler),
                "timescale":track.timescale,"declaredDuration":track.declared_media_duration,"decodeDuration":track.decode_duration,
                "flags":track.flags,"trackDuration":track.declared_track_duration,"displayMatrix":track.display_matrix,"displaySize16_16":track.display_size_16_16,
            "edits":track.edits.iter().map(|e|json!({"movieStart":e.movie_start,"movieDuration":e.movie_duration,"mediaTime":e.media_time,"rate16_16":e.rate_16_16})).collect::<Vec<_>>(),
            "samples":samples}));
    }
    let value = json!({"movieTimescale":index.movie_timescale,"declaredMovieDuration":index.declared_movie_duration,
        "bytesRead":index.bytes_read,"fileLength":length,"tracks":tracks});
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&value).expect("serialize evidence"),
    )
    .expect("write evidence");
}
