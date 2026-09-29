#[allow(dead_code)]
#[path = "table_styles/support.rs"]
mod support;
use mo_pptx::source::{
    SourceIndex,
    table::{SourceCellAddress, SourceTable, grid::NativeTableGrid, styles::*},
};
use support::*;
fn native(i: &SourceIndex) -> &SourceTable {
    i.surfaces[table::SLIDE].objects[0].table.as_ref().unwrap()
}
fn fixture(flags: &str, body: &str) -> SourceIndex {
    let b = table::rewrite(&table::bytes(), |s| {
        s.replacen("<a:tblPr>", &format!("<a:tblPr {flags}>"), 1)
    });
    read(&inline(&b, &style("tableStyle", ID, body)))
}
fn empty_regions() -> String {
    REGIONS.into_iter().map(|r| format!("<a:{r}/>")).collect()
}
fn regions(
    bound: &BoundTableStyle<'_>,
    grid: &NativeTableGrid<'_>,
    row: u32,
    column: u32,
) -> Vec<TableStyleRegion> {
    bound
        .cell(grid, SourceCellAddress { row, column }, &|| false)
        .unwrap()
        .iter()
        .map(|v| v.region)
        .collect()
}
#[test]
fn physical_cells_select_ordered_layers_and_headers_adjust_band_phase() {
    use TableStyleRegion::*;
    let i = fixture(
        "firstRow=\"1\" lastRow=\"1\" firstCol=\"1\" lastCol=\"1\" bandRow=\"1\" bandCol=\"1\"",
        &empty_regions(),
    );
    let t = native(&i);
    let grid = NativeTableGrid::compile(t, Default::default(), &|| false).unwrap();
    let bound = BoundTableStyle::bind(t, None, &|| false).unwrap();
    for (r, c, expected) in [
        (0, 0, vec![WholeTbl, FirstCol, FirstRow, NwCell]),
        (0, 1, vec![WholeTbl, Band1V, FirstRow]),
        (0, 2, vec![WholeTbl, LastCol, FirstRow, NeCell]),
        (1, 0, vec![WholeTbl, Band1H, FirstCol]),
        (1, 1, vec![WholeTbl, Band1H, Band1V]),
        (1, 2, vec![WholeTbl, Band1H, LastCol]),
        (2, 0, vec![WholeTbl, FirstCol, LastRow, SwCell]),
        (2, 1, vec![WholeTbl, Band1V, LastRow]),
        (2, 2, vec![WholeTbl, LastCol, LastRow, SeCell]),
    ] {
        assert_eq!(regions(&bound, &grid, r, c), expected);
    }
    let i = fixture("bandRow=\"1\" bandCol=\"1\"", &empty_regions());
    let t = native(&i);
    let grid = NativeTableGrid::compile(t, Default::default(), &|| false).unwrap();
    let bound = BoundTableStyle::bind(t, None, &|| false).unwrap();
    assert_eq!(regions(&bound, &grid, 0, 0), [WholeTbl, Band1H, Band1V]);
    assert_eq!(regions(&bound, &grid, 1, 1), [WholeTbl, Band2H, Band2V]);
    assert_eq!(regions(&bound, &grid, 2, 2), [WholeTbl, Band1H, Band1V]);
}
#[test]
fn property_selection_preserves_origins_and_def_does_not_erase_parent_values() {
    use TableStyleRegion::*;
    let i = fixture(
        "firstRow=\"1\" firstCol=\"1\"",
        "<a:wholeTbl><a:tcTxStyle b=\"on\" i=\"on\"><a:fontRef idx=\"major\"/><a:srgbClr val=\"112233\"/></a:tcTxStyle></a:wholeTbl><a:firstCol><a:tcTxStyle b=\"off\"><a:srgbClr val=\"445566\"/></a:tcTxStyle></a:firstCol><a:firstRow><a:tcTxStyle b=\"def\" i=\"off\"><a:fontRef idx=\"none\"/></a:tcTxStyle></a:firstRow><a:nwCell><a:tcTxStyle i=\"def\"/></a:nwCell>",
    );
    let before = i.clone();
    let t = native(&i);
    let grid = NativeTableGrid::compile(t, Default::default(), &|| false).unwrap();
    let bound = BoundTableStyle::bind(t, None, &|| false).unwrap();
    let cells = bound
        .cell(&grid, SourceCellAddress { row: 0, column: 0 }, &|| false)
        .unwrap();
    let text = cells.text(&|| false).unwrap();
    let bold = text.bold.unwrap();
    assert!(!bold.value);
    assert_eq!(bold.region, FirstCol);
    let italic = text.italic.unwrap();
    assert!(!italic.value);
    assert_eq!(italic.region, FirstRow);
    assert_eq!(text.color.unwrap().region, FirstCol);
    assert_eq!(text.font.unwrap().region, FirstRow);
    let (definition, _) = bound.definition().unwrap();
    assert_eq!(
        bold.source_ordinal,
        definition.parts[&FirstCol]
            .text
            .as_ref()
            .unwrap()
            .source_ordinal
    );
    assert!(std::ptr::eq(
        text.font.unwrap().value,
        definition.parts[&FirstRow]
            .text
            .as_ref()
            .unwrap()
            .font
            .as_ref()
            .unwrap()
    ));
    assert_eq!(i, before);
}
#[test]
fn single_cell_overlap_applies_bottom_parts_before_top_parts() {
    use TableStyleRegion::*;
    let i = fixture(
        "firstRow=\"1\" lastRow=\"1\" firstCol=\"1\" lastCol=\"1\"",
        &empty_regions(),
    );
    let mut t = native(&i).clone();
    t.columns.truncate(1);
    t.rows.truncate(1);
    t.rows[0].cells.truncate(1);
    let cell = &mut t.rows[0].cells[0];
    cell.row_span = None;
    cell.grid_span = None;
    let grid = NativeTableGrid::compile(&t, Default::default(), &|| false).unwrap();
    let bound = BoundTableStyle::bind(&t, None, &|| false).unwrap();
    assert_eq!(
        regions(&bound, &grid, 0, 0),
        [
            WholeTbl, LastCol, FirstCol, LastRow, SeCell, SwCell, FirstRow, NeCell, NwCell
        ]
    );
}
#[test]
fn shared_binding_uses_explicit_identity_and_never_applies_insertion_default() {
    let base = table::bytes();
    let b = catalog(&base, &list(&style("tblStyle", ID, "<a:wholeTbl/>")));
    let i = read(&b);
    let t = native(&i);
    assert!(
        BoundTableStyle::bind(t, i.table_styles.as_deref(), &|| false)
            .unwrap()
            .definition()
            .is_none()
    );
    let b = inline(
        &b,
        &format!("<a:tableStyleId>{}</a:tableStyleId>", ID.to_lowercase()),
    );
    let i = read(&b);
    let bound = BoundTableStyle::bind(native(&i), i.table_styles.as_deref(), &|| false).unwrap();
    let (style, location) = bound.definition().unwrap();
    let TableStyleLocation::Shared(part) = location else {
        panic!()
    };
    assert_eq!(part.part, STYLES);
    assert!(std::ptr::eq(style, &part.styles[ID]));
    assert!(matches!(
        BoundTableStyle::bind(native(&i), None, &|| false),
        Err(TableStyleSelectionError::MissingDefinition)
    ));
    let mut forged = native(&i).clone();
    forged.properties.as_mut().unwrap().style_id = Some("invalid".into());
    assert!(matches!(
        BoundTableStyle::bind(&forged, i.table_styles.as_deref(), &|| false),
        Err(TableStyleSelectionError::InvalidIdentity)
    ));
    forged.properties.as_mut().unwrap().style_id = Some(ID2.into());
    assert!(matches!(
        BoundTableStyle::bind(&forged, i.table_styles.as_deref(), &|| false),
        Err(TableStyleSelectionError::MissingDefinition)
    ));
    let local = fixture("", "");
    forged.properties.as_mut().unwrap().inline_style = native(&local)
        .properties
        .as_ref()
        .unwrap()
        .inline_style
        .clone();
    assert!(matches!(
        BoundTableStyle::bind(&forged, i.table_styles.as_deref(), &|| false),
        Err(TableStyleSelectionError::ConflictingStyles)
    ));
}
#[test]
fn only_selected_retained_regions_block_text_and_limits_never_return_partial_selection() {
    let i = fixture(
        "firstRow=\"1\"",
        "<a:tblBg unknown=\"background-only\"/><a:wholeTbl><a:tcTxStyle b=\"on\"/></a:wholeTbl><a:firstRow unknown=\"header-only\"/>",
    );
    let t = native(&i);
    let grid = NativeTableGrid::compile(t, Default::default(), &|| false).unwrap();
    let bound = BoundTableStyle::bind(t, None, &|| false).unwrap();
    let header = SourceCellAddress { row: 0, column: 0 };
    let body = SourceCellAddress { row: 2, column: 0 };
    assert!(matches!(
        bound
            .cell(&grid, header, &|| false)
            .unwrap()
            .text(&|| false),
        Err(TableStyleSelectionError::RetainedDeclaration { .. })
    ));
    assert!(
        bound
            .cell(&grid, body, &|| false)
            .unwrap()
            .text(&|| false)
            .unwrap()
            .bold
            .unwrap()
            .value
    );
    let other = t.clone();
    let other_grid = NativeTableGrid::compile(&other, Default::default(), &|| false).unwrap();
    assert!(matches!(
        bound.cell(&other_grid, body, &|| false),
        Err(TableStyleSelectionError::GridMismatch)
    ));
    assert!(matches!(
        bound.cell(&grid, SourceCellAddress { row: 3, column: 0 }, &|| false),
        Err(TableStyleSelectionError::CellOutsideGrid)
    ));
    let calls = std::cell::Cell::new(0);
    assert!(matches!(
        bound.cell(&grid, body, &|| {
            calls.set(calls.get() + 1);
            calls.get() > 2
        }),
        Err(TableStyleSelectionError::Cancelled)
    ));
    assert!(matches!(
        bound.cell(&grid, body, &|| false).unwrap().text(&|| true),
        Err(TableStyleSelectionError::Cancelled)
    ));
}
