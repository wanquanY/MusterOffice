//! Verification executable, not a host or public execution API.
use mo_common::Ticks;
use mo_video_compile::{CompileLimits, CompiledDocument, MediaBinding, QueryLimits};
use mo_video_gpu_plan::{
    ImageDescription, PreparedGpuFrame, ProgramLimits, VisualDescription, VisualReply,
};
use mo_video_model::{TimeSample, VideoDocument};
use mo_video_render::*;
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use std::sync::Arc;
#[derive(Deserialize)]
struct Input {
    document: VideoDocument,
    queries: Vec<Query>,
}
#[derive(Deserialize)]
struct Query {
    frame: Ticks,
    width: u32,
    height: u32,
    replies: Option<Vec<VisualReply>>,
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin()
        .take((8 << 20) + 1)
        .read_to_string(&mut input)?;
    if input.len() > 8 << 20 {
        return Err("input size".into());
    }
    let Input { document, queries } = serde_json::from_str(&input)?;
    if queries.len() > 128 {
        return Err("query count".into());
    }
    let d = CompiledDocument::new(document, CompileLimits::default(), &|| false)?;
    let mut results = Vec::new();
    for q in queries {
        let p = d
            .frame(q.frame.get(), QueryLimits::default(), &|| false)?
            .value;
        let options = FrameRenderOptions {
            raster: RasterExtent {
                width: q.width,
                height: q.height,
            },
            sampling: Sampling::Bilinear,
            limits: RenderLimits::default(),
        };
        let c = PreparedGpuFrame::new(p, options, ProgramLimits::default(), &|| false)?;
        let program = q
            .replies
            .as_ref()
            .map(|r| c.compile(r.clone(), &|| false))
            .transpose()?;
        let (inputs, linear) = if let Some(replies) = q.replies {
            let mut source = FixtureSources {
                candidate: &c,
                replies: &replies,
            };
            let rendered = render_frame_with_options(c.plan(), &mut source, options, &|| false)?;
            let inputs=replies.iter().filter_map(|r|match &r.visual {
                VisualDescription::Image{image}|VisualDescription::VideoFrame{image,..}=>Some(json!({"id":c.requests().iter().find(|q|q.key==r.request).unwrap().image_id,"pixels":pixels(image.width,image.height)})),
                VisualDescription::VideoGap{..}=>None,
            }).collect::<Vec<_>>();
            (
                inputs,
                rendered
                    .image
                    .pixels()
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            )
        } else {
            (vec![], vec![])
        };
        results.push(
            json!({"requests":c.requests(),"program":program,"inputs":inputs,"linear":linear}),
        );
    }
    println!("{}", json!({"documentDigest":d.digest(),"results":results}));
    Ok(())
}

fn pixels(w: u32, h: u32) -> Vec<[f32; 4]> {
    (0..w * h)
        .map(|i| {
            let a = (i % 5) as f32 / 4.;
            [
                ((i * 3) % 7) as f32 / 8. * a,
                ((i * 5) % 11) as f32 / 16. * a,
                ((i * 7) % 13) as f32 / 16. * a,
                a,
            ]
        })
        .collect()
}
fn frame(i: &ImageDescription) -> Result<VisualFrame> {
    Ok(VisualFrame {
        image: Arc::new(LinearImage::from_pixels(
            i.width,
            i.height,
            pixels(i.width, i.height),
            i.has_hdr,
            PixelLimits::default(),
            &|| false,
        )?),
        geometry: SourceGeometry::from_display_metadata(
            (
                i.display.sample_aspect_ratio[0],
                i.display.sample_aspect_ratio[1],
            ),
            i.display.matrix,
        )?,
    })
}
struct FixtureSources<'a> {
    candidate: &'a PreparedGpuFrame,
    replies: &'a [VisualReply],
}
impl VisualResources for FixtureSources<'_> {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        let request = self
            .candidate
            .requests()
            .iter()
            .find(|r| &r.binding == binding && &r.sample == sample)
            .unwrap();
        let reply = self
            .replies
            .iter()
            .find(|r| r.request == request.key)
            .unwrap();
        let visual = match &reply.visual {
            VisualDescription::Image { image } => ResolvedVisual::Image {
                frame: frame(image)?,
            },
            VisualDescription::VideoFrame { interval, image } => ResolvedVisual::VideoFrame {
                interval: interval.clone(),
                frame: frame(image)?,
            },
            VisualDescription::VideoGap { interval } => ResolvedVisual::VideoGap {
                interval: interval.clone(),
            },
        };
        Ok(ResolvedMedia {
            binding: reply.binding.clone(),
            visual,
        })
    }
}
