use crate::{Fixed, GeometryError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Point {
    pub x: Fixed,
    pub y: Fixed,
}
impl Point {
    pub fn translate(self, by: Self) -> Result<Self, GeometryError> {
        Ok(Self {
            x: self.x.checked_add(by.x)?,
            y: self.y.checked_add(by.y)?,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}
impl Rect {
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
            },
            max: Point {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
            },
        }
    }
    pub fn translate(self, by: Point) -> Result<Self, GeometryError> {
        Ok(Self {
            min: self.min.translate(by)?,
            max: self.max.translate(by)?,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PathCommand {
    Move {
        to: Point,
    },
    Line {
        to: Point,
    },
    Quadratic {
        control: Point,
        to: Point,
    },
    Cubic {
        control1: Point,
        control2: Point,
        to: Point,
    },
    Close,
}
