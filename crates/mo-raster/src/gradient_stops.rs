//! Shared evaluated stops. Geometry rebasing must not copy the color ramp.
use crate::GradientStop;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{borrow::Cow, ops::Deref, sync::Arc};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GradientStops(Arc<[GradientStop]>);
impl From<Vec<GradientStop>> for GradientStops {
    fn from(value: Vec<GradientStop>) -> Self {
        Self(value.into())
    }
}
impl Deref for GradientStops {
    type Target = [GradientStop];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<[GradientStop]> for GradientStops {
    fn as_ref(&self) -> &[GradientStop] {
        self
    }
}
impl<'a> IntoIterator for &'a GradientStops {
    type Item = &'a GradientStop;
    type IntoIter = std::slice::Iter<'a, GradientStop>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl GradientStops {
    /// Explicit copy-on-write for a caller changing an evaluated value. Shared
    /// paints remain unchanged; merely cloning or rebasing never calls this.
    pub fn make_mut(&mut self) -> &mut [GradientStop] {
        Arc::make_mut(&mut self.0)
    }
}
impl Serialize for GradientStops {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_ref().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for GradientStops {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<GradientStop>::deserialize(deserializer).map(Self::from)
    }
}
// Ownership is an in-process detail. Keep the exact existing array schema and
// schema identity, without enabling Serde-wide reference-counted serialization.
impl JsonSchema for GradientStops {
    fn inline_schema() -> bool {
        Vec::<GradientStop>::inline_schema()
    }
    fn schema_name() -> Cow<'static, str> {
        Vec::<GradientStop>::schema_name()
    }
    fn schema_id() -> Cow<'static, str> {
        Vec::<GradientStop>::schema_id()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Vec::<GradientStop>::json_schema(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient_tests::{gradient, request};
    #[test]
    fn cloning_and_rebasing_share_stops_but_edits_are_isolated() {
        let mut q = request();
        let g = gradient(&mut q);
        g.stops = vec![g.stops[0].clone(); 4096].into();
        let pointer = g.stops.as_ptr();
        let mut copies: Vec<_> = (0..64)
            .map(|_| q.draws[0].brush.rebased(q.viewport.origin).unwrap())
            .collect();
        for brush in &copies {
            let crate::Brush::Gradient { gradient } = brush else {
                panic!()
            };
            assert_eq!(pointer, gradient.stops.as_ptr());
        }
        let crate::Brush::Gradient { gradient: g } = &mut copies[0] else {
            panic!()
        };
        g.stops.make_mut()[0].srgb = [0.0; 4];
        assert_ne!(pointer, g.stops.as_ptr());
        assert_eq!(gradient(&mut q).stops[0].srgb, [1.0, 0.0, 0.0, 1.0]);
        let detached = g.stops.as_ptr();
        g.stops.make_mut()[1].srgb = [0.5; 4];
        assert_eq!(detached, g.stops.as_ptr());
    }
    #[test]
    fn shared_stop_ownership_preserves_array_wire_and_schema() {
        let q = request();
        let crate::Brush::Gradient { gradient: g } = &q.draws[0].brush else {
            panic!()
        };
        let bytes = serde_json::to_vec(&g.stops).unwrap();
        assert_eq!(bytes, serde_json::to_vec(g.stops.as_ref()).unwrap());
        let roundtrip: GradientStops = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(roundtrip, g.stops);
        assert_ne!(roundtrip.as_ptr(), g.stops.as_ptr());
        assert_eq!(
            schemars::schema_for!(GradientStops),
            schemars::schema_for!(Vec<crate::GradientStop>)
        );
    }
}
