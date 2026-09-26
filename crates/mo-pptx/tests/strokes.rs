use mo_common::{Emu, ObjectId};
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::*;
use mo_presentation_model::*;
mod support;
use support::{input, resources};

fn exported(cap: Option<LineCap>, join: Option<LineJoin>) -> Result<String, PptxError> {
    let (mut d, defaults) = input();
    let id = d.slides.values().next().unwrap().objects[0].clone();
    d.objects.get_mut(&id).unwrap().appearance.stroke = Inherited::Value(Stroke::Solid {
        color: Color::Srgb {
            rgba: Rgba {
                red: 255,
                green: 0,
                blue: 0,
                alpha: 128,
            },
        },
        width: Emu::new(114300),
        cap,
        join,
    });
    let before = serde_json::to_string(&d).unwrap();
    let bytes = export(&d, &defaults, &resources(), PptxLimits::default(), &|| {
        false
    })?;
    assert_eq!(serde_json::to_string(&d).unwrap(), before);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    Ok(String::from_utf8(
        package
            .read_part(
                &PartName::new("/ppt/slides/slide1.xml").unwrap(),
                1024 * 1024,
                &|| false,
            )
            .unwrap(),
    )
    .unwrap())
}

#[test]
fn native_strokes_preserve_absence_and_explicit_editable_properties() {
    let absent = exported(None, None).unwrap();
    assert!(absent.contains("<a:ln w=\"114300\"><a:solidFill>"));
    for (cap, wire) in [
        (LineCap::Flat, "flat"),
        (LineCap::Round, "rnd"),
        (LineCap::Square, "sq"),
    ] {
        for (join, fragment) in [
            (LineJoin::Round {}, "<a:round/>"),
            (LineJoin::Bevel {}, "<a:bevel/>"),
            (
                LineJoin::Miter {
                    limit: Some(314159),
                },
                "<a:miter lim=\"314159\"/>",
            ),
        ] {
            let xml = exported(Some(cap), Some(join)).unwrap();
            assert!(xml.contains(&format!("<a:ln w=\"114300\" cap=\"{wire}\"><a:solidFill>")));
            assert!(xml.contains(&format!("</a:solidFill>{fragment}</a:ln>")));
            assert!(xml.contains("<a:alpha val=\""));
        }
    }
    assert!(
        exported(
            Some(LineCap::Flat),
            Some(LineJoin::Miter {
                limit: Some(u32::MAX)
            })
        )
        .is_err()
    );
}

#[test]
fn omitted_miter_limit_remains_distinct_from_explicit_zero() {
    let join: LineJoin = serde_json::from_str(r#"{"kind":"miter"}"#).unwrap();
    assert_eq!(join, LineJoin::Miter { limit: None });
    assert_eq!(serde_json::to_string(&join).unwrap(), r#"{"kind":"miter"}"#);
    assert!(
        exported(Some(LineCap::Flat), Some(join))
            .unwrap()
            .contains("<a:miter/>")
    );
    assert!(
        exported(
            Some(LineCap::Flat),
            Some(LineJoin::Miter { limit: Some(0) })
        )
        .unwrap()
        .contains("<a:miter lim=\"0\"/>")
    );
}

#[test]
fn authored_stroke_declarations_are_strict_without_filling_in_absence() {
    for bad in [
        r#"{"kind":"none","cap":"flat"}"#,
        r#"{"kind":"solid","width":"1","color":{"kind":"srgb","rgba":{"red":0,"green":0,"blue":0,"alpha":255}},"cap":"flat","join":{"kind":"round","limit":4}}"#,
    ] {
        assert!(serde_json::from_str::<Stroke>(bad).is_err());
    }
    let (d, _) = input();
    let id = ObjectId::new("title:1").unwrap();
    let o = d.objects.get(&id).unwrap();
    let bytes = serde_json::to_string(&o.appearance.stroke).unwrap();
    assert!(!bytes.contains("\"cap\""));
}
