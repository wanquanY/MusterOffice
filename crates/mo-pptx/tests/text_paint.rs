mod support;
#[path = "support/text.rs"]
#[allow(dead_code)]
mod text_support;
use mo_pptx::{
    PptxError,
    source::{
        color::*,
        text::{paint::*, *},
    },
};
use text_support::*;

fn fill(color: &str) -> String {
    format!("<a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>")
}
fn paints(
    list: &str,
    runs: &str,
    limits: ColorLimits,
) -> Result<Vec<Vec<TextRunPaint>>, TextPaintError> {
    let i = index(&base(list, &p(runs)));
    resolve(&i, &text(&i), &ColorContext::default(), limits, &|| false)
}
fn rgba(p: &TextPaint) -> Option<[u8; 4]> {
    match p {
        TextPaint::None { .. } => None,
        TextPaint::Solid {
            color: ColorSample::Resolved { rgba8, .. },
            ..
        } => Some(*rgba8),
        _ => panic!("resolved paint"),
    }
}
#[test]
fn underline_fill_inherits_independently_and_follow_text_overrides_it() {
    let list = format!(
        "<a:lvl1pPr><a:defRPr u=\"sng\">{}<a:uFill>{}</a:uFill></a:defRPr></a:lvl1pPr>",
        fill("FF0000"),
        fill("FFFF00")
    );
    let runs = format!(
        "{}<a:r><a:rPr>{}</a:rPr><a:t>B</a:t></a:r><a:r><a:rPr>{}<a:uFillTx/></a:rPr><a:t>C</a:t></a:r>",
        run("A"),
        fill("0000FF"),
        fill("00FF00")
    );
    let result = paints(&list, &runs, ColorLimits::default()).unwrap();
    let r = &result[0];
    assert_eq!(
        r.iter().map(|p| rgba(&p.fill)).collect::<Vec<_>>(),
        [
            Some([255, 0, 0, 255]),
            Some([0, 0, 255, 255]),
            Some([0, 255, 0, 255])
        ]
    );
    assert_eq!(
        r.iter()
            .map(|p| rgba(p.underline_fill().unwrap()))
            .collect::<Vec<_>>(),
        [
            Some([255, 255, 0, 255]),
            Some([255, 255, 0, 255]),
            Some([0, 255, 0, 255])
        ]
    );
    let UnderlinePaint::Independent { declaration, fill } = r[0].underline.as_ref().unwrap() else {
        panic!("independent provenance")
    };
    let TextPaint::Solid {
        declaration: child, ..
    } = fill.as_ref()
    else {
        panic!("solid underline")
    };
    assert_eq!(declaration.element, NativeTextElement::UFill);
    assert_eq!(child.element, NativeTextElement::SolidFill);
    assert!(matches!(
        r[2].underline,
        Some(UnderlinePaint::FollowText {
            declaration: Some(_)
        })
    ));
}
#[test]
fn underline_and_glyph_nofill_do_not_hide_each_other() {
    for (glyph, underline, expected_glyph, expected_underline) in [
        (
            "<a:noFill/>".to_owned(),
            fill("00FF00"),
            None,
            Some([0, 255, 0, 255]),
        ),
        (
            fill("0000FF"),
            "<a:noFill/>".to_owned(),
            Some([0, 0, 255, 255]),
            None,
        ),
    ] {
        let runs = format!(
            "<a:r><a:rPr u=\"sng\" strike=\"sngStrike\">{glyph}<a:uFill>{underline}</a:uFill></a:rPr><a:t>A</a:t></a:r>"
        );
        let result = paints("", &runs, ColorLimits::default()).unwrap();
        let r = &result[0][0];
        assert_eq!(rgba(&r.fill), expected_glyph);
        assert_eq!(rgba(r.underline_fill().unwrap()), expected_underline);
        assert!(r.strike);
    }
    let inactive = format!(
        "<a:r><a:rPr u=\"none\">{}<a:uFill><a:solidFill><a:sysClr val=\"window\"/></a:solidFill></a:uFill></a:rPr><a:t>A</a:t></a:r>",
        fill("0000FF")
    );
    assert!(
        paints("", &inactive, ColorLimits::default()).unwrap()[0][0]
            .underline
            .is_none()
    );
}
#[test]
fn underline_expressions_share_color_budgets_and_keep_inner_declaration_errors() {
    let independent = format!(
        "<a:r><a:rPr u=\"sng\">{}<a:uFill>{}</a:uFill></a:rPr><a:t>A</a:t></a:r>",
        fill("0000FF"),
        fill("FF0000")
    );
    let follow = independent.replace(
        &format!("<a:uFill>{}</a:uFill>", fill("FF0000")),
        "<a:uFillTx/>",
    );
    for limits in [
        ColorLimits {
            max_queries: 1,
            ..Default::default()
        },
        ColorLimits {
            max_steps: 1,
            ..Default::default()
        },
    ] {
        assert!(paints("", &follow, limits).is_ok());
        assert!(
            matches!(paints("",&independent,limits),Err(TextPaintError::AtRun{error,..}) if matches!(*error,TextPaintError::Source(PptxError::Limit(_))))
        );
    }
    let bad = independent.replace(
        &format!("<a:uFill>{}</a:uFill>", fill("FF0000")),
        "<a:uFill><a:pattFill prst=\"pct5\"/></a:uFill>",
    );
    assert!(
        matches!(paints("",&bad,ColorLimits::default()),Err(TextPaintError::AtRun{location,error}) if location.run==0 && matches!(error.as_ref(),TextPaintError::Declaration(d) if d.element==NativeTextElement::PattFill))
    );
    let bytes = base("", &p(&independent));
    let i = index(&bytes);
    let t = text(&i);
    assert!(matches!(
        resolve(
            &i,
            &t,
            &ColorContext::default(),
            ColorLimits::default(),
            &|| true
        ),
        Err(TextPaintError::Source(PptxError::Cancelled))
    ));
}
