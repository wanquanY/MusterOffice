//! Original bounded multiplication probes, with an image preceding the gradient.
#[allow(dead_code)]
#[path = "source_gradient_page.rs"]
mod gradient;
pub use gradient::*;
pub fn source(paths: usize, first_unpainted: bool, background_gradient: bool) -> Vec<u8> {
    let stops = (0..4096)
        .map(|i| {
            format!(
                "<a:gs pos=\"{}\"><a:srgbClr val=\"{:02X}0000\"/></a:gs>",
                i * 100000 / 4095,
                i % 256
            )
        })
        .collect::<String>();
    let fill = linear(0, false, "", "", &stops);
    let mut geometry = String::from(
        "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"w\" b=\"h\"/><a:pathLst>",
    );
    for i in 0..paths {
        geometry.push_str(&format!("<a:path w=\"1200000\" h=\"600000\" fill=\"{}\" stroke=\"0\"><a:moveTo><a:pt x=\"0\" y=\"0\"/></a:moveTo><a:lnTo><a:pt x=\"1200000\" y=\"0\"/></a:lnTo><a:lnTo><a:pt x=\"1200000\" y=\"600000\"/></a:lnTo><a:lnTo><a:pt x=\"0\" y=\"600000\"/></a:lnTo><a:close/></a:path>",if first_unpainted&&i==0 {"none"}else{"norm"}));
    }
    geometry.push_str("</a:pathLst></a:custGeom>");
    let shape = receiver(42, [100000, 200000, 1200000, 600000], "", &fill)
        .replace("<a:prstGeom prst=\"rect\"/>", &geometry);
    let bytes = image_fixture(
        &(picture(41, "", &blip("owned-image", "", STRETCH), "<a:noFill/>") + &shape),
    );
    if background_gradient {
        rewrite(&bytes, SLIDE, |s| {
            s.replace(
                &format!("<p:bgPr>{}</p:bgPr>", solid("FFFFFF")),
                &format!("<p:bgPr>{}</p:bgPr>", linear(0, false, "", "", STOPS)),
            )
        })
    } else {
        bytes
    }
}
