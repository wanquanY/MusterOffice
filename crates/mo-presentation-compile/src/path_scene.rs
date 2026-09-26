//! Bounded path/affine interning shared by author and native page compilation.
use mo_geometry::{Affine, Fixed, PathCommand as C, Point};
use mo_raster::{FillPath, FillRule, RasterError};
use mo_render::{ClipNode, DrawScene, PathInstance, TransformNode};
use std::collections::BTreeMap;
pub(crate) struct ScenePaint {
    pub brush: mo_raster::Brush,
    pub stroke: Option<mo_raster::StrokeStyle>,
    pub clip: Option<u32>,
    pub blend: mo_raster::BlendMode,
}
pub(crate) struct SceneBuilder<S> {
    pub scene: DrawScene,
    interned: [BTreeMap<Vec<C>, u32>; 2],
    transforms: BTreeMap<([Fixed; 4], Point), u32>,
    source_commands: u32,
    paint_budget: mo_raster::PaintBudget,
    pub sources: Vec<S>,
    pub generated_commands: u32,
}
impl<S> SceneBuilder<S> {
    pub fn new() -> Self {
        Self {
            scene: DrawScene {
                clips: vec![],
                paths: vec![],
                transforms: vec![],
                instances: vec![],
            },
            interned: [BTreeMap::new(), BTreeMap::new()],
            transforms: BTreeMap::new(),
            source_commands: 0,
            paint_budget: Default::default(),
            sources: vec![],
            generated_commands: 0,
        }
    }
    pub fn add(
        &mut self,
        commands: &[C],
        affine: Affine,
        brush: mo_raster::Brush,
        stroke: Option<mo_raster::StrokeStyle>,
        source: impl FnOnce(u32) -> S,
    ) -> Result<(), RasterError> {
        self.add_clipped(commands, affine, brush, stroke, None, source)
    }
    pub fn add_clipped(
        &mut self,
        commands: &[C],
        affine: Affine,
        brush: mo_raster::Brush,
        stroke: Option<mo_raster::StrokeStyle>,
        clip: Option<u32>,
        source: impl FnOnce(u32) -> S,
    ) -> Result<(), RasterError> {
        self.paint(
            commands,
            affine,
            ScenePaint {
                brush,
                stroke,
                clip,
                blend: Default::default(),
            },
            source,
        )
    }
    pub fn paint(
        &mut self,
        commands: &[C],
        affine: Affine,
        paint: ScenePaint,
        source: impl FnOnce(u32) -> S,
    ) -> Result<(), RasterError> {
        let ScenePaint {
            brush,
            stroke,
            clip,
            blend,
        } = paint;
        if clip.is_some_and(|n| n as usize >= self.scene.clips.len()) {
            return Err(RasterError::Invalid("page clip reference"));
        }
        let mut budget = self.paint_budget;
        budget.include_brush(&brush)?;
        let path = self.path(commands, FillRule::Nonzero)?;
        let transform = self.transform(affine)?;
        let instance = self.scene.instances.len() as u32;
        self.scene.instances.push(PathInstance {
            blend,
            clip,
            path,
            transform,
            brush,
            stroke,
        });
        self.paint_budget = budget;
        self.sources.push(source(instance));
        Ok(())
    }
    pub fn clip(&mut self, path: &FillPath, affine: Affine) -> Result<u32, RasterError> {
        if self.scene.clips.len() >= 8192 {
            return Err(RasterError::Limit("page clips"));
        }
        let path = self.path(&path.commands, path.fill_rule)?;
        let transform = self.transform(affine)?;
        // Clip resources are independent world-space masks. Their paths and
        // transforms use exactly the same interning and limits as draw paths.
        let index = self.scene.clips.len() as u32;
        self.scene.clips.push(ClipNode {
            parent: None,
            path,
            transform,
        });
        Ok(index)
    }
    fn path(&mut self, commands: &[C], fill_rule: FillRule) -> Result<u32, RasterError> {
        self.generated_commands = self
            .generated_commands
            .checked_add(
                u32::try_from(commands.len()).map_err(|_| RasterError::Limit("page commands"))?,
            )
            .filter(|v| *v <= 1048576)
            .ok_or(RasterError::Limit("page generated commands"))?;
        let rule = usize::from(matches!(fill_rule, FillRule::Evenodd));
        let path = if let Some(index) = self.interned[rule].get(commands) {
            *index
        } else {
            if self.scene.paths.len() >= 4096 {
                return Err(RasterError::Limit("page unique paths"));
            }
            self.source_commands = self
                .source_commands
                .checked_add(
                    u32::try_from(commands.len())
                        .map_err(|_| RasterError::Limit("page commands"))?,
                )
                .filter(|n| *n <= 262144)
                .ok_or(RasterError::Limit("page unique commands"))?;
            let index = self.scene.paths.len() as u32;
            self.interned[rule].insert(commands.to_vec(), index);
            self.scene.paths.push(FillPath {
                fill_rule,
                commands: commands.to_vec(),
            });
            index
        };
        Ok(path)
    }
    fn transform(&mut self, affine: Affine) -> Result<Option<u32>, RasterError> {
        let transform = if affine == Affine::IDENTITY {
            None
        } else {
            let key = (affine.linear, affine.translation);
            Some(if let Some(index) = self.transforms.get(&key) {
                *index
            } else {
                if self.scene.transforms.len() >= 8192 {
                    return Err(RasterError::Limit("page transforms"));
                }
                let index = self.scene.transforms.len() as u32;
                self.transforms.insert(key, index);
                self.scene.transforms.push(TransformNode {
                    parent: None,
                    affine,
                });
                index
            })
        };
        Ok(transform)
    }
}
