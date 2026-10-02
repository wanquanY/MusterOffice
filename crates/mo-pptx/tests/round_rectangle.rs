mod support;
use mo_common::Emu;
use mo_pptx::{
    source::{
        geometry::{SourceGeometryDefinition, evaluate::*},
        inspect_source,
    },
    *,
};
use mo_presentation_model::{Geometry, ObjectContent};

#[test]
fn arbitrary_emu_radius_stays_an_editable_native_round_rectangle_after_export() {
    for radius in [0, 1, 123457, 450001] {
        let (mut document, defaults) = support::input();
        for object in document.objects.values_mut() {
            if let ObjectContent::Shape { geometry, .. } = &mut object.content {
                let transform = object.transform.as_mut().unwrap();
                transform.size.width = Emu::new(4000003);
                transform.size.height = Emu::new(900003);
                *geometry = Geometry::RoundRectangle {
                    radius: Emu::new(radius),
                };
            }
        }
        let plan = AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
        let output = export_plan_to(
            &plan,
            &support::resources(),
            Vec::new(),
            PptxLimits::default().package,
            &|| false,
        )
        .unwrap();
        let readback = inspect_source(output.package(), Default::default(), &|| false).unwrap();
        let mut checked = 0;
        for (part, surface) in &readback.surfaces {
            let objects: Vec<_> = surface.objects.iter().filter(|o| matches!(o.geometry.as_ref().map(|g| &g.definition), Some(SourceGeometryDefinition::Preset { preset, .. }) if preset.name() == "roundRect")).map(|o| o.native_id).collect();
            let values = query(
                &readback,
                &SourceGeometryQuery {
                    expected_source_sha256: readback.source_sha256.clone(),
                    surface: part.clone(),
                    objects,
                    profile: GeometryProfile::Drawingml2016PresetsDraftV2,
                },
                Default::default(),
                &|| false,
            )
            .unwrap();
            for object in values.objects {
                let GeometryOutcome::Resolved { geometry } = object.outcome else {
                    panic!("native geometry must resolve")
                };
                let x1 = geometry
                    .guides
                    .iter()
                    .find(|g| g.name == "x1")
                    .unwrap()
                    .value;
                assert!(
                    (x1 - radius as f64).abs() < 0.000001,
                    "radius changed: {x1} != {radius}"
                );
                checked += 1;
            }
        }
        assert!(checked > 0);
    }
}
