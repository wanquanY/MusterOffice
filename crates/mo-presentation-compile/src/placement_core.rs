//! Shared DrawingML placement arithmetic for authored and imported objects.
//! Callers resolve defaults/inheritance; this module owns scale/rotation, exact
//! anchors and outward Q96-to-Q32 bounds. It never concatenates domain rotations
//! as generic affine nodes.
use crate::{AffineUncertainty, CompileError, angle::Angle, interval::Interval};
use mo_geometry::{Affine, Fixed, Point};
use mo_presentation_model::{Point as ModelPoint, Size};
use std::collections::BTreeMap;
#[derive(Clone)]
struct Matrix {
    a: [Interval; 4],
    t: [Interval; 2],
}
impl Matrix {
    fn identity() -> Self {
        Self {
            a: [1, 0, 0, 1].map(Interval::integer),
            t: [0, 0].map(Interval::integer),
        }
    }
    fn map(&self, p: &[Interval; 2]) -> [Interval; 2] {
        std::array::from_fn(|i| {
            self.a[2 * i]
                .mul(&p[0])
                .add(&self.a[2 * i + 1].mul(&p[1]))
                .add(&self.t[i])
        })
    }
}
#[derive(Clone)]
pub(crate) struct State {
    center_map: Matrix,
    // Magnitudes remain separate from reflections, including collapsed axes.
    scale: [Interval; 2],
    flips: [bool; 2],
    angle: Angle,
}
impl State {
    pub(crate) fn identity() -> Self {
        Self {
            center_map: Matrix::identity(),
            scale: [1, 1].map(Interval::integer),
            flips: [false; 2],
            angle: Angle::integer(0),
        }
    }
}
fn linear(cs: &[Interval; 2], scale: &[Interval; 2]) -> [Interval; 4] {
    [
        cs[0].mul(&scale[0]),
        cs[1].neg().mul(&scale[1]),
        cs[1].mul(&scale[0]),
        cs[0].mul(&scale[1]),
    ]
}
struct Angles(BTreeMap<Angle, [Interval; 2]>);
impl Angles {
    fn get(&mut self, a: &Angle, check: &dyn Fn() -> bool) -> Result<[Interval; 2], CompileError> {
        if let Some(v) = self.0.get(a) {
            return Ok(v.clone());
        }
        let v = a.cos_sin(check)?;
        self.0.insert(a.clone(), v.clone());
        Ok(v)
    }
}
fn evaluated(
    a: [Interval; 4],
    t: [Interval; 2],
) -> Result<(Affine, AffineUncertainty), CompileError> {
    let mut values = [Fixed::ZERO; 4];
    let mut bounds = values;
    for i in 0..4 {
        (values[i], bounds[i]) = a[i].q32()?;
    }
    let (x, ex) = t[0].q32()?;
    let (y, ey) = t[1].q32()?;
    Ok((
        Affine {
            linear: values,
            translation: Point { x, y },
        },
        AffineUncertainty {
            linear: bounds,
            translation: Point { x: ex, y: ey },
        },
    ))
}

pub(crate) struct Frame {
    pub origin: ModelPoint,
    pub target_size: Size,
    pub source_origin: ModelPoint,
    pub source_size: Size,
    pub rotation: Angle,
    pub flips: [bool; 2],
}
pub(crate) struct PlacedFrame {
    pub anchor: Point,
    pub affine: Affine,
    pub uncertainty: AffineUncertainty,
    pub children: Option<State>,
}
pub(crate) struct Engine {
    angles: Angles,
}
impl Engine {
    pub fn new() -> Self {
        Self {
            angles: Angles(BTreeMap::new()),
        }
    }
    pub fn unique_angles(&self) -> u32 {
        self.angles.0.len() as u32
    }
    pub fn place(
        &mut self,
        frame: &Frame,
        parent: &State,
        children: bool,
        check: &dyn Fn() -> bool,
    ) -> Result<PlacedFrame, CompileError> {
        crate::cancel(check)?;
        let source = [
            frame.source_size.width.get(),
            frame.source_size.height.get(),
        ];
        let target = [
            frame.target_size.width.get(),
            frame.target_size.height.get(),
        ];
        let own_scale: [Interval; 2] = std::array::from_fn(|i| {
            if source[i] == 0 {
                Interval::integer(1)
            } else {
                Interval::ratio(target[i], source[i])
            }
        });
        let own_angle = &frame.rotation;
        let swap = own_angle.exchanges_parent_axes();
        let scale =
            std::array::from_fn(|i| parent.scale[if swap { 1 - i } else { i }].mul(&own_scale[i]));
        let flips = std::array::from_fn(|i| parent.flips[i] ^ frame.flips[i]);
        let angle = parent
            .angle
            .compose(own_angle, parent.flips[0] ^ parent.flips[1])?;
        let source_offset = [frame.source_origin.x.get(), frame.source_origin.y.get()];
        let source_center = std::array::from_fn(|i| {
            Interval::integer(source_offset[i]).add(&Interval::integer(source[i]).divide(2))
        });
        let own_center = [
            Interval::integer(frame.origin.x.get()).add(&Interval::integer(target[0]).divide(2)),
            Interval::integer(frame.origin.y.get()).add(&Interval::integer(target[1]).divide(2)),
        ];
        let center = parent.center_map.map(&own_center);
        let signed_scale = std::array::from_fn(|i| scale[i].clone().signed(flips[i]));
        let a = linear(&self.angles.get(&angle, check)?, &signed_scale);
        let (affine, uncertainty) = evaluated(a.clone(), center.clone())?;
        // Integer EMU plus half integer EMU is exact in Q32, including negative
        // child origins. Source and target centers must never be conflated.
        let anchor = Point {
            x: source_center[0].q32()?.0,
            y: source_center[1].q32()?.0,
        };
        let children = if children {
            let mut center_map = Matrix {
                a,
                t: [0, 0].map(Interval::integer),
            };
            let offset = center_map.map(&source_center);
            center_map.t = std::array::from_fn(|i| center[i].sub(&offset[i]));
            Some(State {
                center_map,
                scale,
                flips,
                angle,
            })
        } else {
            None
        };
        Ok(PlacedFrame {
            anchor,
            affine,
            uncertainty,
            children,
        })
    }
}
