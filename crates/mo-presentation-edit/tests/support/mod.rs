use mo_common::*;
use mo_presentation_model::*;

pub fn id() -> ObjectId {
    ObjectId::new("shape:1").unwrap()
}
pub fn slide_id() -> SlideId {
    SlideId::new("slide:1").unwrap()
}
pub fn paragraph_id() -> ParagraphId {
    ParagraphId::new("paragraph:1").unwrap()
}
pub fn run_id() -> RunId {
    RunId::new("run:1").unwrap()
}

pub fn document() -> Document {
    let mut document = Document::empty(
        DocumentId::new("document:1").unwrap(),
        Size {
            width: Emu::new(12_192_000),
            height: Emu::new(6_858_000),
        },
    );
    document.slide_order.push(slide_id());
    document.slides.insert(
        slide_id(),
        Slide {
            id: slide_id(),
            name: "Slide 1".into(),
            layout: None,
            objects: vec![id()],
            background: Inherited::Value(Fill::None),
            hidden: false,
        },
    );
    document.objects.insert(
        id(),
        Object {
            id: id(),
            parent: ContainerId::Slide(slide_id()),
            transform: Transform {
                origin: Point {
                    x: Emu::ZERO,
                    y: Emu::ZERO,
                },
                size: document.page_size,
                rotation: 0,
                flip_horizontal: false,
                flip_vertical: false,
            },
            appearance: Appearance::default(),
            accessibility: Accessibility::default(),
            content: ObjectContent::Shape {
                geometry: Geometry::Rectangle,
                text: Some(TextBody {
                    paragraphs: vec![Paragraph {
                        id: paragraph_id(),
                        style: ParagraphStyle::default(),
                        default_run_style: CharacterStyle::default(),
                        runs: vec![TextRun {
                            id: run_id(),
                            style: CharacterStyle::default(),
                            content: InlineContent::Text {
                                text: "A😀e\u{301}中".into(),
                            },
                        }],
                    }],
                    style: CharacterStyle::default(),
                    insets: Insets {
                        left: Emu::ZERO,
                        top: Emu::ZERO,
                        right: Emu::ZERO,
                        bottom: Emu::ZERO,
                    },
                    wrap: true,
                    overflow: OverflowPolicy::Report,
                }),
            },
        },
    );
    document
}

pub fn connector(document: &mut Document) -> ObjectId {
    let connector = ObjectId::new("connector:1").unwrap();
    let mut object = document.objects[&id()].clone();
    object.id = connector.clone();
    object.content = ObjectContent::Connector {
        start: ConnectorEndpoint::Attached {
            object: id(),
            site: 0,
        },
        end: ConnectorEndpoint::Free {
            position: Point {
                x: Emu::new(50),
                y: Emu::new(50),
            },
        },
    };
    document.objects.insert(connector.clone(), object);
    document
        .slides
        .get_mut(&slide_id())
        .unwrap()
        .objects
        .push(connector.clone());
    connector
}
