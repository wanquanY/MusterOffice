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
struct Frame {
    paths: Vec<Vec<PathCommand>>,
    draws: Vec<Draw>,
    work: FrameWork,
    clip: Option<clip::LocalClip>,
}
pub(crate) struct RetainedText {
    objects: BTreeMap<(String, u32), std::ops::Range<usize>>,
    frames: Vec<Frame>,
    pub work: FrameWork,
    /// Logical element storage; excludes allocator, index and transient scene memory.
    pub path_bytes: u64,
    pub capacity: crate::source_frame::capacity::TextCapacity,
}
impl RetainedText {
    pub fn new(
        content: TextPageContent,
        bindings: &[SourcePagePaintBinding],
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let capacity = super::capacity(&content.texts, check)?;
        let mut objects: BTreeMap<(String, u32), std::ops::Range<usize>> = BTreeMap::new();
        let mut frames = Vec::with_capacity(content.texts.len());
        let mut cells = BTreeSet::new();
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
            let at = &bindings
                .get(text.binding as usize)
                .ok_or(SourcePageError::Invalid("retained text binding"))?
                .location;
            let key = (
                at.part.clone(),
                at.object
                    .ok_or(SourcePageError::Invalid("retained text object"))?,
            );
            if text.frame.text.object.part != at.part
                || Some(text.frame.text.object.native_id) != at.object
            {
                return Err(SourcePageError::Invalid(
                    "retained text object differs from source",
                ));
            }
            let cell = text.frame.text.cell;
            let region_cell = match text.frame.region.source {
                TextRectangleSource::TableCell { cell, .. } => Some(cell),
                _ => None,
            };
            if cell != region_cell || !cells.insert((key.clone(), cell)) {
                return Err(SourcePageError::Invalid(
                    "duplicate or mismatched retained text frame",
                ));
            }
            if let Some(range) = objects.get_mut(&key) {
                if range.end != i || cell.is_none() || cells.contains(&(key.clone(), None)) {
                    return Err(SourcePageError::Invalid(
                        "noncontiguous or mixed retained text frames",
                    ));
                }
                range.end += 1;
            } else {
                objects.insert(key, i..i + 1);
            }
            let mut object = Frame {
                paths: vec![],
                draws: vec![],
                work: text.frame.work.clone(),
                clip: clip::LocalClip::for_frame(
                    &text.frame,
                    &text.decorations,
                    precision::frame_bound(&text.frame)?,
                )?,
            };
            if object.clip.is_some() {
                admit(std::mem::size_of::<clip::LocalClip>())?;
            }
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
            frames.push(object);
        }
        cancel(check)?;
        Ok(Self {
            objects,
            frames,
            work: content.text_work,
            path_bytes: bytes as u64,
            capacity,
        })
    }
    pub fn frames(&self) -> u32 {
        self.frames.len() as u32
    }
    pub fn painter(&self, visible: &BTreeSet<(String, Option<u32>)>) -> RetainedPainter<'_> {
        RetainedPainter {
            text: self,
            seen: BTreeSet::new(),
            failed: false,
            expected: self
                .objects
                .keys()
                .filter(|(part, id)| visible.contains(&(part.clone(), Some(*id))))
                .cloned()
                .collect(),
        }
    }
}
pub(crate) struct RetainedPainter<'a> {
    text: &'a RetainedText,
    seen: BTreeSet<(String, u32)>,
    expected: BTreeSet<(String, u32)>,
    // Failure invalidates this painter, not the immutable retained plan.
    failed: bool,
}
impl RetainedPainter<'_> {
    pub fn finish(
        self,
    ) -> Result<(FrameWork, crate::source_frame::capacity::TextCapacity), SourcePageError> {
        if self.failed || self.seen != self.expected {
            return Err(SourcePageError::Invalid("unpainted retained text"));
        }
        let mut work = FrameWork::default();
        for key in &self.seen {
            for frame in &self.text.frames[self.text.objects[key].clone()] {
                work.glyphs = work
                    .glyphs
                    .checked_add(frame.work.glyphs)
                    .ok_or(RasterError::Limit("retained text glyphs"))?;
                work.path_commands = work
                    .path_commands
                    .checked_add(frame.work.path_commands)
                    .ok_or(RasterError::Limit("retained text paths"))?;
            }
        }
        let mut capacity = self.text.capacity.clone();
        capacity.frames.retain(|f| {
            self.seen
                .contains(&(f.object.part.clone(), f.object.native_id))
        });
        Ok((work, capacity))
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
        if self.failed {
            return Err(SourcePageError::Invalid("failed retained text paint"));
        }
        self.failed = true;
        let result = self.append_object(binding, object, builder, viewport, check);
        self.failed = result.is_err();
        result
    }
}
impl RetainedPainter<'_> {
    fn append_object(
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
        let Some(range) = self.text.objects.get(&key) else {
            return Ok((Fixed::ZERO, Fixed::ZERO));
        };
        if !self.seen.insert(key) {
            return Err(SourcePageError::Invalid("duplicate retained text paint"));
        }
        let mut position = Fixed::ZERO;
        let mut geometry = Fixed::ZERO;
        for local in &self.text.frames[range.clone()] {
            cancel(check)?;
            let (clip, p, g) = clip::append(local.clip, object, builder, viewport, check)?;
            position = position.max(p);
            geometry = geometry.max(g);
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
                        clip,
                    },
                    check,
                )?;
                position = position.max(p);
                geometry = geometry.max(g);
            }
        }
        Ok((position, geometry))
    }
}
