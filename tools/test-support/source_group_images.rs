//! Original paired sources: inherited group properties and explicit equivalent
//! fills. The controls keep each receiver's native geometry and transform.
#[allow(dead_code)]
#[path = "source_resource_page.rs"]
mod resource;
pub use resource::*;

pub struct GroupImageCase {
    pub name: &'static str,
    pub inherited: Vec<u8>,
    pub explicit: Vec<u8>,
    pub expected_image_uses: usize,
}
pub fn receiver(id: u32, box_emu: [i64; 4], orientation: &str, fill: &str) -> String {
    let [x, y, w, h] = box_emu;
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Owned group receiver\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm {orientation}><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"/>{fill}<a:ln><a:noFill/></a:ln></p:spPr></p:sp>"
    )
}
pub fn group(id: u32, transform: &str, fill: &str, children: &str) -> String {
    format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"{id}\" name=\"Owned fill group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{transform}{fill}</p:grpSpPr>{children}</p:grpSp>"
    )
}
pub fn transform(orientation: &str, target: [i64; 4], child: [i64; 4]) -> String {
    let [x, y, w, h] = target;
    let [cx, cy, cw, ch] = child;
    format!(
        "<a:xfrm {orientation}><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/><a:chOff x=\"{cx}\" y=\"{cy}\"/><a:chExt cx=\"{cw}\" cy=\"{ch}\"/></a:xfrm>"
    )
}
pub const INHERIT: &str = "<a:grpFill/>";
fn paired(name: &'static str, shapes: &str, fill: &str, count: usize) -> GroupImageCase {
    GroupImageCase {
        name,
        inherited: image_fixture(shapes),
        explicit: image_fixture(&shapes.replace(INHERIT, fill)),
        expected_image_uses: count,
    }
}
pub fn cases() -> Vec<GroupImageCase> {
    let image = blip("owned-image", "", STRETCH);
    let children = receiver(42, [0, 0, 800000, 800000], "", INHERIT)
        + &receiver(43, [800000, 0, 800000, 800000], "", INHERIT);
    let frame = transform("", [0, 0, 1600000, 800000], [0, 0, 1600000, 800000]);
    let mut cases = vec![paired(
        "siblings",
        &group(90, &frame, &image, &children),
        &image,
        2,
    )];
    let root = image_fixture(&children);
    cases.push(GroupImageCase {
        name: "root",
        inherited: rewrite(&root, SLIDE, |s| {
            s.replace("<p:grpSpPr/>", &format!("<p:grpSpPr>{image}</p:grpSpPr>"))
        }),
        explicit: image_fixture(&children.replace(INHERIT, &image)),
        expected_image_uses: 2,
    });
    let varied = receiver(42, [40000, 80000, 600000, 400000], "", INHERIT)
        + &receiver(43, [840000, 200000, 400000, 600000], "flipH=\"1\"", INHERIT);
    cases.push(paired(
        "different-receivers",
        &group(90, &frame, &image, &varied),
        &image,
        2,
    ));
    for (name, mode) in [
        (
            "stretch-clip",
            "<a:stretch><a:fillRect l=\"25000\" r=\"25000\"/></a:stretch>",
        ),
        (
            "tile",
            "<a:tile tx=\"25000\" ty=\"-12500\" sx=\"10000000\" sy=\"10000000\" flip=\"xy\" algn=\"ctr\"/>",
        ),
    ] {
        let fill = blip("owned-image", "dpi=\"9144\"", mode);
        cases.push(paired(name, &group(90, &frame, &fill, &children), &fill, 2));
    }
    let outer = transform(
        "rot=\"1800000\" flipH=\"1\"",
        [120000, 160000, 1200000, 760000],
        [40000, 80000, 1600000, 1000000],
    );
    let inner = transform(
        "rot=\"2700000\" flipV=\"1\"",
        [200000, 120000, 800000, 600000],
        [160000, -40000, 1600000, 800000],
    );
    let nested = group(91, &inner, INHERIT, &varied);
    cases.push(paired(
        "nested-oriented",
        &group(90, &outer, &image, &nested),
        &image,
        2,
    ));
    cases.push(paired(
        "nearest-group-override",
        &group(
            90,
            &outer,
            &blip("owned-cyan", "", STRETCH),
            &group(91, &inner, &image, &varied),
        ),
        &image,
        2,
    ));
    // The receiver can inherit grpFill from a placeholder on another surface.
    // Neither its layout ancestor's box nor the declaring root group is its
    // actual page placement; resource relationships still belong to that part.
    let placeholder = |s: String| {
        s.replace(
            "<p:nvPr/>",
            "<p:nvPr><p:ph type=\"body\" idx=\"77\"/></p:nvPr>",
        )
    };
    let ancestor = placeholder(receiver(42, [0, 0, 800000, 800000], "", INHERIT));
    let child = placeholder(receiver(
        142,
        [200000, 200000, 400000, 600000],
        "flipH=\"1\"",
        "",
    ));
    let inherited = image_fixture(&child);
    let inherited = rewrite(&inherited, "/ppt/slideLayouts/slideLayout2.xml", |s| {
        let start = s.find("<p:cSld").unwrap();
        let end = s.find("</p:cSld>").unwrap() + "</p:cSld>".len();
        let mut s = s;
        s.replace_range(start..end, &format!("<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{image}</p:grpSpPr>{ancestor}</p:spTree></p:cSld>"));
        s
    });
    let inherited = rewrite(&inherited, "/ppt/slideMasters/slideMaster2.xml", |s| {
        let start = s.find("<p:cSld").unwrap();
        let end = s.find("</p:cSld>").unwrap() + "</p:cSld>".len();
        let mut s = s;
        let master = placeholder(receiver(242, [0, 0, 800000, 800000], "", "<a:noFill/>"));
        s.replace_range(start..end, &format!("<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{master}</p:spTree></p:cSld>"));
        s
    });
    let explicit = rewrite(&inherited, SLIDE, |s| {
        s.replace(
            "<a:ln><a:noFill/></a:ln>",
            &format!("{image}<a:ln><a:noFill/></a:ln>"),
        )
    });
    cases.push(GroupImageCase {
        name: "layout-placeholder",
        inherited,
        explicit,
        expected_image_uses: 1,
    });
    let rotated = receiver(
        42,
        [40000, 80000, 600000, 400000],
        "rot=\"5400000\"",
        INHERIT,
    ) + &receiver(
        43,
        [840000, 200000, 400000, 600000],
        "rot=\"16200000\" flipV=\"1\"",
        INHERIT,
    );
    cases.push(paired(
        "receiver-rotation",
        &group(90, &frame, &image, &rotated),
        &image,
        2,
    ));
    cases.push(paired(
        "picture-dual-fill",
        &group(
            90,
            &frame,
            &image,
            &picture(42, "", &blip("owned-copy", "", STRETCH), INHERIT),
        ),
        &image,
        2,
    ));
    cases
}
