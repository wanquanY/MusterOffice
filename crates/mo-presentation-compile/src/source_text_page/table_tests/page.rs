//! Public native table page compilation, not direct private text-layer calls.
use super::cases::{edit_cell, two_tables};
use super::*;

pub(super) fn uniform_borders(bytes: &[u8]) -> Vec<u8> {
    rewrite(bytes, SLIDE, |mut s| {
        let starts = s
            .match_indices("<a:tcPr")
            .map(|(n, _)| n)
            .collect::<Vec<_>>();
        for start in starts.into_iter().rev() {
            let end = start + s[start..].find("</a:tcPr>").unwrap();
            let mut props = s[start..end].to_owned();
            for edge in ["lnL", "lnR", "lnT", "lnB", "lnTlToBr", "lnBlToTr"] {
                if let Some(at) = props.find(&format!("<a:{edge}")) {
                    let end =
                        at + props[at..].find(&format!("</a:{edge}>")).unwrap() + edge.len() + 5;
                    props.replace_range(at..end, "");
                }
            }
            let insert = props.find('>').unwrap() + 1;
            let borders = ["lnL", "lnR", "lnT", "lnB"].iter().map(|e| format!(
                "<a:{e} w=\"25400\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:srgbClr val=\"224466\"/></a:solidFill><a:prstDash val=\"solid\"/><a:round/><a:headEnd type=\"none\"/><a:tailEnd type=\"none\"/></a:{e}>"
            )).collect::<String>();
            props.insert_str(insert, &borders);
            s.replace_range(start..end, &props);
        }
        s
    })
}
pub(super) fn page_request(index: &SourceIndex) -> SourcePageRequest {
    let mut q = request(index);
    let size = index.page_size.unwrap();
    q.viewport.width = (size.width.get() as u64).div_ceil(12700) as u32;
    q.viewport.height = (size.height.get() as u64).div_ceil(12700) as u32;
    q
}

