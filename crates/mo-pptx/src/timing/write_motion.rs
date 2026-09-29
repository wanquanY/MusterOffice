//! Export original editable control points, never the compiled pacing mesh.
use mo_timeline::{MotionPath, MotionPoint, MotionSegment};
use std::fmt::Write;

pub(super) fn path(path: &MotionPath) -> String {
    fn point(out: &mut String, p: &MotionPoint) {
        write!(out, " {} {}", p.x.lexical(), p.y.lexical()).expect("string writer");
    }
    let mut out = String::from("M");
    point(&mut out, &path.from);
    for segment in &path.segments {
        match segment {
            MotionSegment::Line { to } => {
                out.push_str(" L");
                point(&mut out, to);
            }
            MotionSegment::Cubic {
                control1,
                control2,
                to,
            } => {
                out.push_str(" C");
                for p in [control1, control2, to] {
                    point(&mut out, p);
                }
            }
            MotionSegment::Close => out.push_str(" Z"),
        }
    }
    out.push_str(" E");
    out
}
