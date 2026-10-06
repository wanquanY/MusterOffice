use crate::boxes::{children, fourcc, full, optional, required, u16be, u32be, u64be};
use crate::{
    Budget, ByteRange, Edit, IndexError as E, Result, SampleAuxiliary, SampleDescription,
    TrackIndex, reserve,
};

pub(crate) fn clock(data: &[u8]) -> Result<(u32, Option<u64>)> {
    let version = full(data, &[0, 1])?;
    let (scale, duration) = if version == 0 {
        (
            u32be(data, 12)?,
            match u32be(data, 16)? {
                u32::MAX => None,
                v => Some(u64::from(v)),
            },
        )
    } else {
        (
            u32be(data, 20)?,
            match u64be(data, 24)? {
                u64::MAX => None,
                v => Some(v),
            },
        )
    };
    if scale == 0 {
        return Err(E::Invalid("zero timescale"));
    }
    Ok((scale, duration))
}
pub(crate) fn read_track(
    data: &[u8],
    media: &[ByteRange],
    budget: &mut Budget<'_>,
) -> Result<TrackIndex> {
    let track = children(data, budget)?;
    let tkhd = required(&track, b"tkhd")?;
    let version = *tkhd.first().ok_or(E::Invalid("track header"))?;
    let (id, matrix_at) = match version {
        0 if tkhd.len() == 84 => (u32be(tkhd, 12)?, 40),
        1 if tkhd.len() == 96 => (u32be(tkhd, 20)?, 52),
        _ => return Err(E::Unsupported("track header version/size")),
    };
    if id == 0 {
        return Err(E::Invalid("zero track ID"));
    }
    let flags = u32be(tkhd, 0)? & 0x00ff_ffff;
    let declared_track_duration = if version == 0 {
        match u32be(tkhd, 20)? {
            u32::MAX => None,
            v => Some(u64::from(v)),
        }
    } else {
        match u64be(tkhd, 28)? {
            u64::MAX => None,
            v => Some(v),
        }
    };
    let mut display_matrix = [0; 9];
    for (i, v) in display_matrix.iter_mut().enumerate() {
        *v = u32be(tkhd, matrix_at + i * 4)? as i32;
    }
    let display_size_16_16 = [u32be(tkhd, matrix_at + 36)?, u32be(tkhd, matrix_at + 40)?];
    let edits = match optional(&track, b"edts")? {
        None => vec![],
        Some(data) => read_edits(required(&children(data, budget)?, b"elst")?, budget)?,
    };
    let mdia = children(required(&track, b"mdia")?, budget)?;
    let (timescale, declared_media_duration) = clock(required(&mdia, b"mdhd")?)?;
    let hdlr = required(&mdia, b"hdlr")?;
    full(hdlr, &[0])?;
    let handler = fourcc(hdlr, 8)?;
    let minf = children(required(&mdia, b"minf")?, budget)?;
    let dinf = children(required(&minf, b"dinf")?, budget)?;
    let dref = required(&dinf, b"dref")?;
    full(dref, &[0])?;
    let refs = u32be(dref, 4)? as usize;
    let references = children(dref.get(8..).ok_or(E::Invalid("data reference"))?, budget)?;
    if refs == 0 || references.len() != refs {
        return Err(E::Invalid("data reference count"));
    }
    for reference in &references {
        // The self-contained URL flag has no actual URL. External/URN references
        // are explicitly refused; the parser never grants network/file access.
        if reference.kind != *b"url " || reference.data != [0, 0, 0, 1] {
            return Err(E::Unsupported("external or unknown data reference"));
        }
    }
    let stbl = children(required(&minf, b"stbl")?, budget)?;
    for unsupported in [b"cslg", b"senc", b"saio", b"saiz", b"stsh"] {
        if optional(&stbl, unsupported)?.is_some() {
            return Err(E::Unsupported(
                "composition shift, protected or shadow samples",
            ));
        }
    }
    let stsd = required(&stbl, b"stsd")?;
    full(stsd, &[0])?;
    let count = u32be(stsd, 4)? as usize;
    budget.descriptions = budget.descriptions.checked_add(count).ok_or(E::Budget)?;
    if count == 0 || budget.descriptions > budget.limits.max_sample_descriptions {
        return Err(E::Budget);
    }
    let entries = children(
        stsd.get(8..).ok_or(E::Invalid("sample description"))?,
        budget,
    )?;
    if entries.len() != count {
        return Err(E::Invalid("sample description count"));
    }
    let mut descriptions = Vec::new();
    reserve(&mut descriptions, count)?;
    for entry in entries {
        budget.active()?;
        if matches!(&entry.kind, b"enca" | b"encv") {
            return Err(E::Unsupported("encrypted samples"));
        }
        if entry.data.get(..6) != Some(&[0; 6]) {
            return Err(E::Invalid("sample description reserved bytes"));
        }
        let reference = usize::from(u16be(entry.data, 6)?);
        if reference == 0 || reference > refs {
            return Err(E::Invalid("sample data reference index"));
        }
        if entry.data.len() > budget.limits.max_description_bytes {
            return Err(E::Budget);
        }
        let mut payload = Vec::new();
        reserve(&mut payload, entry.data.len())?;
        payload.extend_from_slice(entry.data);
        descriptions.push(SampleDescription {
            format: entry.kind,
            payload,
        });
    }
    let (samples, decode_duration) =
        crate::tables::samples(&stbl, descriptions.len(), media, budget)?;
    let mut auxiliary = Vec::new();
    for entry in stbl
        .iter()
        .filter(|b| matches!(&b.kind, b"sdtp" | b"sbgp" | b"sgpd" | b"stdp" | b"stps"))
    {
        budget.active()?;
        let mut payload = Vec::new();
        reserve(&mut payload, entry.data.len())?;
        payload.extend_from_slice(entry.data);
        reserve(&mut auxiliary, 1)?;
        auxiliary.push(SampleAuxiliary {
            kind: entry.kind,
            payload,
        });
    }
    Ok(TrackIndex {
        id,
        flags,
        handler,
        declared_track_duration,
        timescale,
        declared_media_duration,
        decode_duration,
        display_matrix,
        display_size_16_16,
        descriptions,
        samples,
        edits,
        auxiliary,
    })
}
fn read_edits(data: &[u8], budget: &mut Budget<'_>) -> Result<Vec<Edit>> {
    let version = full(data, &[0, 1])?;
    let (_, count) = crate::tables::table(
        data,
        if version == 0 { 12 } else { 20 },
        budget.limits.max_edits,
        &[0, 1],
    )?;
    budget.edits = budget.edits.checked_add(count).ok_or(E::Budget)?;
    if budget.edits > budget.limits.max_edits {
        return Err(E::Budget);
    }
    let mut result = Vec::new();
    reserve(&mut result, count)?;
    let mut movie_start = 0u64;
    for i in 0..count {
        budget.active()?;
        let (movie_duration, media_time, rate_16_16) = if version == 0 {
            let at = 8 + i * 12;
            (
                u64::from(u32be(data, at)?),
                i64::from(u32be(data, at + 4)? as i32),
                u32be(data, at + 8)? as i32,
            )
        } else {
            let at = 8 + i * 20;
            (
                u64be(data, at)?,
                u64be(data, at + 8)? as i64,
                u32be(data, at + 16)? as i32,
            )
        };
        if media_time < -1 {
            return Err(E::Invalid("negative edit media time"));
        }
        let media_time = if media_time == -1 {
            None
        } else {
            Some(media_time)
        };
        result.push(Edit {
            movie_start,
            movie_duration,
            media_time,
            rate_16_16,
        });
        movie_start = movie_start
            .checked_add(movie_duration)
            .ok_or(E::Invalid("movie edit duration overflow"))?;
    }
    Ok(result)
}
