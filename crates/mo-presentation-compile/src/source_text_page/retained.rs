//! Compact, immutable shape-local paint. Font bytes and full shaping provenance
//! are not retained. Only the shared placement routine applies a sampled affine.
use super::*;
use mo_geometry::PathCommand;
use std::collections::BTreeSet;

struct Draw {
    path: usize,
    origin: Point,
    rgba: [u8; 4],
    uncertainty: Fixed,
}
struct Object {
    paths: Vec<Vec<PathCommand>>,
    draws: Vec<Draw>,
}
pub(crate) struct RetainedText {
    objects: BTreeMap<(String, u32), Object>,
    pub work: FrameWork,
    /// Logical element storage; excludes allocator, index and transient scene memory.
    pub path_bytes: u64,
}
impl RetainedText {
    pub fn new(
        content: TextPageContent,
        bindings: &[SourcePagePaintBinding],
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let mut objects = BTreeMap::new();
        let mut bytes = 0usize;
        let mut admit = |n: usize| -> Result<(), SourcePageError> {
            bytes = bytes
                .checked_add(n)
                .filter(|n| *n <= 64 * 1024 * 1024)
                .ok_or(RasterError::Limit("retained text paint bytes"))?;
            Ok(())
        };
        let mut painted = BTreeMap::<u32, Vec<_>>::new();
        for source in content.text_sources {
            cancel(check)?;
            painted.entry(source.text_binding).or_default().push(source);
        }
        for (i, text) in content.texts.into_iter().enumerate() {
            cancel(check)?;
            let at = &bindings[text.binding as usize].location;
            let mut object = Object {
                paths: vec![],
                draws: vec![],
            };
            let mut paths = BTreeMap::new();
            let uncertainty = precision::frame_bound(&text.frame)?;
            for source in painted.remove(&(i as u32)).unwrap_or_default() {
                cancel(check)?;
                let glyph = &text.frame.glyphs[source.glyph as usize];
                let scene = text.frame.paragraphs[glyph.paragraph as usize]
                    .computed
                    .geometry
                    .paths
                    .scene
                    .as_ref()
                    .expect("complete text");
                let path = scene.glyphs[glyph.glyph as usize].path;
                let id = if let Some(id) = paths.get(&(glyph.paragraph, path)) {
                    *id
                } else {
                    let commands = &scene.paths[path as usize].commands;
                    admit(commands.len() * std::mem::size_of::<PathCommand>())?;
                    let id = object.paths.len();
                    object.paths.push(commands.clone());
                    paths.insert((glyph.paragraph, path), id);
                    id
                };
                admit(std::mem::size_of::<Draw>())?;
                object.draws.push(Draw {
                    path: id,
                    origin: glyph.origin,
                    rgba: text.clusters[source.cluster as usize]
                        .rgba
                        .expect("painted glyph"),
                    uncertainty,
                });
            }
            for decoration in text.decorations {
                cancel(check)?;
                admit(5 * std::mem::size_of::<PathCommand>() + std::mem::size_of::<Draw>())?;
                object.draws.push(Draw {
                    path: object.paths.len(),
                    origin: Point {
                        x: Fixed::ZERO,
                        y: Fixed::ZERO,
                    },
                    rgba: decoration.rgba,
                    uncertainty: uncertainty.checked_add(Fixed::from_raw(8))?,
                });
                object
                    .paths
                    .push(decorations::commands(decoration.rect).to_vec());
            }
            if objects
                .insert((at.part.clone(), at.object.expect("text object")), object)
                .is_some()
            {
                return Err(SourcePageError::Invalid("duplicate retained text object"));
            }
        }
        cancel(check)?;
        Ok(Self {
            objects,
            work: content.text_work,
            path_bytes: bytes as u64,
        })
    }
    pub fn frames(&self) -> u32 {
        self.objects.len() as u32
    }
    pub fn painter(&self) -> RetainedPainter<'_> {
        RetainedPainter {
            text: self,
            seen: BTreeSet::new(),
        }
    }
    pub fn sample_work(&self) -> FrameWork {
        FrameWork {
            component_calls: 0,
            font_upload_bytes: 0,
            request_words: 0,
            glyphs: self.work.glyphs,
            path_commands: self.work.path_commands,
        }
    }
}
pub(crate) struct RetainedPainter<'a> {
    text: &'a RetainedText,
    seen: BTreeSet<(String, u32)>,
}
impl RetainedPainter<'_> {
    pub fn finish(self) -> Result<(), SourcePageError> {
        if self.seen.len() != self.text.objects.len() {
            return Err(SourcePageError::Invalid("unpainted retained text"));
        }
        Ok(())
    }
}
impl Painter for RetainedPainter<'_> {
    fn append(
        &mut self,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError> {
        let key = (
            object.location.part.clone(),
            object.location.object.expect("object"),
        );
        let Some(local) = self.text.objects.get(&key) else {
            return Ok((Fixed::ZERO, Fixed::ZERO));
        };
        if !self.seen.insert(key) {
            return Err(SourcePageError::Invalid("duplicate retained text paint"));
        }
        let mut position = Fixed::ZERO;
        let mut geometry = Fixed::ZERO;
        for draw in &local.draws {
            cancel(check)?;
            let (_, p, g) = placement::paint(
                binding,
                object,
                builder,
                viewport,
                placement::LocalPath {
                    commands: &local.paths[draw.path],
                    origin: draw.origin,
                    rgba: draw.rgba,
                    uncertainty: draw.uncertainty,
                },
                check,
            )?;
            position = position.max(p);
            geometry = geometry.max(g);
        }
        Ok((position, geometry))
    }
}
