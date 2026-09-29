//! Connected layout-relative native paths. Decimal control points survive
//! import; playback subdivision never changes this editable representation.
use super::*;
use mo_timeline::{MotionCoordinate, MotionPath, MotionPoint, MotionSegment};

pub(super) fn read<'a>(tree: &'a [Node], anim: &Node) -> Result<(Payload, &'a Node), PptxError> {
    attrs(
        anim,
        &["origin", "path", "pathEditMode", "ptsTypes", "rAng"],
    )?;
    value(anim, "origin", "layout")?;
    if !matches!(
        anim.element.attribute("pathEditMode"),
        None | Some("relative")
    ) {
        return Err(unsupported("absolute motion path edit mode"));
    }
    if anim
        .element
        .attribute("rAng")
        .is_some_and(|v| v.parse::<i32>() != Ok(0))
    {
        return Err(unsupported("rotated motion path"));
    }
    if let Some(types) = anim.element.attribute("ptsTypes")
        && (types.len() > 4096 || !types.bytes().all(|b| b"AFTSafts".contains(&b)))
    {
        return Err(unsupported("motion edit point types"));
    }
    let path = anim
        .element
        .attribute("path")
        .ok_or_else(|| unsupported("empty motion path"))?;
    let path = read_path(path)?;
    let payload = match path.segments.as_slice() {
        [MotionSegment::Line { to }] => Payload::MotionLine {
            from: path.from,
            to: to.clone(),
        },
        _ => Payload::MotionPath(path),
    };
    Ok((payload, single(tree, anim, "cBhvr")?))
}
struct Cursor<'a>(&'a str);
impl Cursor<'_> {
    fn space(&mut self) {
        self.0 = self
            .0
            .trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
    }
    fn command(&mut self) -> Option<u8> {
        self.space();
        let b = *self.0.as_bytes().first()?;
        if !b.is_ascii_alphabetic() {
            return None;
        }
        self.0 = &self.0[1..];
        Some(b)
    }
    fn number(&mut self) -> Result<MotionCoordinate, PptxError> {
        self.space();
        let bytes = self.0.as_bytes();
        let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
            end += 1;
        }
        // Never mistake an unsupported exponent for an End command.
        if matches!(bytes.get(end), Some(b'e' | b'E'))
            && bytes
                .get(end + 1)
                .is_some_and(|b| b.is_ascii_digit() || *b == b'+' || *b == b'-')
        {
            return Err(unsupported("motion scientific notation"));
        }
        let lexical = &self.0[..end];
        self.0 = &self.0[end..];
        lexical
            .strip_prefix('+')
            .unwrap_or(lexical)
            .to_owned()
            .try_into()
            .map_err(|_| unsupported("motion coordinate precision or syntax"))
    }
    fn point(&mut self) -> Result<MotionPoint, PptxError> {
        Ok(MotionPoint {
            x: self.number()?,
            y: self.number()?,
        })
    }
}
fn read_path(path: &str) -> Result<MotionPath, PptxError> {
    if path.len() > 1_048_576 {
        return Err(PptxError::Limit("motion path bytes"));
    }
    let mut cursor = Cursor(path);
    if !matches!(cursor.command(), Some(b'M' | b'm')) {
        return Err(unsupported("motion initial move"));
    }
    let from = cursor.point()?;
    let mut current = from.clone();
    let mut segments = vec![];
    loop {
        cursor.space();
        if cursor.0.is_empty() {
            break;
        }
        let command = cursor
            .command()
            .ok_or_else(|| unsupported("motion segment command"))?;
        // Office ignores all commands after explicit End. Original bytes stay in OPC.
        if matches!(command, b'E' | b'e') {
            break;
        }
        if segments.len() == 4096 {
            return Err(PptxError::Limit("native motion segments"));
        }
        let mut point = || {
            let mut p = cursor.point()?;
            if command.is_ascii_lowercase() {
                p.x = current
                    .x
                    .checked_add(&p.x)
                    .map_err(|_| unsupported("motion control point range"))?;
                p.y = current
                    .y
                    .checked_add(&p.y)
                    .map_err(|_| unsupported("motion control point range"))?;
            }
            Ok::<_, PptxError>(p)
        };
        let segment = match command {
            b'L' | b'l' => MotionSegment::Line { to: point()? },
            b'C' | b'c' => MotionSegment::Cubic {
                control1: point()?,
                control2: point()?,
                to: point()?,
            },
            b'Z' | b'z' => MotionSegment::Close,
            _ => return Err(unsupported("disconnected or unknown motion segment")),
        };
        current = match &segment {
            MotionSegment::Line { to } | MotionSegment::Cubic { to, .. } => to.clone(),
            MotionSegment::Close => from.clone(),
        };
        segments.push(segment);
    }
    if segments.is_empty() {
        return Err(unsupported("motion requires a segment"));
    }
    Ok(MotionPath { from, segments })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_relative_coordinates_and_native_end_are_preserved() {
        let path = read_path("m0.100000000000000001,-0.20 l+0.2 0.30 E C ignored").unwrap();
        let a = path.from;
        let MotionSegment::Line { to: b } = &path.segments[0] else {
            panic!("line")
        };
        assert_eq!(a.x.lexical(), "0.100000000000000001");
        assert_eq!(b.x.lexical(), "0.300000000000000001");
        assert_eq!(b.y.lexical(), "0.1");
        let MotionSegment::Line { to } = &read_path("M0 0 L1 -1").unwrap().segments[0] else {
            panic!("line")
        };
        assert_eq!(to.y.lexical(), "-1");
        for bad in [
            "",
            "M 0 0",
            "M 0 0 L 1",
            "M 0 0 L 1 1 M 2 2 L 3 3",
            "M 0 0 C 0 0 1 1 2",
            "M 0 0 L 1 1e-3",
            "M 0 0 L .5 1",
            "M 0 0 L (ppt_x) 1",
            "M0 0L1 1 2",
            "M0 0LNaN 1",
        ] {
            assert!(read_path(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn relative_cubic_controls_share_the_segment_origin_and_close_restores_it() {
        let p = read_path("M 0.1 0.2 c 0.1 0.2 0.3 0.4 0.5 0.6 z l 0.1 0.1").unwrap();
        let MotionSegment::Cubic {
            control1,
            control2,
            to,
        } = &p.segments[0]
        else {
            panic!("curve")
        };
        assert_eq!(control1.x.lexical(), "0.2");
        assert_eq!(control2.y.lexical(), "0.6");
        assert_eq!(to.x.lexical(), "0.6");
        let MotionSegment::Line { to } = &p.segments[2] else {
            panic!("line")
        };
        assert_eq!(to.y.lexical(), "0.3");
        assert!(read_path(&format!("M0 0 {}", "L0 0 ".repeat(4097))).is_err());
    }
}