#[test]
fn native_table_reaches_public_text_page_without_synthetic_objects() {
    let bytes = uniform_borders(&fixture(false));
    let index = read(&bytes);
    let q = page_request(&index);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let plan = crate::source_text_page::compile(
        &index,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(plan.texts.len(), 6);
    assert_eq!(
        plan.page
            .info
            .layers
            .iter()
            .map(|l| l.objects.len())
            .sum::<usize>(),
        1
    );
    assert_eq!(plan.page.bindings[1].location.object, Some(2));
    assert!(
        plan.page
            .bindings
            .iter()
            .skip(1)
            .all(|b| b.location.object == Some(2))
    );
    assert!(plan.page.paint_sources.iter().any(|p| p.binding > 1));
    let pixels = raster(&q, plan.page.raster.scene);
    assert!(pixels.chunks_exact(4).filter(|p| *p != [255; 4]).count() > 1000);
}

#[test]
fn conflicting_native_borders_are_diagnosed_before_shaping() {
    let index = read(&fixture(false));
    let q = page_request(&index);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    let error = crate::source_text_page::compile(
        &index,
        &q,
        &manifest,
        &mut shaper,
        Default::default(),
        &|| false,
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("TableBorderConflict"));
    assert_eq!(shaper.calls, 0);
}

fn public_compile(bytes: &[u8]) -> (SourcePageRequest, SourceTextPagePlan, Vec<u8>) {
    let index = read(bytes);
    let q = page_request(&index);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let plan = crate::source_text_page::compile(
        &index,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let pixels = raster(&q, plan.page.raster.scene.clone());
    (q, plan, pixels)
}
fn save(name: &str, suffix: &str, bytes: &[u8]) {
    let Some(dir) = std::env::var_os("MO_TABLE_PAGE_EVIDENCE_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    assert!(dir.is_absolute());
    std::fs::create_dir_all(&dir).unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(format!("{name}.{suffix}")))
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
fn save_case(
    name: &str,
    bytes: &[u8],
    q: &SourcePageRequest,
    plan: &SourceTextPagePlan,
    pixels: &[u8],
) {
    save(name, "pptx", bytes);
    save(name, "request.json", &serde_json::to_vec(q).unwrap());
    save(name, "plan.json", &serde_json::to_vec(plan).unwrap());
    save(name, "rgba", pixels);
}
pub(super) fn gradient_cell(bytes: &[u8], kind: &str) -> Vec<u8> {
    let geometry = match kind {
        "linear" => "<a:lin ang=\"0\" scaled=\"0\"/>",
        "rect" => {
            "<a:path path=\"rect\"><a:fillToRect l=\"50000\" t=\"50000\" r=\"50000\" b=\"50000\"/></a:path>"
        }
        "circle" => {
            "<a:path path=\"circle\"><a:fillToRect l=\"50000\" t=\"50000\" r=\"50000\" b=\"50000\"/></a:path>"
        }
        _ => panic!("gradient fixture"),
    };
    edit_cell(bytes, 2, |s| {
        let at = s.rfind("<a:solidFill>").unwrap();
        let end = at + s[at..].find("</a:solidFill>").unwrap() + 14;
        format!(
            "{}<a:gradFill rotWithShape=\"1\"><a:gsLst><a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs></a:gsLst>{geometry}</a:gradFill>{}",
            &s[..at],
            &s[end..]
        )
    })
}
#[test]
fn public_merged_and_rtl_cells_use_their_own_gradient_geometry() {
    for rtl in [false, true] {
        for kind in ["linear", "rect", "circle"] {
            let bytes = gradient_cell(&uniform_borders(&fixture(false)), kind);
            let bytes = rewrite(&bytes, SLIDE, |s| {
                s.replace("<a:tblPr", &format!("<a:tblPr rtl=\"{}\"", u8::from(rtl)))
            });
            let (q, plan, pixels) = public_compile(&bytes);
            assert_eq!(plan.texts.len(), 6);
            assert_eq!(
                plan.page
                    .bindings
                    .iter()
                    .filter(|b| matches!(b.fill.target, FillTarget::TableCell { .. }))
                    .count(),
                6
            );
            // Declared 3 x 3 grid has 24 physical horizontal/vertical segments.
            // A 2 x 2 merge hides four internal segments; shared sides emit once.
            assert_eq!(
                plan.page
                    .bindings
                    .iter()
                    .filter(|b| b.table_stroke.is_some())
                    .count(),
                20
            );
            assert!(
                plan.page
                    .paint_sources
                    .iter()
                    .all(|s| s.binding <= 1 || s.fill_target.is_some())
            );
            if kind == "linear" {
                // Independent XML geometry: each column is 144 pixels. Cell (0,2)
                // starts at x=288 in LTR, x=0 in RTL. Gradient remains physical LTR.
                let left = if rtl { 0 } else { 288 };
                for x in [20usize, 55, 110] {
                    let pixel = &pixels[((10 * q.viewport.width as usize) + left + x) * 4..][..4];
                    let t = (x as f64 + 0.5) / 144.0;
                    let expected = [
                        255.0 * (1.0 - t.powf(1.875)),
                        0.0,
                        255.0 * (1.0 - (1.0 - t).powf(1.875)),
                        255.0,
                    ];
                    for c in 0..4 {
                        assert!(
                            (f64::from(pixel[c]) - expected[c]).abs() <= 1.0,
                            "{rtl}/{x}: {pixel:?} vs {expected:?}"
                        );
                    }
                }
            }
            save_case(&format!("{kind}-rtl-{rtl}"), &bytes, &q, &plan, &pixels);
        }
    }
}

#[test]
fn public_page_preflight_and_table_order_are_transactional() {
    let bytes = two_tables(&uniform_borders(&fixture(false)));
    let (q, plan, pixels) = public_compile(&bytes);
    assert_eq!(plan.texts.len(), 12);
    assert_eq!(plan.texts[0].frame.text.object.native_id, 2);
    assert_eq!(plan.texts[6].frame.text.object.native_id, 990);
    assert_ne!(plan.texts[0].binding, plan.texts[6].binding);
    save_case("two-tables", &bytes, &q, &plan, &pixels);
    // A bad late object must fail before the first table invokes the shaper.
    let bytes = two_tables(&fixture(false));
    let bytes = rewrite(&bytes, SLIDE, |s| {
        let at = s.find("id=\"990\"").unwrap();
        format!(
            "{}{}",
            &s[..at],
            s[at..].replacen("<a:r>", "<a:r><a:rPr cap=\"all\"/>", 1)
        )
    });
    let bytes = uniform_borders(&bytes);
    let index = read(&bytes);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    assert!(
        crate::source_text_page::compile(
            &index,
            &page_request(&index),
            &manifest,
            &mut shaper,
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!(shaper.calls, 0);
    assert!(
        crate::source_text_page::compile(
            &read(&uniform_borders(&fixture(false))),
            &q,
            &manifest,
            &mut shaper,
            Default::default(),
            &|| true
        )
        .is_err()
    );
    assert_eq!(shaper.calls, 0);
}

mod playback;

#[test]
fn aggregate_table_query_budget_precedes_grid_allocation_and_host_calls() {
    let mut index = read(&two_tables(&uniform_borders(&fixture(false))));
    // Adversarial inspected index: each table alone fits the query reservation,
    // but their total exceeds it. No copied cell payload is ever interpreted.
    for o in &mut index.surfaces.get_mut(SLIDE).unwrap().objects {
        let Some(t) = &mut o.table else {
            continue;
        };
        t.columns.resize(70, t.columns[0].clone());
        let mut row = t.rows[0].clone();
        row.cells.resize(70, row.cells[0].clone());
        t.rows = vec![row; 70];
    }
    let q = page_request(&index);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    let error = crate::source_text_page::compile(
        &index,
        &q,
        &manifest,
        &mut shaper,
        Default::default(),
        &|| false,
    )
    .unwrap_err();
    assert!(
        format!("{error:?}").contains("page table paint queries"),
        "{error:?}"
    );
    assert_eq!(shaper.calls, 0);
    index
        .surfaces
        .get_mut(SLIDE)
        .unwrap()
        .objects
        .iter_mut()
        .find(|o| o.native_id == 2)
        .unwrap()
        .table
        .as_mut()
        .unwrap()
        .columns
        .clear();
    let error = crate::source_text_page::compile(
        &index,
        &q,
        &manifest,
        &mut shaper,
        Default::default(),
        &|| false,
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("RowWidth"));
    assert_eq!(shaper.calls, 0);
}
