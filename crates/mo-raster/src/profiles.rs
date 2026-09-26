//! One registry for emitted raster metadata and host publication validation.
//! This does not substitute for a backend's optional ABI capability checks.
use crate::*;
#[derive(Clone, Copy)]
enum Resources {
    None,
    Images,
    Either,
}
const PROFILES: &[(u32, &str, Resources)] = &[
    (4, PROFILE, Resources::None),
    (5, IMAGE_PROFILE, Resources::Images),
    (6, IMAGE_DOMAIN_PROFILE, Resources::Images),
    (7, CLIP_PROFILE, Resources::Either),
    (8, COMPOSITE_PROFILE, Resources::Either),
    (9, GRADIENT_PLANE_PROFILE, Resources::Either),
    (10, OFFICE_GRADIENT_PROFILE, Resources::Either),
    (11, RECT_GRADIENT_PROFILE, Resources::Either),
    (12, ELLIPTIC_GRADIENT_PROFILE, Resources::Either),
];
/// Known wire version to metadata profile. Unknown versions fail closed.
pub fn profile_for_frame(version: u32) -> Option<&'static str> {
    PROFILES.iter().find(|p| p.0 == version).map(|p| p.1)
}
/// Validate a metadata profile against the resource mode used for the request.
pub fn accepts_profile(profile: &str, with_images: bool) -> bool {
    PROFILES.iter().any(|p| {
        p.1 == profile
            && match p.2 {
                Resources::None => !with_images,
                Resources::Images => with_images,
                Resources::Either => true,
            }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_modes_share_the_emitted_profile_registry() {
        for version in 0..=13 {
            let p = profile_for_frame(version);
            assert_eq!(p.is_some(), (4..=12).contains(&version));
            if let Some(p) = p {
                assert_eq!(accepts_profile(p, false), version == 4 || version >= 7);
                assert_eq!(accepts_profile(p, true), version >= 5);
            }
        }
        assert!(!accepts_profile("unknown", false));
        assert!(!accepts_profile("unknown", true));
    }
}
