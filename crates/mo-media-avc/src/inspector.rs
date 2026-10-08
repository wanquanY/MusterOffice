use crate::{
    MAX_ACTIVE_PARAMETERS, MAX_NALS_PER_ACCESS_UNIT, MAX_PARAMETER_BYTES, Picture, Result,
    bits::Bits,
    invalid, limit,
    sps::{self, Sps},
    unsupported,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct Parameter<T> {
    bytes: Arc<[u8]>,
    revision: u32,
    value: T,
}
#[derive(Clone, Debug)]
struct Pps {
    id: u8,
    sps: u8,
    bottom_poc_present: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct PictureKey {
    pps: u8,
    frame: u32,
    plane: u32,
    field: bool,
    bottom: bool,
    reference: bool,
    idr: Option<u32>,
    poc_lsb: u32,
    delta_bottom: i32,
    deltas: [i32; 2],
}
#[derive(Default)]
pub(crate) struct AccessUnit {
    key: Option<PictureKey>,
    picture: Option<Picture>,
}
impl AccessUnit {
    pub fn finish(self) -> Option<Picture> {
        self.picture
    }
}

/// Clone is a bounded, shared immutable snapshot of parameter NALs. An access
/// unit commits all updates only on success; malformed suffixes cannot alter
/// the state used by the next request. No decoded frames are retained here.
#[derive(Clone, Debug)]
pub struct AvcInspector {
    length_size: u8,
    sps: [Option<Arc<Parameter<Sps>>>; 32],
    pps: [Option<Arc<Parameter<Pps>>>; 256],
    bytes: usize,
    revision: u32,
}
impl AvcInspector {
    pub fn from_avcc(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_ACTIVE_PARAMETERS {
            return Err(limit("avcC bytes"));
        }
        if bytes.len() < 7 || bytes[0] != 1 || bytes[4] & 0xfc != 0xfc || bytes[5] & 0xe0 != 0xe0 {
            return Err(invalid("avcC header"));
        }
        let length_size = (bytes[4] & 3) + 1;
        if length_size == 3 {
            return Err(invalid("reserved AVC NAL length size"));
        }
        let mut owner = Self {
            length_size,
            sps: std::array::from_fn(|_| None),
            pps: std::array::from_fn(|_| None),
            bytes: 0,
            revision: 0,
        };
        let mut at = 6;
        let count = bytes[5] & 31;
        for _ in 0..count {
            owner.parameter(config_nal(bytes, &mut at)?, 7)?;
        }
        let count = *bytes.get(at).ok_or_else(|| invalid("avcC PPS count"))?;
        at += 1;
        for _ in 0..count {
            owner.parameter(config_nal(bytes, &mut at)?, 8)?;
        }
        // AVC file records in the wild may omit the optional high-profile tail.
        // Validate a present tail; SPS extensions require another syntax family.
        if at != bytes.len() {
            if !matches!(bytes[1], 100 | 110 | 122 | 144 | 244)
                || bytes.len() - at < 4
                || bytes[at] & 0xfc != 0xfc
                || bytes[at + 1] & 0xf8 != 0xf8
                || bytes[at + 2] & 0xf8 != 0xf8
            {
                return Err(invalid("avcC extension"));
            }
            if bytes[at + 3] != 0 {
                return Err(unsupported("SPS extensions"));
            }
            at += 4;
        }
        if at != bytes.len() {
            return Err(invalid("avcC trailing bytes"));
        }
        Ok(owner)
    }
    pub fn length_size(&self) -> u8 {
        self.length_size
    }
    pub fn revision(&self) -> u32 {
        self.revision
    }
    pub fn parameter_bytes(&self) -> usize {
        self.bytes
    }
    /// Length-prefixed SPS/PPS state BEFORE a requested access unit. A random
    /// access decoder must receive this state before that unit, including sets
    /// defined in an earlier in-band update and absent from the original avcC.
    pub fn bootstrap(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(self.bytes + 288 * usize::from(self.length_size));
        for nal in self
            .sps
            .iter()
            .flatten()
            .map(|p| p.bytes.as_ref())
            .chain(self.pps.iter().flatten().map(|p| p.bytes.as_ref()))
        {
            if (nal.len() as u64) >= 1u64 << (8 * self.length_size) {
                return Err(limit("bootstrap NAL length"));
            }
            bytes.extend_from_slice(
                &(nal.len() as u32).to_be_bytes()[4 - usize::from(self.length_size)..],
            );
            bytes.extend_from_slice(nal);
        }
        Ok(bytes)
    }
    pub fn inspect(&mut self, bytes: &[u8], check: &dyn Fn() -> bool) -> Result<Option<Picture>> {
        if bytes.is_empty() || bytes.len() > 64 << 20 {
            return Err(limit("AVC access unit bytes"));
        }
        let mut next = self.clone();
        let mut unit = AccessUnit::default();
        let mut at = 0;
        let mut count = 0;
        while at < bytes.len() {
            crate::active(check)?;
            count += 1;
            if count > MAX_NALS_PER_ACCESS_UNIT {
                return Err(limit("access unit NAL count"));
            }
            let end = at + usize::from(self.length_size);
            let size = length(bytes.get(at..end).ok_or_else(|| invalid("NAL length"))?);
            at = end;
            let end = at
                .checked_add(size)
                .filter(|end| *end <= bytes.len())
                .ok_or_else(|| invalid("NAL extent"))?;
            next.nal(&bytes[at..end], &mut unit)?;
            at = end;
        }
        crate::active(check)?;
        *self = next;
        Ok(unit.finish())
    }
    pub(crate) fn nal(&mut self, bytes: &[u8], unit: &mut AccessUnit) -> Result<()> {
        let (kind, reference) = header(bytes)?;
        match kind {
            7 | 8 => {
                if unit.picture.is_some() {
                    return Err(invalid("parameter set after picture in access unit"));
                }
                self.parameter(bytes, kind)?;
            }
            1 | 5 => {
                let (key, picture) = self.picture(bytes, kind == 5, reference)?;
                if unit.key.as_ref().is_some_and(|previous| previous != &key)
                    || unit
                        .picture
                        .as_ref()
                        .is_some_and(|previous| previous != &picture)
                {
                    return Err(invalid("multiple pictures in access unit"));
                }
                unit.key = Some(key);
                unit.picture = Some(picture);
            }
            6 | 9..=12 => {} // Payloads are not decoded or interpreted as colour.
            _ => return Err(unsupported("AVC NAL extension or partition")),
        }
        Ok(())
    }
    fn admit(&mut self, old: usize, new: usize) -> Result<u32> {
        let bytes = self.bytes - old + new;
        if bytes > MAX_ACTIVE_PARAMETERS {
            return Err(limit("active AVC parameters"));
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| limit("AVC parameter revisions"))?;
        self.bytes = bytes;
        self.revision = revision;
        Ok(revision)
    }
    fn parameter(&mut self, bytes: &[u8], expected: u8) -> Result<()> {
        if bytes.len() > MAX_PARAMETER_BYTES {
            return Err(limit("parameter set bytes"));
        }
        if header(bytes)?.0 != expected {
            return Err(invalid("parameter NAL type"));
        }
        if expected == 7 {
            let value = sps::parse(bytes)?;
            let slot = usize::from(value.info.id);
            let previous = &self.sps[slot];
            if previous.as_ref().is_some_and(|p| p.bytes.as_ref() == bytes) {
                return Ok(());
            }
            let revision =
                self.admit(previous.as_ref().map_or(0, |p| p.bytes.len()), bytes.len())?;
            self.sps[slot] = Some(Arc::new(Parameter {
                bytes: bytes.into(),
                revision,
                value,
            }));
        } else {
            // Only this PPS prefix determines the referenced SPS and picture
            // boundary fields. Entropy/scaling/FMO syntax remains decoder-owned.
            let mut b = Bits::new(&bytes[1..]);
            let id = b.ue(255)? as u8;
            let sps = b.ue(31)? as u8;
            b.flag()?;
            let value = Pps {
                id,
                sps,
                bottom_poc_present: b.flag()?,
            };
            let previous = &self.pps[usize::from(id)];
            if previous.as_ref().is_some_and(|p| p.bytes.as_ref() == bytes) {
                return Ok(());
            }
            let revision =
                self.admit(previous.as_ref().map_or(0, |p| p.bytes.len()), bytes.len())?;
            self.pps[usize::from(id)] = Some(Arc::new(Parameter {
                bytes: bytes.into(),
                revision,
                value,
            }));
        }
        Ok(())
    }
    fn picture(&self, bytes: &[u8], idr: bool, reference: bool) -> Result<(PictureKey, Picture)> {
        let mut b = Bits::new(&bytes[1..]);
        let first_mb = b.ue(2 << 20)?;
        b.ue(9)?; // slice_type
        let id = b.ue(255)?;
        let pps = self.pps[id as usize]
            .as_ref()
            .ok_or_else(|| invalid("undefined PPS"))?;
        let sps = self.sps[usize::from(pps.value.sps)]
            .as_ref()
            .ok_or_else(|| invalid("undefined SPS"))?;
        let seq = &sps.value;
        if first_mb >= seq.info.coded_width / 16 * (seq.info.coded_height / 16) {
            return Err(invalid("slice macroblock address"));
        }
        let plane = if seq.separate_planes { b.u(2)? } else { 0 };
        if plane == 3 {
            return Err(invalid("colour plane id"));
        }
        let frame = b.u(seq.frame_bits)?;
        let field = !seq.info.frame_mbs_only && b.flag()?;
        let bottom = field && b.flag()?;
        let idr_id = if idr { Some(b.ue(65535)?) } else { None };
        let (mut poc_lsb, mut delta_bottom, mut deltas) = (0, 0, [0; 2]);
        if seq.poc_type == 0 {
            poc_lsb = b.u(seq.poc_bits)?;
            if pps.value.bottom_poc_present && !field {
                delta_bottom = b.se()?;
            }
        }
        if seq.poc_type == 1 && !seq.delta_zero {
            deltas[0] = b.se()?;
            if pps.value.bottom_poc_present && !field {
                deltas[1] = b.se()?;
            }
        }
        Ok((
            PictureKey {
                pps: pps.value.id,
                frame,
                plane,
                field,
                bottom,
                reference,
                idr: idr_id,
                poc_lsb,
                delta_bottom,
                deltas,
            },
            Picture {
                pps_id: pps.value.id,
                sps_revision: sps.revision,
                pps_revision: pps.revision,
                idr,
                field_picture: field,
                bottom_field: bottom,
                sequence: seq.info.clone(),
            },
        ))
    }
}
pub(crate) fn length(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .fold(0, |value, byte| (value << 8) | usize::from(*byte))
}
pub(crate) fn header(bytes: &[u8]) -> Result<(u8, bool)> {
    let value = *bytes.first().ok_or_else(|| invalid("empty NAL"))?;
    let kind = value & 31;
    let reference = value & 0x60 != 0;
    if value & 0x80 != 0 || (matches!(kind, 5 | 7 | 8) && !reference) {
        return Err(invalid("NAL header"));
    }
    Ok((kind, reference))
}
fn config_nal<'a>(bytes: &'a [u8], at: &mut usize) -> Result<&'a [u8]> {
    let size = length(
        bytes
            .get(*at..*at + 2)
            .ok_or_else(|| invalid("avcC NAL size"))?,
    );
    *at += 2;
    let nal = bytes
        .get(*at..*at + size)
        .ok_or_else(|| invalid("avcC NAL extent"))?;
    *at += size;
    Ok(nal)
}
