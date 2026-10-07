use crate::{
    presentation::{VideoTimeSelection, VideoTimingSummary},
    query::*,
};
use serde_json::{Value, json};
pub(super) fn summary(s: IndexSummary) -> Value {
    json!({"profile":super::PROFILE,"movieTimescale":s.movie_timescale,
        "declaredMovieDuration":s.declared_movie_duration.map(|n|n.to_string()),
        "tracks":s.tracks,"mediaData":s.media_data,"bytesRead":s.bytes_read.to_string()})
}
pub(super) fn page(page: IndexPage) -> Value {
    match page {
        IndexPage::Track { track: t } => json!({"type":"track","track": {
            "id":t.id,"flags":t.flags,"handler":t.handler,"timescale":t.timescale,
            "declaredTrackDuration":t.declared_track_duration.map(|n|n.to_string()),
            "declaredMediaDuration":t.declared_media_duration.map(|n|n.to_string()),
            "decodeDuration":t.decode_duration.to_string(),"displayMatrix":t.display_matrix,
            "displaySize1616":t.display_size_16_16,"descriptions":t.descriptions,
            "samples":t.samples,"edits":t.edits,"auxiliary":t.auxiliary }}),
        IndexPage::Samples { total, items } => json!({"type":"samples","total":total,
            "items":items.iter().map(|s|json!({"bytes":{"offset":s.bytes.offset.to_string(),"length":s.bytes.length.to_string()},
                "dts":s.dts.to_string(),"pts":s.pts.to_string(),"decodeDuration":s.decode_duration,
                "description":s.description,"syncSample":s.sync_sample})).collect::<Vec<_>>() }),
        IndexPage::Edits { total, items } => json!({"type":"edits","total":total,
            "items":items.iter().map(|e|json!({"movieStart":e.movie_start.to_string(),"movieDuration":e.movie_duration.to_string(),
                "mediaTime":e.media_time.map(|n|n.to_string()),"rate1616":e.rate_16_16})).collect::<Vec<_>>() }),
        IndexPage::MediaData { total, items } => json!({"type":"mediaData","total":total,
            "items":items.iter().map(|r|json!({"offset":r.offset.to_string(),"length":r.length.to_string()})).collect::<Vec<_>>() }),
        IndexPage::Blob { kind, total, bytes } => {
            json!({"type":"blob","kind":kind,"total":total,"bytes":bytes})
        }
    }
}
pub(super) fn video_summary(s: &VideoTimingSummary) -> Value {
    json!({"trackId":s.track_id,"movieTimescale":s.movie_timescale,"mediaTimescale":s.media_timescale,
        "samples":s.samples,"edits":s.edits,"rawSpan":s.raw_span.map(|(a,b)|[a.to_string(),b.to_string()]),"movieEnd":s.movie_end})
}
pub(super) fn selection(s: VideoTimeSelection) -> Value {
    match s {
        VideoTimeSelection::Outside => json!({"type":"outside"}),
        VideoTimeSelection::Gap { movie, edit } => json!({"type":"gap","movie":movie,"edit":edit}),
        VideoTimeSelection::Sample {
            sample,
            decode_start,
            access,
            edit,
            movie,
            raw_start,
            raw_end,
            media_time,
            media_side,
        } => json!({"type":"sample","sample":sample,"decodeStart":decode_start.to_string(),
                "access":access.map(|a|json!({"sample":a.sample,"decodeStart":a.decode_start.to_string()})),
                "edit":edit,"movie":movie,"rawStart":raw_start.to_string(),"rawEnd":raw_end.to_string(),
                "mediaTime":media_time,"mediaSide":media_side}),
    }
}
