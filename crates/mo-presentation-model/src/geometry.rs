use mo_common::{Emu, ObjectId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Point {
    pub x: Emu,
    pub y: Emu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Size {
    pub width: Emu,
    pub height: Emu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transform {
    pub origin: Point,
    pub size: Size,
    /// Units of 1/60000 degree. Author direction is preserved.
    pub rotation: i32,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}

/// DrawingML uses 60000 integer units per degree for static object transforms.
pub const ROTATION_UNITS_PER_TURN: i32 = 21_600_000;

impl Transform {
    /// Equivalent static orientation in one nonnegative turn. The author value
    /// remains unchanged: signed/multi-turn values may carry editing intent and
    /// must not be normalized in storage or interpreted as animation travel.
    pub fn normalized_rotation(&self) -> i32 {
        self.rotation.rem_euclid(ROTATION_UNITS_PER_TURN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Geometry {
    Rectangle,
    Ellipse,
    RoundRectangle {
        radius: Emu,
    },
    Path {
        commands: Vec<PathCommand>,
        viewport: Size,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ConnectorEndpoint {
    Free { position: Point },
    Attached { object: ObjectId, site: u32 },
}
