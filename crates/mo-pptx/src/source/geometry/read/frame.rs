use super::*;
use crate::source::{boolean, drawingml::enumeration, integer};
use mo_common::Emu;

pub(super) fn required<'a>(e: &'a Element, name: &str) -> Result<&'a str, XmlError> {
    e.attribute(name)
        .ok_or_else(|| malformed(format!("missing geometry attribute {name}")))
}
fn point(e: &Element, ordinal: u32) -> Result<SourceGeometryPoint, XmlError> {
    Ok(SourceGeometryPoint {
        source_ordinal: ordinal,
        x: required(e, "x")?.into(),
        y: required(e, "y")?.into(),
    })
}
fn size(e: &Element, name: &str) -> Result<Option<Emu>, XmlError> {
    e.attribute(name)
        .map(|s| {
            let n: i64 = integer(Some(s), name)?;
            if !(0..=27273042316900).contains(&n) {
                return Err(malformed("native geometry path extent outside range"));
            }
            Ok(Emu::new(n))
        })
        .transpose()
}
struct Handle {
    ordinal: u32,
    polar: bool,
    refs: [Option<String>; 6],
    position: Option<SourceGeometryPoint>,
}
struct Connection {
    ordinal: u32,
    angle: String,
    position: Option<SourceGeometryPoint>,
}
struct Command {
    ordinal: u32,
    kind: CommandKind,
    points: Vec<SourceGeometryPoint>,
}
#[derive(Clone, Copy)]
enum CommandKind {
    Move,
    Line,
    Quadratic,
    Cubic,
}
impl CommandKind {
    fn points(self) -> usize {
        match self {
            Self::Move | Self::Line => 1,
            Self::Quadratic => 2,
            Self::Cubic => 3,
        }
    }
}
pub(super) struct Frame {
    node: Node,
}
enum Node {
    Guides(SourceGeometryList<SourceGuide>, bool),
    Guide(SourceGuide),
    Handles(SourceGeometryList<SourceAdjustHandle>),
    Handle(Box<Handle>),
    Connections(SourceGeometryList<SourceConnectionSite>),
    Connection(Connection),
    Rect(SourceGeometryRect),
    Point(SourceGeometryPoint),
    Paths(SourceGeometryList<SourceGeometryPath>),
    Path(SourceGeometryPath),
    Command(Command),
    Leaf(SourceGeometryCommand),
}
pub(super) enum Complete {
    Guides(SourceGeometryList<SourceGuide>, bool),
    Guide(SourceGuide),
    Handles(SourceGeometryList<SourceAdjustHandle>),
    Handle(Box<SourceAdjustHandle>),
    Connections(SourceGeometryList<SourceConnectionSite>),
    Connection(SourceConnectionSite),
    Rect(SourceGeometryRect),
    Point(SourceGeometryPoint),
    Paths(SourceGeometryList<SourceGeometryPath>),
    Path(SourceGeometryPath),
    Command(SourceGeometryCommand),
}
fn list<T>(ordinal: u32) -> SourceGeometryList<T> {
    SourceGeometryList {
        source_ordinal: ordinal,
        entries: Vec::new(),
    }
}
impl Frame {
    pub fn new(e: &Element, ordinal: u32) -> Result<(Self, &'static [&'static str]), XmlError> {
        let local = e.name.local.as_str();
        let (node, attrs): (Node, &[&str]) = match local {
            "avLst" | "gdLst" => (Node::Guides(list(ordinal), local == "avLst"), &[]),
            "gd" => (
                Node::Guide(SourceGuide {
                    source_ordinal: ordinal,
                    name: required(e, "name")?.into(),
                    formula: required(e, "fmla")?.into(),
                }),
                &["name", "fmla"],
            ),
            "ahLst" => (Node::Handles(list(ordinal)), &[]),
            "ahXY" | "ahPolar" => {
                let polar = local == "ahPolar";
                let attrs = if polar {
                    ["gdRefR", "minR", "maxR", "gdRefAng", "minAng", "maxAng"]
                } else {
                    ["gdRefX", "minX", "maxX", "gdRefY", "minY", "maxY"]
                };
                (
                    Node::Handle(Box::new(Handle {
                        ordinal,
                        polar,
                        refs: attrs.map(|name| e.attribute(name).map(str::to_owned)),
                        position: None,
                    })),
                    if polar {
                        &["gdRefR", "minR", "maxR", "gdRefAng", "minAng", "maxAng"]
                    } else {
                        &["gdRefX", "minX", "maxX", "gdRefY", "minY", "maxY"]
                    },
                )
            }
            "cxnLst" => (Node::Connections(list(ordinal)), &[]),
            "cxn" => (
                Node::Connection(Connection {
                    ordinal,
                    angle: required(e, "ang")?.into(),
                    position: None,
                }),
                &["ang"],
            ),
            "pt" | "pos" => (Node::Point(point(e, ordinal)?), &["x", "y"]),
            "rect" => (
                Node::Rect(SourceGeometryRect {
                    source_ordinal: ordinal,
                    left: required(e, "l")?.into(),
                    top: required(e, "t")?.into(),
                    right: required(e, "r")?.into(),
                    bottom: required(e, "b")?.into(),
                }),
                &["l", "t", "r", "b"],
            ),
            "pathLst" => (Node::Paths(list(ordinal)), &[]),
            "path" => (
                Node::Path(SourceGeometryPath {
                    source_ordinal: ordinal,
                    width: size(e, "w")?,
                    height: size(e, "h")?,
                    fill: e.attribute("fill").map(enumeration).transpose()?,
                    stroke: e.attribute("stroke").map(boolean).transpose()?,
                    extrusion_ok: e.attribute("extrusionOk").map(boolean).transpose()?,
                    commands: Vec::new(),
                }),
                &["w", "h", "fill", "stroke", "extrusionOk"],
            ),
            "moveTo" | "lnTo" | "quadBezTo" | "cubicBezTo" => {
                let kind = match local {
                    "moveTo" => CommandKind::Move,
                    "lnTo" => CommandKind::Line,
                    "quadBezTo" => CommandKind::Quadratic,
                    _ => CommandKind::Cubic,
                };
                (
                    Node::Command(Command {
                        ordinal,
                        kind,
                        points: Vec::with_capacity(kind.points()),
                    }),
                    &[],
                )
            }
            "arcTo" => (
                Node::Leaf(SourceGeometryCommand::Arc {
                    source_ordinal: ordinal,
                    width_radius: required(e, "wR")?.into(),
                    height_radius: required(e, "hR")?.into(),
                    start_angle: required(e, "stAng")?.into(),
                    sweep_angle: required(e, "swAng")?.into(),
                }),
                &["wR", "hR", "stAng", "swAng"],
            ),
            "close" => (
                Node::Leaf(SourceGeometryCommand::Close {
                    source_ordinal: ordinal,
                }),
                &[],
            ),
            _ => return Err(malformed("unknown geometry frame")),
        };
        Ok((Self { node }, attrs))
    }
    pub fn accepts(&self, local: &str) -> bool {
        match &self.node {
            Node::Guides(..) => local == "gd",
            Node::Handles(_) => matches!(local, "ahXY" | "ahPolar"),
            Node::Connections(_) => local == "cxn",
            Node::Handle(v) => local == "pos" && v.position.is_none(),
            Node::Connection(v) => local == "pos" && v.position.is_none(),
            Node::Paths(_) => local == "path",
            Node::Path(_) => matches!(
                local,
                "moveTo" | "lnTo" | "arcTo" | "quadBezTo" | "cubicBezTo" | "close"
            ),
            Node::Command(v) => local == "pt" && v.points.len() < v.kind.points(),
            _ => false,
        }
    }
    pub fn append(&mut self, record: Complete) -> Result<(), XmlError> {
        match (&mut self.node, record) {
            (Node::Guides(v, _), Complete::Guide(c)) => v.entries.push(c),
            (Node::Handles(v), Complete::Handle(c)) => v.entries.push(*c),
            (Node::Connections(v), Complete::Connection(c)) => v.entries.push(c),
            (Node::Paths(v), Complete::Path(c)) => v.entries.push(c),
            (Node::Path(v), Complete::Command(c)) => v.commands.push(c),
            (Node::Command(v), Complete::Point(c)) => v.points.push(c),
            (Node::Handle(v), Complete::Point(c)) => v.position = Some(c),
            (Node::Connection(v), Complete::Point(c)) => v.position = Some(c),
            _ => return Err(malformed("incompatible completed geometry record")),
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Complete, XmlError> {
        Ok(match self.node {
            Node::Guides(v, role) => Complete::Guides(v, role),
            Node::Guide(v) => Complete::Guide(v),
            Node::Handles(v) => Complete::Handles(v),
            Node::Connections(v) => Complete::Connections(v),
            Node::Rect(v) => Complete::Rect(v),
            Node::Point(v) => Complete::Point(v),
            Node::Paths(v) => Complete::Paths(v),
            Node::Path(v) => Complete::Path(v),
            Node::Leaf(v) => Complete::Command(v),
            Node::Connection(v) => Complete::Connection(SourceConnectionSite {
                source_ordinal: v.ordinal,
                angle: v.angle,
                position: v
                    .position
                    .ok_or_else(|| malformed("connection site lacks position"))?,
            }),
            Node::Handle(v) => {
                let [a, b, c, d, e, f] = v.refs;
                let position = v
                    .position
                    .ok_or_else(|| malformed("adjust handle lacks position"))?;
                Complete::Handle(Box::new(if v.polar {
                    SourceAdjustHandle::Polar {
                        source_ordinal: v.ordinal,
                        position,
                        guide_radius: a,
                        min_radius: b,
                        max_radius: c,
                        guide_angle: d,
                        min_angle: e,
                        max_angle: f,
                    }
                } else {
                    SourceAdjustHandle::Xy {
                        source_ordinal: v.ordinal,
                        position,
                        guide_x: a,
                        min_x: b,
                        max_x: c,
                        guide_y: d,
                        min_y: e,
                        max_y: f,
                    }
                }))
            }
            Node::Command(v) => {
                if v.points.len() != v.kind.points() {
                    return Err(malformed("geometry command point count"));
                }
                let source_ordinal = v.ordinal;
                let mut pts = v.points.into_iter();
                let a = pts.next().expect("checked point count");
                Complete::Command(match v.kind {
                    CommandKind::Move => SourceGeometryCommand::Move {
                        source_ordinal,
                        to: a,
                    },
                    CommandKind::Line => SourceGeometryCommand::Line {
                        source_ordinal,
                        to: a,
                    },
                    CommandKind::Quadratic => SourceGeometryCommand::Quadratic {
                        source_ordinal,
                        control: a,
                        to: pts.next().expect("checked point count"),
                    },
                    CommandKind::Cubic => SourceGeometryCommand::Cubic {
                        source_ordinal,
                        control1: a,
                        control2: pts.next().expect("checked point count"),
                        to: pts.next().expect("checked point count"),
                    },
                })
            }
        })
    }
}
