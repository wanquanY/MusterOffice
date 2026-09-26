use mo_common::*;
use mo_presentation_model::*;

fn document() -> Document {
    let mut d: Document = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap();
    let theme = ThemeId::new("theme:1").unwrap();
    let master = MasterId::new("master:1").unwrap();
    let layout = LayoutId::new("layout:1").unwrap();
    let font = FontId::new("font:1").unwrap();
    let resource = ResourceId::new("resource:font").unwrap();
    d.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            kind: ResourceKind::Font,
            sha256: Digest::from_sha256([1; 32]),
            media_type: "font/ttf".into(),
        },
    );
    d.fonts.insert(
        font.clone(),
        FontFace {
            id: font.clone(),
            resource,
            face_index: 0,
            family: "Fixture".into(),
            weight: 400,
            italic: false,
        },
    );
    d.themes.insert(
        theme.clone(),
        Theme {
            id: theme.clone(),
            name: "Theme".into(),
            colors: Default::default(),
            default_text: CharacterStyle {
                font: Inherited::Value(font),
                ..Default::default()
            },
        },
    );
    d.masters.insert(
        master.clone(),
        Master {
            id: master.clone(),
            theme,
            objects: vec![],
            background: Inherited::Inherit,
            default_text: Default::default(),
        },
    );
    d.layouts.insert(
        layout.clone(),
        Layout {
            id: layout.clone(),
            master,
            name: "Layout".into(),
            objects: vec![],
            background: Inherited::Inherit,
            default_text: Default::default(),
        },
    );
    d.slides.values_mut().next().unwrap().layout = Some(layout);
    let id = SlideId::new("slide:2").unwrap();
    d.slides.insert(
        id.clone(),
        Slide {
            id: id.clone(),
            name: "Independent".into(),
            layout: None,
            objects: vec![],
            background: Inherited::Inherit,
            hidden: false,
        },
    );
    d.slide_order.push(id);
    assert!(validate(&d, Default::default()).is_valid());
    d
}
fn keys(d: &Document) -> Vec<Digest> {
    assert!(validate(d, Default::default()).is_valid());
    d.slide_order
        .iter()
        .map(|id| PageDependencies::new(d, id).unwrap().digest().unwrap())
        .collect()
}
#[test]
fn inherited_styles_and_resource_content_invalidate_only_dependent_pages() {
    let original = document();
    let before = keys(&original);
    let mutations: [fn(&mut Document); 5] = [
        |d| {
            d.themes
                .values_mut()
                .next()
                .unwrap()
                .colors
                .insert(
                    ThemeColor::Accent1,
                    Rgba {
                        red: 1,
                        green: 2,
                        blue: 3,
                        alpha: 255,
                    },
                )
                .map(|_| ())
                .unwrap_or(())
        },
        |d| d.masters.values_mut().next().unwrap().default_text.bold = Inherited::Value(true),
        |d| {
            d.layouts.values_mut().next().unwrap().default_text.size =
                Inherited::Value(Emu::new(12700))
        },
        |d| d.fonts.values_mut().next().unwrap().face_index = 1,
        |d| d.resources.values_mut().next().unwrap().sha256 = Digest::from_sha256([2; 32]),
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        let after = keys(&changed);
        assert_ne!(before[0], after[0]);
        assert_eq!(before[1], after[1]);
    }
    let mut labels = original.clone();
    labels.title = "new".into();
    labels.themes.values_mut().next().unwrap().name = "new".into();
    labels.layouts.values_mut().next().unwrap().name = "new".into();
    labels.slides.values_mut().next().unwrap().name = "new".into();
    assert_eq!(before, keys(&labels));
}
#[test]
fn unknown_source_dependencies_are_conservative_and_missing_references_fail() {
    let mut d = document();
    let before = keys(&d);
    let source = ResourceId::new("source:1").unwrap();
    d.resources.insert(
        source.clone(),
        Resource {
            id: source,
            kind: ResourceKind::SourcePackage,
            sha256: Digest::from_sha256([3; 32]),
            media_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                .into(),
        },
    );
    let after = keys(&d);
    assert!(before.iter().zip(after).all(|(a, b)| a != &b));
    d.fonts.clear();
    assert!(PageDependencies::new(&d, &d.slide_order[0]).is_none());
}
