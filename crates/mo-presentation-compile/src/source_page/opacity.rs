//! Resolve opacity ancestry against the immutable source object hierarchy.
use super::*;
use std::collections::BTreeMap;
pub(super) type Key = (String, u32);
pub(super) type Chain = Vec<(Key, u16)>;
pub(super) fn chains(
    index: &SourceIndex,
    layers: &[SourcePageLayer],
    properties: Option<&SourceProperties>,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<Key, Chain>, SourcePageError> {
    let Some(properties) = properties.filter(|p| p.values().any(|v| v.opacity.is_some())) else {
        return Ok(BTreeMap::new());
    };
    let mut result = BTreeMap::new();
    for layer in layers {
        cancel(check)?;
        let parents: BTreeMap<_, _> = index.surfaces[&layer.part]
            .objects
            .iter()
            .map(|o| (o.native_id, o.parent_group))
            .collect();
        for &object in &layer.objects {
            cancel(check)?;
            let mut chain = vec![];
            let mut current = Some(object);
            let mut depth = 0;
            while let Some(id) = current {
                depth += 1;
                if depth > 64 {
                    return Err(RasterError::Limit("source opacity ancestry").into());
                }
                let key = (layer.part.clone(), id);
                if let Some(value) = properties.get(&key).and_then(|p| p.opacity) {
                    chain.push((key, value));
                }
                current = *parents
                    .get(&id)
                    .ok_or(SourcePageError::Invalid("source opacity parent"))?;
            }
            if !chain.is_empty() {
                chain.reverse();
                result.insert((layer.part.clone(), object), chain);
            }
        }
    }
    Ok(result)
}
