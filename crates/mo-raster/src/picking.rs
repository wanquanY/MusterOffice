//! Point queries over the exact, certified device paths sent to the renderer.
//! Geometry masks are independent of shader color/alpha and compositing.
use crate::{PixelScale, RasterBackend, RasterError, cancel, number::Scale};
use mo_geometry::{Fixed, Point};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Range;
#[cfg(test)]
mod tests;
pub const PICK_MAGIC: u32 = 0x4d4f504b;
pub const MAX_PICK_FRAME_WORDS: usize =
    10 + 4096 * 2 + 262144 * 7 + 4096 * 4 + 8192 * 4 + 65536 * 5 + 64 * 3;
pub const MAX_PICK_REPLY_WORDS: usize = 7 + 64 * (2048 * 2 + 257);
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DevicePickQuery {
    /// Q32 device pixels, in the exact prepared viewport.
    pub point: Point,
    /// Q32 device pixels, 0..=32. Expands geometry, never the clipping region.
    pub radius: Fixed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DrawHitKind {
    Exact,
    Nearby,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawHit {
    pub draw: u32,
    pub kind: DrawHitKind,
}
pub struct DrawHits {
    point: Point,
    exact: Vec<u32>,
    nearby: Vec<u32>,
    clips: Vec<u32>,
    clip_count: u32,
    draws: u32,
}
impl DrawHits {
    /// Exact float32 sample expressed back in Q32 device pixels. Semantic
    /// regions must query this same point, not an independently rounded input.
    pub fn sampled_point(&self) -> Point {
        self.point
    }
    /// Membership in the exact renderer clip intersection. None is the viewport.
    /// Clip indices use the lowered raster batch's zero-based IDs.
    pub fn contains_clip(&self, clip: Option<u32>) -> bool {
        let index = match clip {
            Some(c) if c < self.clip_count => c + 1,
            Some(_) => return false,
            None => 0,
        };
        self.clips[index as usize / 32] & (1 << (index % 32)) != 0
    }
    /// Reverse original paint order, with no alpha/occlusion filtering.
    pub fn hits(&self) -> impl Iterator<Item = DrawHit> + '_ {
        (0..self.draws).rev().filter_map(|draw| {
            let bit = 1 << (draw % 32);
            let word = (draw / 32) as usize;
            if self.exact[word] & bit != 0 {
                Some(DrawHit {
                    draw,
                    kind: DrawHitKind::Exact,
                })
            } else if self.nearby[word] & bit != 0 {
                Some(DrawHit {
                    draw,
                    kind: DrawHitKind::Nearby,
                })
            } else {
                None
            }
        })
    }
}
/// Private section boundaries are captured by the raster compiler, not parsed
/// from untrusted client frames. Ordinary rasterization retains only these offsets.
pub(crate) struct GeometrySections {
    pub paths: Range<usize>,
    pub strokes: Range<usize>,
    pub clips: Range<usize>,
    pub draws: Range<usize>,
    pub draw_words: usize,
}
pub struct CompiledPicking {
    frame: Vec<u32>,
}
pub struct PickingReply {
    pub status: u32,
    pub words: Vec<u32>,
}
impl GeometrySections {
    pub(crate) fn extract(
        &self,
        raster: &[u32],
        check: &dyn Fn() -> bool,
    ) -> Result<CompiledPicking, RasterError> {
        cancel(check)?;
        let mut frame = vec![
            PICK_MAGIC,
            1,
            raster[2],
            raster[3],
            raster[5],
            raster[7],
            raster[8],
            (self.clips.len() / 4) as u32,
            raster[6],
            0,
        ];
        frame
            .try_reserve(
                self.paths.len() + self.strokes.len() + self.clips.len() + raster[6] as usize * 5,
            )
            .map_err(|_| RasterError::Host("picking geometry allocation"))?;
        for range in [&self.paths, &self.strokes, &self.clips] {
            for chunk in raster[range.clone()].chunks(4096) {
                cancel(check)?;
                frame.extend_from_slice(chunk);
            }
        }
        for draw in raster[self.draws.clone()].chunks_exact(self.draw_words) {
            cancel(check)?;
            frame.extend_from_slice(&[
                draw[0],
                draw[1],
                draw[2],
                draw[4],
                if self.draw_words >= 7 { draw[6] } else { 0 },
            ]);
        }
        Ok(CompiledPicking { frame })
    }
}
impl CompiledPicking {
    pub fn draw_count(&self) -> u32 {
        self.frame[8]
    }
    pub fn query(
        &self,
        queries: &[DevicePickQuery],
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<DrawHits>, RasterError> {
        cancel(check)?;
        if queries.len() > 64 {
            return Err(RasterError::Limit("picking queries"));
        }
        if queries.is_empty() {
            return Ok(vec![]);
        }
        let scale = Scale::new(PixelScale {
            numerator: 1,
            denominator: 1,
        })?;
        let mut points = Vec::with_capacity(queries.len() * 3);
        for q in queries {
            cancel(check)?;
            if q.radius.raw() < 0 || q.radius.raw() > 32i128 << 32 {
                return Err(RasterError::Invalid("picking radius"));
            }
            for v in [q.point.x, q.point.y, q.radius] {
                let n = scale.value(v.raw())?;
                if n.abs() > 32768.0 {
                    return Err(RasterError::Range);
                }
                // Same maximum coordinate policy as the rendered viewport.
                if scale.error(v.raw(), n)? > 1 << 24 {
                    return Err(RasterError::Precision);
                }
                points.push(n.to_bits());
            }
        }
        let mut frame = Vec::new();
        frame
            .try_reserve_exact(self.frame.len() + points.len())
            .map_err(|_| RasterError::Host("picking query allocation"))?;
        for chunk in self.frame.chunks(4096) {
            cancel(check)?;
            frame.extend_from_slice(chunk);
        }
        frame[9] = queries.len() as u32;
        frame.extend(points);
        cancel(check)?;
        let result = (|| {
            let reply = backend.pick(&frame)?;
            if reply.status > 4 || (reply.status != 0 && !reply.words.is_empty()) {
                return Err(RasterError::ComponentInvalid("picking failure ownership"));
            }
            if reply.status != 0 {
                return Err(RasterError::Component(reply.status));
            }
            cancel(check)?;
            let stride = (self.draw_count() as usize).div_ceil(32);
            let clip_count = self.frame[7];
            let clip_stride = (clip_count as usize + 1).div_ceil(32);
            let query_stride = stride * 2 + clip_stride;
            if reply.words.len() != 7 + queries.len() * query_stride
                || reply.words[..7]
                    != [
                        PICK_MAGIC,
                        1,
                        queries.len() as u32,
                        self.draw_count(),
                        stride as u32,
                        clip_count,
                        clip_stride as u32,
                    ]
            {
                return Err(RasterError::ComponentInvalid("picking reply shape"));
            }
            let mut result = Vec::with_capacity(queries.len());
            for (i, query) in queries.iter().enumerate() {
                cancel(check)?;
                let start = 7 + i * query_stride;
                let exact = &reply.words[start..start + stride];
                let nearby = &reply.words[start + stride..start + 2 * stride];
                let clips = &reply.words[start + 2 * stride..start + query_stride];
                if invalid_last_bits(clips, clip_count + 1)
                    || (clips[0] & 1 == 0
                        && (clips.iter().chain(exact).chain(nearby).any(|&v| v != 0)))
                {
                    return Err(RasterError::ComponentInvalid("picking clip bits"));
                }
                for (j, (&a, &b)) in exact.iter().zip(nearby).enumerate() {
                    cancel(check)?;
                    let remaining = self.draw_count() as usize - j * 32;
                    let valid = if remaining < 32 {
                        (1u32 << remaining) - 1
                    } else {
                        u32::MAX
                    };
                    if a & b != 0
                        || (a | b) & !valid != 0
                        || (query.radius == Fixed::ZERO && b != 0)
                    {
                        return Err(RasterError::ComponentInvalid("picking reply bits"));
                    }
                }
                result.push(DrawHits {
                    point: Point {
                        x: sampled_fixed(frame[self.frame.len() + i * 3]),
                        y: sampled_fixed(frame[self.frame.len() + i * 3 + 1]),
                    },
                    exact: copy_bits(exact)?,
                    nearby: copy_bits(nearby)?,
                    clips: copy_bits(clips)?,
                    clip_count,
                    draws: self.draw_count(),
                });
            }
            cancel(check)?;
            Ok(result)
        })();
        if result.as_ref().is_err_and(RasterError::invalidates_backend) {
            backend.invalidate();
        }
        result
    }
}
fn sampled_fixed(word: u32) -> Fixed {
    // These are our validated finite input floats, bounded by 32768. Every
    // float sampled from an integral Q32 value is still an integral Q32 value;
    // this power-of-two expansion is exact in f64 and fits i128.
    Fixed::from_raw((f64::from(f32::from_bits(word)) * 4294967296.0) as i128)
}
fn invalid_last_bits(words: &[u32], count: u32) -> bool {
    let remaining = count % 32;
    remaining != 0 && words.last().is_some_and(|&v| v >> remaining != 0)
}
fn copy_bits(bits: &[u32]) -> Result<Vec<u32>, RasterError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(bits.len())
        .map_err(|_| RasterError::Host("picking result allocation"))?;
    result.extend_from_slice(bits);
    Ok(result)
}
