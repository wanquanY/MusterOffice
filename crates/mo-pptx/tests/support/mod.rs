use mo_common::ResourceId;
use mo_pptx::*;
use mo_presentation_model::*;

pub fn input() -> (Document, ExportDefaults) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    (
        serde_json::from_value(value["document"].clone()).unwrap(),
        serde_json::from_value(value["defaults"].clone()).unwrap(),
    )
}
pub struct Fixture(pub &'static [u8]);
impl Resources for Fixture {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        if id.as_str() != "resource:checker" {
            return Err(PptxError::ResourceRequired(id.clone()));
        }
        Ok(ResourceData {
            reader: &self.0,
            byte_length: self.0.len() as u64,
        })
    }
}
pub fn resources() -> Fixture {
    Fixture(include_bytes!(
        "../../../../fixtures/presentations/native-export/resources.bin"
    ))
}
