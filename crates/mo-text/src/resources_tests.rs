use super::*;
use crate::{
    cascade::{CascadeRequest, context::Context},
    geometry::test_support::{FONT, request},
};
use mo_common::{ByteLength, Digest};

#[test]
fn repeated_contexts_share_verified_faces_and_keep_text_state_separate() {
    let mut q = request("A").shaping.paragraph;
    q.fonts.push(q.fonts[0].clone());
    let resources =
        FontResources::load(&q.fonts, FONT, CascadeLimits::default(), &|| false).unwrap();
    let clone = resources.clone();
    assert!(Arc::ptr_eq(&resources.data, &clone.data));
    assert_eq!(resources.verified_faces(), 1);
    assert_eq!(resources.bindings(), &[0, 0]);
    let a = CascadeRequest {
        text: "A".into(),
        fonts: q.fonts.clone(),
        items: vec![],
    };
    let b = CascadeRequest {
        text: "A A".into(),
        fonts: q.fonts,
        items: vec![],
    };
    for (q, count) in [(&a, 1), (&b, 3)] {
        let c = Context::prepare_using(
            q,
            ResourceInput::Prepared(&resources),
            CascadeLimits::default(),
            &|| false,
            |_, _| Ok(()),
        )
        .unwrap();
        assert!(std::ptr::eq(&c.fonts()[0], &resources.fonts()[0]));
        assert_eq!(c.scalars.len(), count);
        assert_eq!(c.shaping_runs, 0);
        assert_eq!(c.probed_glyphs, 0);
    }
}

#[test]
fn a_prepared_handle_cannot_be_rebound_to_stale_or_different_descriptors() {
    let sources = request("A").shaping.paragraph.fonts;
    let resources =
        FontResources::load(&sources, FONT, CascadeLimits::default(), &|| false).unwrap();
    for field in 0..5 {
        let mut altered = sources.clone();
        match field {
            0 => altered[0].expected_sha256 = Digest::from_sha256([0; 32]),
            1 => altered[0].face_index = 1,
            2 => altered[0].offset = ByteLength::new(1),
            3 => altered[0].byte_length = ByteLength::new(1),
            _ => altered.clear(),
        }
        assert!(matches!(
            ResourceInput::Prepared(&resources).load(&altered, CascadeLimits::default(), &|| false),
            Err(TextError::Font(FontError::ResourceConflict))
        ));
    }
    let mut aliases = sources.clone();
    aliases.push(sources[0].clone());
    aliases[1].expected_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        FontResources::load(&aliases, FONT, CascadeLimits::default(), &|| false),
        Err(TextError::Font(FontError::ResourceConflict))
    ));
}

#[test]
fn reused_resources_still_obey_current_limits_and_cancellation() {
    let sources = request("A").shaping.paragraph.fonts;
    let resources =
        FontResources::load(&sources, FONT, CascadeLimits::default(), &|| false).unwrap();
    for limits in [
        CascadeLimits {
            max_bundle_bytes: 10,
            ..Default::default()
        },
        CascadeLimits {
            max_font_bindings: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            ResourceInput::Prepared(&resources).load(&sources, limits, &|| false),
            Err(TextError::Limit(_))
        ));
    }
    assert!(matches!(
        ResourceInput::Prepared(&resources).load(&sources, CascadeLimits::default(), &|| true),
        Err(TextError::Cancelled)
    ));
    // Handle remains reusable after a caller's failed or cancelled operation.
    assert!(
        ResourceInput::Prepared(&resources)
            .load(&sources, CascadeLimits::default(), &|| false)
            .is_ok()
    );
}
