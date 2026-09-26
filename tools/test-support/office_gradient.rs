//! Owned editable source fixtures for the exact Office ramp eligibility rules.
#[allow(dead_code)]
#[path = "source_gradient_page.rs"]
mod gradient_fixture;
pub use gradient_fixture::*;
pub fn office_cases() -> Vec<(String, Vec<u8>, bool)> {
    let stop = |position: &str, color: &str, alpha: &str| {
        format!("<a:gs pos=\"{position}\"><a:srgbClr val=\"{color}\">{alpha}</a:srgbClr></a:gs>")
    };
    let a = stop("0", "19CC66", "");
    let b = stop("100000", "E63399", "");
    let middle = stop("37500", "E63399", "");
    let last = stop("100000", "19CC66", "");
    [
        ("pair", a.clone() + &b, true),
        ("pair-reversed", stop("0", "E63399", "") + &last, true),
        (
            "pair-alpha",
            stop("0", "19CC66", "<a:alpha val=\"0\"/>") + &b,
            true,
        ),
        ("symmetric-three", a.clone() + &middle + &last, true),
        (
            "symmetric-alpha",
            stop("0", "19CC66", "<a:alpha val=\"25000\"/>")
                + &middle
                + &stop("100000", "19CC66", "<a:alpha val=\"25000\"/>"),
            true,
        ),
        (
            "shifted-end",
            a.clone() + &stop("99000", "E63399", ""),
            false,
        ),
        (
            "asymmetric-three",
            a.clone() + &middle + &stop("100000", "19CC67", ""),
            false,
        ),
        (
            "unequal-alpha",
            a.clone() + &middle + &stop("100000", "19CC66", "<a:alpha val=\"25000\"/>"),
            false,
        ),
        (
            "four-stops",
            a.clone() + &middle + &stop("70000", "CC3399", "") + &last,
            false,
        ),
        (
            "duplicate-middle",
            a + &stop("0", "E63399", "") + &last,
            false,
        ),
    ]
    .into_iter()
    .map(|(name, stops, office)| {
        let fill = linear(0, false, "", "", &stops);
        (
            name.into(),
            image_fixture(&receiver(42, [100000, 200000, 1200000, 600000], "", &fill)),
            office,
        )
    })
    .collect()
}
