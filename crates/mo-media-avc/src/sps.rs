use crate::{Result, Sequence, VuiColor, bits::Bits, invalid, unsupported};

#[derive(Clone, Debug)]
pub(crate) struct Sps {
    pub info: Sequence,
    pub frame_bits: u8,
    pub poc_type: u32,
    pub poc_bits: u8,
    pub delta_zero: bool,
    pub separate_planes: bool,
}
pub(crate) fn parse(bytes: &[u8]) -> Result<Sps> {
    let mut b = Bits::new(&bytes[1..]);
    let profile = b.u(8)? as u8;
    if !matches!(profile, 44 | 66 | 77 | 88 | 100 | 110 | 122 | 244) {
        return Err(unsupported("AVC profile extensions"));
    }
    if b.u(8)? & 3 != 0 {
        return Err(invalid("reserved SPS bits"));
    }
    let level = b.u(8)? as u8;
    let id = b.ue(31)? as u8;
    let (mut chroma, mut depth_y, mut depth_c, mut separate) = (1, 8, 8, false);
    if matches!(profile, 44 | 100 | 110 | 122 | 244) {
        chroma = b.ue(3)? as u8;
        if chroma == 3 {
            separate = b.flag()?;
        }
        depth_y += b.ue(6)? as u8;
        depth_c += b.ue(6)? as u8;
        b.flag()?; // qpprime_y_zero_transform_bypass_flag
        if b.flag()? {
            for list in 0..if chroma == 3 { 12 } else { 8 } {
                if b.flag()? {
                    scaling(&mut b, if list < 6 { 16 } else { 64 })?;
                }
            }
        }
    }
    let frame_bits = b.ue(12)? as u8 + 4;
    let poc_type = b.ue(2)?;
    let (mut poc_bits, mut delta_zero) = (0, false);
    if poc_type == 0 {
        poc_bits = b.ue(12)? as u8 + 4;
    }
    if poc_type == 1 {
        delta_zero = b.flag()?;
        b.se()?;
        b.se()?;
        for _ in 0..b.ue(255)? {
            b.se()?;
        }
    }
    let max_references = b.ue(16)? as u8;
    b.flag()?;
    // Public metadata bounds, independent from any decoder's level admission.
    let mb_width = b.ue(1023)? + 1;
    let mb_height = b.ue(1023)? + 1;
    let frame_only = b.flag()?;
    if !frame_only {
        b.flag()?;
    }
    b.flag()?;
    let crop = if b.flag()? {
        [b.ue(16384)?, b.ue(16384)?, b.ue(32768)?, b.ue(32768)?]
    } else {
        [0; 4]
    };
    let width = mb_width * 16;
    let height = mb_height * 16 * if frame_only { 1 } else { 2 };
    let array_type = if separate { 0 } else { chroma };
    let unit_x = if matches!(array_type, 1 | 2) { 2 } else { 1 };
    let unit_y = if array_type == 1 { 2 } else { 1 } * if frame_only { 1 } else { 2 };
    let left = crop[0] * unit_x;
    let right = crop[1] * unit_x;
    let top = crop[2] * unit_y;
    let bottom = crop[3] * unit_y;
    if left + right >= width || top + bottom >= height {
        return Err(invalid("SPS crop"));
    }
    let mut info = Sequence {
        id,
        profile,
        level,
        chroma_format: chroma,
        bit_depth_luma: depth_y,
        bit_depth_chroma: depth_c,
        coded_width: width,
        coded_height: height,
        visible: [left, top, width - left - right, height - top - bottom],
        frame_mbs_only: frame_only,
        separate_colour_plane: separate,
        max_references,
        reorder_frames: None,
        decoder_buffer_frames: None,
        color: VuiColor::default(),
        chroma_location_top: 0,
        chroma_location_bottom: 0,
        chroma_location_present: false,
    };
    if b.flag()? {
        vui(&mut b, &mut info)?;
    }
    b.trailing()?;
    Ok(Sps {
        info,
        frame_bits,
        poc_type,
        poc_bits,
        delta_zero,
        separate_planes: separate,
    })
}
fn scaling(b: &mut Bits<'_>, count: u32) -> Result<()> {
    let (mut last, mut next) = (8i64, 8i64);
    for _ in 0..count {
        if next != 0 {
            let delta = b.se()?;
            if !(-128..=127).contains(&delta) {
                return Err(invalid("scaling delta"));
            }
            next = (last + i64::from(delta) + 256) % 256;
        }
        if next != 0 {
            last = next;
        }
    }
    Ok(())
}
fn vui(b: &mut Bits<'_>, info: &mut Sequence) -> Result<()> {
    if b.flag()? && b.u(8)? == 255 {
        b.u(16)?;
        b.u(16)?;
    }
    if b.flag()? {
        b.flag()?;
    }
    info.color.signal_present = b.flag()?;
    if info.color.signal_present {
        b.u(3)?;
        info.color.full_range = b.flag()?;
        info.color.description_present = b.flag()?;
        if info.color.description_present {
            info.color.primaries = b.u(8)? as u8;
            info.color.transfer = b.u(8)? as u8;
            info.color.matrix = b.u(8)? as u8;
        }
    }
    info.chroma_location_present = b.flag()?;
    if info.chroma_location_present {
        info.chroma_location_top = b.ue(5)? as u8;
        info.chroma_location_bottom = b.ue(5)? as u8;
    }
    if b.flag()? {
        if b.u(32)? == 0 || b.u(32)? == 0 {
            return Err(invalid("VUI timing"));
        }
        b.flag()?;
    }
    let nal_hrd = b.flag()?;
    if nal_hrd {
        hrd(b)?;
    }
    let vcl_hrd = b.flag()?;
    if vcl_hrd {
        hrd(b)?;
    }
    if nal_hrd || vcl_hrd {
        b.flag()?;
    }
    b.flag()?;
    if b.flag()? {
        b.flag()?;
        b.ue(16)?;
        b.ue(16)?;
        b.ue(16)?;
        b.ue(16)?;
        let reorder = b.ue(16)? as u8;
        let buffer = b.ue(16)? as u8;
        if reorder > buffer || buffer < info.max_references {
            return Err(invalid("VUI decoder buffer"));
        }
        info.reorder_frames = Some(reorder);
        info.decoder_buffer_frames = Some(buffer);
    }
    Ok(())
}
fn hrd(b: &mut Bits<'_>) -> Result<()> {
    let count = b.ue(31)? + 1;
    b.u(8)?;
    for _ in 0..count {
        b.ue(u32::MAX - 1)?;
        b.ue(u32::MAX - 1)?;
        b.flag()?;
    }
    b.u(20)?;
    Ok(())
}
