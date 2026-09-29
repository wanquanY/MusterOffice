#[allow(dead_code)]
#[path = "table_styles/support.rs"]
mod support;
use mo_pptx::{
    PptxError,
    source::{
        SourceIndex,
        color::*,
        table::{SourceCellAddress, grid::*},
        text::{cascade::*, fonts::*, paint::*, *},
    },
};
use support::*;
fn at(row: u32, column: u32) -> SourceCellAddress {
    SourceCellAddress { row, column }
}
fn fixture(style_body: &str, inline_style: bool) -> Vec<u8> {
    let base = table::bytes();
    if inline_style {
        inline(&base, &style("tableStyle", ID, style_body))
    } else {
        let base = inline(&base, &format!("<a:tableStyleId>{ID}</a:tableStyleId>"));
        catalog(&base, &list(&style("tblStyle", ID, style_body)))
    }
}
fn cell_body(bytes: &[u8], cell: usize, body: &str) -> Vec<u8> {
    table::rewrite(bytes, |mut s| {
        let a = s.match_indices("<a:txBody>").nth(cell).unwrap().0;
        let z = a + s[a..].find("</a:txBody>").unwrap() + "</a:txBody>".len();
        s.replace_range(a..z, &format!("<a:txBody><a:bodyPr/>{body}</a:txBody>"));
        s
    })
}
fn bound(i: &SourceIndex) -> TableTextResolver<'_> {
    match TableTextResolver::bind(
        i,
        &i.source_sha256,
        &table::target(i),
        Default::default(),
        Default::default(),
        &|| false,
    )
    .unwrap()
    {
        TableTextBinding::Bound { table } => table,
        TableTextBinding::Unresolved { reason } => panic!("{reason:?}"),
    }
}
fn cascaded(table: &TableTextResolver<'_>, cell: SourceCellAddress) -> CascadedText {
    match table.resolve(cell, Default::default(), &|| false).unwrap() {
        TextCascadeOutcome::Cascaded { text } => *text,
        other => panic!("{other:?}"),
    }
}
fn font(
    i: &SourceIndex,
    t: &CascadedText,
    slot: NativeFontSlot,
    script: Option<&str>,
) -> TypefaceOutcome {
    fonts::resolve(i, t, 0, Some(0), slot, script, Default::default(), &|| {
        false
    })
    .unwrap()
}
fn named(
    i: &SourceIndex,
    t: &CascadedText,
    slot: NativeFontSlot,
    script: Option<&str>,
) -> NativeTypeface {
    match font(i, t, slot, script) {
        TypefaceOutcome::Named { font } => *font,
        other => panic!("{other:?}"),
    }
}
fn colors(i: &SourceIndex, t: &CascadedText) -> Vec<Vec<TextRunPaint>> {
    paint::resolve(i, t, &Default::default(), Default::default(), &|| false).unwrap()
}
fn rgba(p: &TextPaint) -> [u8; 4] {
    match p {
        TextPaint::Solid {
            color: ColorSample::Resolved { rgba8, .. },
            ..
        } => *rgba8,
        other => panic!("{other:?}"),
    }
}
const COLLECTION: &str = "<a:font><a:latin typeface=\"Table Latin\"/><a:ea typeface=\"\"/><a:cs typeface=\"Table CS\"/><a:font script=\"Hans\" typeface=\"Table Hans\"/></a:font>";
fn whole() -> String {
    format!(
        "<a:wholeTbl><a:tcTxStyle b=\"on\" i=\"on\">{COLLECTION}<a:srgbClr val=\"102030\"/></a:tcTxStyle></a:wholeTbl>"
    )
}
#[test]
fn native_cell_scopes_preserve_flat_paragraphs_and_real_declaration_origins() {
    for inline_style in [false, true] {
        let bytes = cell_body(
            &fixture(&whole(), inline_style),
            4,
            "<a:lstStyle/><a:p><a:r><a:t>first</a:t></a:r></a:p><a:p><a:r><a:t>second</a:t></a:r></a:p>",
        );
        let i = read(&bytes);
        let resolver = bound(&i);
        for row in 0..3 {
            for col in 0..3 {
                let text = cascaded(&resolver, at(row, col));
                assert_eq!(text.object, table::target(&i));
                assert_eq!(text.cell, Some(at(row, col)));
                let flat = row * 3 + col;
                assert_eq!(text.paragraph_start, flat + u32::from(flat > 4));
                assert_eq!(text.paragraphs.len(), if flat == 4 { 2 } else { 1 });
                assert_eq!(text.native_paragraph(0), Some(text.paragraph_start));
                assert_eq!(text.native_paragraph(text.paragraphs.len() as u32), None);
                if flat == 8 {
                    continue;
                } // Native fixture's final cell is empty.
                let latin = named(&i, &text, NativeFontSlot::Latin, None);
                assert_eq!(latin.typeface, "Table Latin");
                assert_eq!(latin.table_font.unwrap().slot, NativeFontSlot::Latin);
                let TextStyleOrigin::TableStyle { source, .. } = &latin.declared_by.origin else {
                    panic!("native table provenance")
                };
                assert!(matches!(source, TableTextStyleSource::Inline { .. }) == inline_style);
                assert!(declaration(&i, &latin.declared_by).is_err()); // No synthetic text node.
                assert!(matches!(
                    table_declaration(&i, &latin.declared_by).unwrap(),
                    TableTextDeclaration::Font(_)
                ));
                assert_eq!(rgba(&colors(&i, &text)[0][0].fill), [16, 32, 48, 255]);
            }
        }
        let t = cascaded(&resolver, at(1, 1));
        assert_eq!(t.paragraphs.len(), 2);
        assert_eq!(resolver.grid().region(at(1, 1)).unwrap().origin, at(0, 0));
    }
}
#[test]
fn direct_run_paragraph_and_local_list_styles_override_individual_table_properties() {
    let bytes = cell_body(
        &fixture(&whole(), false),
        2,
        "<a:lstStyle><a:lvl1pPr><a:defRPr sz=\"2300\" b=\"0\"><a:latin typeface=\"Cell List\"/></a:defRPr></a:lvl1pPr></a:lstStyle><a:p><a:pPr><a:defRPr i=\"0\"><a:solidFill><a:srgbClr val=\"405060\"/></a:solidFill></a:defRPr></a:pPr><a:r><a:rPr b=\"1\"><a:latin typeface=\"Direct Run\"/></a:rPr><a:t>A</a:t></a:r><a:r><a:t>B</a:t></a:r><a:endParaRPr sz=\"1100\"/></a:p>",
    );
    let i = read(&bytes);
    let t = cascaded(&bound(&i), at(0, 2));
    let p = &t.paragraphs[0];
    assert_eq!(p.runs[0].style.attributes.bold, Some(true));
    assert_eq!(p.runs[1].style.attributes.bold, Some(false));
    assert_eq!(p.runs[0].style.attributes.italic, Some(false));
    assert_eq!(p.runs[0].style.attributes.size, Some(2300));
    assert_eq!(p.end_style.attributes.size, Some(1100));
    assert_eq!(
        named(&i, &t, NativeFontSlot::Latin, None).typeface,
        "Direct Run"
    );
    let TypefaceOutcome::Named { font } = fonts::resolve(
        &i,
        &t,
        0,
        Some(1),
        NativeFontSlot::Latin,
        None,
        Default::default(),
        &|| false,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(font.typeface, "Cell List");
    assert_eq!(
        named(&i, &t, NativeFontSlot::ComplexScript, None).typeface,
        "Table CS"
    );
    assert!(
        colors(&i, &t)[0]
            .iter()
            .all(|p| rgba(&p.fill) == [64, 80, 96, 255])
    );
}
#[test]
fn selected_regions_override_independently_and_table_fonts_use_ordered_supplements() {
    let style = whole()
        + "<a:firstRow><a:tcTxStyle b=\"off\" i=\"def\"><a:srgbClr val=\"AABBCC\"/></a:tcTxStyle></a:firstRow>";
    let bytes = table::rewrite(&fixture(&style, false), |s| {
        s.replacen("<a:tblPr>", "<a:tblPr firstRow=\"1\">", 1)
    });
    let i = read(&bytes);
    let resolver = bound(&i);
    let t = cascaded(&resolver, at(0, 2));
    assert_eq!(t.paragraphs[0].runs[0].style.attributes.bold, Some(false));
    assert_eq!(t.paragraphs[0].runs[0].style.attributes.italic, Some(true));
    let f = named(&i, &t, NativeFontSlot::EastAsian, Some("Hans"));
    assert_eq!(f.typeface, "Table Hans");
    assert_eq!(f.table_font.unwrap().supplemental, Some(0));
    assert_eq!(rgba(&colors(&i, &t)[0][0].fill), [170, 187, 204, 255]);
    assert!(matches!(
        font(&i, &t, NativeFontSlot::EastAsian, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::ScriptRequired {}
        }
    ));
    assert!(matches!(
        font(&i, &t, NativeFontSlot::EastAsian, Some("Jpan")),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::MissingSupplemental { .. }
        }
    ));
    let b = fixture(
        &whole().replace(
            "<a:font script=\"Hans\" typeface=\"Table Hans\"/>",
            "<a:font script=\"Hans\" typeface=\"One\"/><a:font script=\"Hans\" typeface=\"Two\"/>",
        ),
        true,
    );
    let i = read(&b);
    let t = cascaded(&bound(&i), at(0, 2));
    assert!(matches!(
        font(&i, &t, NativeFontSlot::EastAsian, Some("Hans")),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::AmbiguousSupplemental { .. }
        }
    ));
}
#[test]
fn table_theme_font_reference_and_placeholder_color_use_the_drawing_surface() {
    for inline_style in [false, true] {
        let b = fixture(
            "<a:wholeTbl><a:tcTxStyle><a:fontRef idx=\"minor\"><a:srgbClr val=\"2468AC\"/></a:fontRef><a:schemeClr val=\"phClr\"/></a:tcTxStyle></a:wholeTbl>",
            inline_style,
        );
        let i = read(&b);
        let t = cascaded(&bound(&i), at(0, 2));
        let f = named(&i, &t, NativeFontSlot::Latin, None);
        assert_eq!(
            f.theme.unwrap().collection,
            NativeFontCollectionIndex::Minor
        );
        assert!(f.table_font.is_none());
        let paints = colors(&i, &t);
        assert_eq!(rgba(&paints[0][0].fill), [36, 104, 172, 255]);
        assert!(matches!(
            &paints[0][0].fill,
            TextPaint::Solid {
                placeholder: Some(TextStyleDeclaration {
                    origin: TextStyleOrigin::TableStyle { .. },
                    ..
                }),
                ..
            }
        ));
    }
    let b = fixture(
        "<a:wholeTbl><a:tcTxStyle><a:fontRef idx=\"none\"/><a:srgbClr val=\"000000\"/></a:tcTxStyle></a:wholeTbl>",
        false,
    );
    let i = read(&b);
    let t = cascaded(&bound(&i), at(0, 2));
    assert!(matches!(
        font(&i, &t, NativeFontSlot::Latin, None),
        TypefaceOutcome::Unresolved {
            reason: TypefaceUnresolved::DisabledThemeFont {}
        }
    ));
    assert_eq!(rgba(&colors(&i, &t)[0][0].fill), [0, 0, 0, 255]);
}
#[test]
fn stale_scope_retained_declarations_and_budgets_are_not_silently_accepted() {
    let b = fixture(&whole(), false);
    let i = read(&b);
    let object = table::target(&i);
    assert!(
        TableTextResolver::bind(
            &i,
            &mo_common::Digest::from_sha256([0; 32]),
            &object,
            Default::default(),
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert!(matches!(
        TableTextResolver::bind(
            &i,
            &i.source_sha256,
            &object,
            Default::default(),
            Default::default(),
            &|| true
        ),
        Err(PptxError::Cancelled)
    ));
    assert!(matches!(
        TableTextResolver::bind(
            &i,
            &i.source_sha256,
            &object,
            NativeTableGridLimits {
                max_cells: 8,
                ..Default::default()
            },
            Default::default(),
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    let resolver = bound(&i);
    assert!(
        resolver
            .resolve(at(3, 0), Default::default(), &|| false)
            .is_err()
    );
    for limits in [
        TextCascadeLimits {
            max_paragraphs: 0,
            ..Default::default()
        },
        TextCascadeLimits {
            max_runs: 0,
            ..Default::default()
        },
        TextCascadeLimits {
            max_steps: 0,
            ..Default::default()
        },
        TextCascadeLimits {
            max_lexical_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            resolver.resolve(at(0, 2), limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    assert!(matches!(
        resolver.resolve(at(0, 2), Default::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    let t = cascaded(&resolver, at(0, 2));
    let mut r = t.paragraphs[0].runs[0].style.declarations[&CharacterSlot::Latin].clone();
    if let TextStyleOrigin::TableStyle { source_ordinal, .. } = &mut r.origin {
        *source_ordinal += 1;
    }
    assert!(table_declaration(&i, &r).is_err());
    let b = fixture(
        &whole().replace("<a:wholeTbl>", "<a:wholeTbl unknown=\"yes\">"),
        false,
    );
    let i = read(&b);
    assert!(matches!(
        bound(&i)
            .resolve(at(0, 2), Default::default(), &|| false)
            .unwrap(),
        TextCascadeOutcome::Unresolved {
            reason: TextCascadeUnresolved::TableStyle { .. }
        }
    ));
}
