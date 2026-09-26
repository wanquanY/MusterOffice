use crate::support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{text::cascade::*, *},
    *,
};
pub(super) const SLIDE: &str = "/ppt/slides/slide1.xml";
pub(super) const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
pub(super) const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";
pub(super) const THEME: &str = "/ppt/theme/theme2.xml";
pub(super) fn package(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap()
}
pub(super) fn rewrite(b: &[u8], part: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(b);
    let part = PartName::new(part).unwrap();
    let s = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, f(s).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
pub(super) fn replace(mut s: String, name: &str, with: &str) -> String {
    let start = format!("<{name}>");
    let end = format!("</{name}>");
    if let Some(a) = s.find(&start) {
        let z = a + s[a..].find(&end).unwrap() + end.len();
        s.replace_range(a..z, with);
    }
    s.replace(&format!("<{name}/>"), with)
}
pub(super) fn base(list: &str, paragraphs: &str) -> Vec<u8> {
    let (d, v) = support::input();
    let b = export(
        &d,
        &v,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let b = rewrite(&b, MASTER, |s| replace(s, "p:txStyles", ""));
    let b = rewrite(&b, "/ppt/presentation.xml", |s| {
        replace(s, "p:defaultTextStyle", "")
    });
    rewrite(&b, SLIDE, |s| {
        replace(
            s,
            "p:txBody",
            &format!("<p:txBody><a:bodyPr/><a:lstStyle>{list}</a:lstStyle>{paragraphs}</p:txBody>"),
        )
    })
}
pub(super) fn index(b: &[u8]) -> SourceIndex {
    let index = inspect_source(&package(b), SourceLimits::default(), &|| false).unwrap();
    // Optional development evidence: the exact owned packages and actual library
    // results allow a separate XML implementation to check physical provenance.
    // No filesystem dependency is introduced into the computation library.
    if let Some(directory) = std::env::var_os("MO_TEXT_CASCADE_EVIDENCE_DIR") {
        static WRITER: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = WRITER.lock().unwrap();
        let directory = std::path::PathBuf::from(directory);
        assert!(
            directory.is_absolute(),
            "evidence directory must be absolute"
        );
        std::fs::create_dir_all(&directory).unwrap();
        let stem = index.source_sha256.as_str();
        std::fs::write(directory.join(format!("{stem}.pptx")), b).unwrap();
        std::fs::write(
            directory.join(format!("{stem}.json")),
            serde_json::to_vec_pretty(&outcome(&index)).unwrap(),
        )
        .unwrap();
    }
    index
}
pub(super) fn target(i: &SourceIndex) -> SourceObjectRef {
    SourceObjectRef {
        part: SLIDE.into(),
        native_id: i.surfaces[SLIDE].objects[0].native_id,
    }
}
pub(super) fn outcome(i: &SourceIndex) -> TextCascadeOutcome {
    resolve(
        i,
        &i.source_sha256,
        &target(i),
        TextCascadeLimits::default(),
        &|| false,
    )
    .unwrap()
}
pub(super) fn text(i: &SourceIndex) -> CascadedText {
    match outcome(i) {
        TextCascadeOutcome::Cascaded { text } => *text,
        other => panic!("{other:?}"),
    }
}
pub(super) fn run(s: &str) -> String {
    format!("<a:r><a:t>{s}</a:t></a:r>")
}
pub(super) fn p(s: &str) -> String {
    format!("<a:p>{s}</a:p>")
}
pub(super) fn add_master(b: &[u8], styles: &str) -> Vec<u8> {
    rewrite(b, MASTER, |s| {
        s.replace(
            "</p:sldMaster>",
            &format!("<p:txStyles>{styles}</p:txStyles></p:sldMaster>"),
        )
    })
}
pub(super) fn add_theme(b: &[u8], defs: &str) -> Vec<u8> {
    rewrite(b, THEME, |s| {
        s.replace(
            "</a:theme>",
            &format!("<a:objectDefaults>{defs}</a:objectDefaults></a:theme>"),
        )
    })
}
pub(super) fn add_presentation(b: &[u8], list: &str) -> Vec<u8> {
    rewrite(b, "/ppt/presentation.xml", |s| {
        s.replace(
            "</p:presentation>",
            &format!("<p:defaultTextStyle>{list}</p:defaultTextStyle></p:presentation>"),
        )
    })
}
pub(super) fn placeholder(b: &[u8], kind: &str) -> Vec<u8> {
    rewrite(b, SLIDE, |s| {
        let a = s.find("<p:sp>").unwrap();
        s[..a].to_owned()
            + &s[a..].replacen(
                "<p:nvPr/>",
                &format!("<p:nvPr><p:ph type=\"{kind}\" idx=\"0\"/></p:nvPr>"),
                1,
            )
    })
}
pub(super) fn template(b: &[u8], part: &str, list: &str, paragraphs: &str) -> Vec<u8> {
    let s = String::from_utf8(
        package(b)
            .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
            .unwrap(),
    )
    .unwrap();
    let a = s.find("<p:sp>").unwrap();
    let z = a + s[a..].find("</p:sp>").unwrap() + 7;
    let mut shape = replace(
        s[a..z].into(),
        "p:txBody",
        &format!("<p:txBody><a:bodyPr/><a:lstStyle>{list}</a:lstStyle>{paragraphs}</p:txBody>"),
    );
    if part == MASTER {
        shape = shape
            .replace("type=\"ctrTitle\"", "type=\"title\"")
            .replace("type=\"subTitle\"", "type=\"body\"")
            .replace("type=\"obj\"", "type=\"body\"");
    }
    rewrite(b, part, |s| {
        s.replace("</p:spTree>", &format!("{shape}</p:spTree>"))
    })
}
