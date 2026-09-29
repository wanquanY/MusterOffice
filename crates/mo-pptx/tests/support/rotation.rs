use mo_timeline::{ExactValue, FrameState, RotationBasis};

// These native roundtrip fixtures target objects with zero local layout rotation.
// Compare the resolved value domain while checking that its basis stays explicit.
pub fn unrotated_values(frame: &FrameState, basis: RotationBasis) -> Vec<ExactValue> {
    frame
        .rotations
        .values()
        .map(|r| {
            assert_eq!(r.basis, basis);
            r.value()
        })
        .collect()
}
