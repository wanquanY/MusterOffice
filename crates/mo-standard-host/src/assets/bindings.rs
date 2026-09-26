use super::AssetReader;
use crate::StandardHost;
use mo_common::ResourceId;
use mo_operation_service::*;
use mo_presentation_model::Document;
use std::collections::BTreeMap;

/// Byte readers for one explicit document's complete declared resource set.
/// Asset handles are resolved in the caller's scope before the writer runs.
pub struct BoundResources<'a> {
    readers: BTreeMap<ResourceId, AssetReader<'a>>,
}
impl mo_pptx::Resources for BoundResources<'_> {
    fn open(&self, id: &ResourceId) -> Result<mo_pptx::ResourceData<'_>, mo_pptx::PptxError> {
        let reader = self
            .readers
            .get(id)
            .ok_or_else(|| mo_pptx::PptxError::ResourceRequired(id.clone()))?;
        Ok(mo_pptx::ResourceData {
            reader,
            byte_length: reader.info().descriptor.byte_length.get(),
        })
    }
}
impl StandardHost {
    pub fn bind_resources(
        &self,
        context: &CallContext,
        document: &Document,
        bindings: &[AssetBinding],
    ) -> Result<BoundResources<'_>, Failure> {
        context.require(Permission::ReadAssets)?;
        if bindings.len() != document.resources.len() {
            return Err(Failure::new(
                FailureCode::ResourceIncomplete,
                "document resource closure is incomplete",
            ));
        }
        let mut readers = BTreeMap::new();
        for binding in bindings {
            if readers.contains_key(&binding.resource_id) {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "duplicate document resource binding",
                ));
            }
            let resource = document
                .resources
                .get(&binding.resource_id)
                .ok_or_else(|| {
                    Failure::new(
                        FailureCode::ResourceConflict,
                        "binding is outside the document",
                    )
                })?;
            let reader = self.open_asset(context, &binding.asset_id)?;
            let descriptor = &reader.info().descriptor;
            if descriptor.sha256 != resource.sha256 || descriptor.media_type != resource.media_type
            {
                return Err(Failure::new(
                    FailureCode::ResourceConflict,
                    "resource content differs from the document declaration",
                ));
            }
            readers.insert(binding.resource_id.clone(), reader);
        }
        Ok(BoundResources { readers })
    }
}
