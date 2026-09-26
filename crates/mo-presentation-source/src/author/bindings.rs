use crate::PptxError;
use mo_common::{LayoutId, MasterId, ObjectId, ResourceId, ThemeId};
use mo_opc::PartName;
use mo_presentation_model::{Document, ObjectContent};
use std::collections::{BTreeMap, BTreeSet};
pub struct MasterPlan {
    pub id: Option<MasterId>,
    pub part: PartName,
    pub theme: PartName,
}
pub struct LayoutPlan {
    pub id: Option<LayoutId>,
    pub part: PartName,
    pub master: PartName,
}
pub struct NativeBindings {
    pub themes: BTreeMap<Option<ThemeId>, PartName>,
    pub masters: Vec<MasterPlan>,
    pub layouts: Vec<LayoutPlan>,
    pub object_ids: BTreeMap<ObjectId, u32>,
    pub images: BTreeMap<ResourceId, PartName>,
}

fn part(path: impl Into<String>) -> Result<PartName, PptxError> {
    Ok(PartName::new(path)?)
}
pub(super) fn plan(document: &Document) -> Result<NativeBindings, PptxError> {
    let mut themes = BTreeMap::new();
    for (i, id) in std::iter::once(None)
        .chain(document.themes.keys().cloned().map(Some))
        .enumerate()
    {
        themes.insert(id, part(format!("/ppt/theme/theme{}.xml", i + 1))?);
    }
    let mut masters = vec![MasterPlan {
        id: None,
        part: part("/ppt/slideMasters/slideMaster1.xml")?,
        theme: themes[&None].clone(),
    }];
    for (i, (id, m)) in document.masters.iter().enumerate() {
        masters.push(MasterPlan {
            id: Some(id.clone()),
            part: part(format!("/ppt/slideMasters/slideMaster{}.xml", i + 2))?,
            theme: themes[&Some(m.theme.clone())].clone(),
        });
    }
    let mut layouts = vec![LayoutPlan {
        id: None,
        part: part("/ppt/slideLayouts/slideLayout1.xml")?,
        master: masters[0].part.clone(),
    }];
    for (i, (id, l)) in document.layouts.iter().enumerate() {
        layouts.push(LayoutPlan {
            id: Some(id.clone()),
            part: part(format!("/ppt/slideLayouts/slideLayout{}.xml", i + 2))?,
            master: masters
                .iter()
                .find(|m| m.id.as_ref() == Some(&l.master))
                .expect("validated master")
                .part
                .clone(),
        });
    }
    for master in &masters {
        if !layouts.iter().any(|l| l.master == master.part) {
            layouts.push(LayoutPlan {
                id: None,
                part: part(format!(
                    "/ppt/slideLayouts/slideLayout{}.xml",
                    layouts.len() + 1
                ))?,
                master: master.part.clone(),
            });
        }
    }
    let mut images = BTreeMap::new();
    let ids = document
        .objects
        .values()
        .filter_map(|o| match &o.content {
            ObjectContent::Picture { resource, .. } => Some(resource.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    for (i, id) in ids.into_iter().enumerate() {
        let extension = match document.resources[&id].media_type.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            _ => {
                return Err(PptxError::Unsupported(
                    "image format export beyond PNG/JPEG".into(),
                ));
            }
        };
        images.insert(
            id,
            part(format!("/ppt/media/image{}.{}", i + 1, extension))?,
        );
    }
    let object_ids = document
        .objects
        .keys()
        .enumerate()
        .map(|(i, id)| (id.clone(), i as u32 + 2))
        .collect();
    Ok(NativeBindings {
        themes,
        masters,
        layouts,
        object_ids,
        images,
    })
}
