//! An immutable object directory stores positions, never self-references.
use super::*;
#[derive(Default)]
pub(super) struct ObjectDirectory {
    surfaces: BTreeMap<String, BTreeMap<u32, usize>>,
    count: usize,
}
#[derive(Clone)]
pub(in crate::source) struct ObjectBindings<'a> {
    index: &'a SourceIndex,
    pub(super) directory: Arc<ObjectDirectory>,
}
impl<'a> ObjectBindings<'a> {
    pub(in crate::source) fn new(
        index: &'a SourceIndex,
        step: &mut dyn FnMut() -> Result<(), PptxError>,
    ) -> Result<Self, PptxError> {
        let mut directory = ObjectDirectory::default();
        for (part, surface) in &index.surfaces {
            let mut objects = BTreeMap::new();
            for (position, object) in surface.objects.iter().enumerate() {
                step()?;
                if objects.insert(object.native_id, position).is_some() {
                    return Err(conflict());
                }
                directory.count += 1;
            }
            if !objects.is_empty() {
                directory.surfaces.insert(part.clone(), objects);
            }
        }
        Ok(Self {
            index,
            directory: Arc::new(directory),
        })
    }
    pub(super) fn retained(index: &'a SourceIndex, directory: Arc<ObjectDirectory>) -> Self {
        Self { index, directory }
    }
    pub(in crate::source) fn len(&self) -> usize {
        self.directory.count
    }
    pub(in crate::source) fn get(&self, key: &(&str, u32)) -> Option<&'a SourceObject> {
        self.get_key_value(key).map(|(_, object)| object)
    }
    pub(in crate::source) fn contains_key(&self, key: &(&str, u32)) -> bool {
        self.get(key).is_some()
    }
    pub(in crate::source) fn get_key_value(
        &self,
        key: &(&str, u32),
    ) -> Option<((&'a str, u32), &'a SourceObject)> {
        let position = self.directory.surfaces.get(key.0)?.get(&key.1)?;
        let (part, surface) = self.index.surfaces.get_key_value(key.0)?;
        let object = surface.objects.get(*position)?;
        (object.native_id == key.1).then_some(((part.as_str(), key.1), object))
    }
}
