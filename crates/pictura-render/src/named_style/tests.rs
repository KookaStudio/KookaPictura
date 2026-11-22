use super::*;
use pictura_core::{BitDepth, CharacterOverrides, ColorMode, ParagraphOverrides, TypeSpec};

fn spec(text: &str) -> TypeSpec {
    let mut spec = TypeSpec::new(text, "Liberation Sans", 20.0);
    spec.origin = (40.0, 40.0);
    spec
}

fn paragraph(size: f64) -> NamedStyleAttrs {
    NamedStyleAttrs::Paragraph {
        character: CharacterOverrides {
            size: Some(size),
            ..Default::default()
        },
        paragraph: ParagraphOverrides::default(),
    }
}

fn character(size: f64) -> NamedStyleAttrs {
    NamedStyleAttrs::Character(CharacterOverrides {
        size: Some(size),
        ..Default::default()
    })
}

fn layer_spec(doc: &Document, path: &str) -> TypeSpec {
    crate::resolve_path(doc, path)
        .and_then(crate::type_layer_spec)
        .expect("a type layer")
}

fn layer_channels(doc: &Document, path: &str) -> Vec<pictura_core::Plane<u8>> {
    crate::resolve_path(doc, path)
        .expect("a layer")
        .channels()
        .iter()
        .map(|channel| channel.data.clone())
        .collect()
}

#[test]
fn renaming_an_applied_paragraph_style_keeps_nested_applications() {
    let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
    let group = crate::add_group(&mut doc, -1, "Group").to_string();
    let nested = crate::add_type_layer(&mut doc, &group, &spec("Nested"));
    let top = crate::add_type_layer(&mut doc, "", &spec("Top"));
    assert!(nested.contains('/'), "a nested type layer: {nested}");
    doc.text_styles
        .create_paragraph_style(
            "Body",
            CharacterOverrides {
                size: Some(22.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    assert!(crate::apply_type_style(&mut doc, &nested, "Body", true));
    assert!(crate::apply_type_style(&mut doc, &top, "Body", true));
    let nested_channels = layer_channels(&doc, &nested);
    let top_channels = layer_channels(&doc, &top);

    edit_named_style(
        &mut doc,
        NamedStyleEdit {
            from: "Body",
            to: "Body 2",
            attrs: paragraph(30.0),
        },
    )
    .unwrap();

    assert!(doc.text_styles.paragraph_style("Body").is_none());
    assert!(doc.text_styles.paragraph_style("Body 2").is_some());
    for path in [nested.as_str(), top.as_str()] {
        let spec = layer_spec(&doc, path);
        assert_eq!(spec.applied_paragraph_style.as_deref(), Some("Body 2"));
        assert_eq!(spec.character.size, 30.0);
    }
    assert_ne!(
        layer_channels(&doc, &nested),
        nested_channels,
        "the nested layer's rendered content changed"
    );
    assert_ne!(
        layer_channels(&doc, &top),
        top_channels,
        "the top layer's rendered content changed"
    );

    edit_named_style(
        &mut doc,
        NamedStyleEdit {
            from: "Body 2",
            to: "Body 2",
            attrs: paragraph(40.0),
        },
    )
    .unwrap();
    for path in [nested.as_str(), top.as_str()] {
        let spec = layer_spec(&doc, path);
        assert_eq!(spec.applied_paragraph_style.as_deref(), Some("Body 2"));
        assert_eq!(spec.character.size, 40.0, "a later edit still reaches it");
    }
}

#[test]
fn editing_basic_paragraph_reaches_the_layers_that_apply_a_style() {
    let mut doc = Document::new(200, 120, ColorMode::Rgb, BitDepth::Eight);
    let path = crate::add_type_layer(&mut doc, "", &spec("Aa"));
    doc.text_styles
        .create_paragraph_style(
            "Body",
            CharacterOverrides {
                size: Some(30.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    assert!(crate::apply_type_style(&mut doc, &path, "Body", true));
    let before = layer_spec(&doc, &path);
    assert_eq!(before.character.size, 30.0);
    let channels = layer_channels(&doc, &path);

    edit_named_style(
        &mut doc,
        NamedStyleEdit {
            from: pictura_core::BASIC_PARAGRAPH,
            to: pictura_core::BASIC_PARAGRAPH,
            attrs: NamedStyleAttrs::Paragraph {
                character: CharacterOverrides {
                    tracking: Some(120.0),
                    ..Default::default()
                },
                paragraph: ParagraphOverrides::default(),
            },
        },
    )
    .unwrap();

    let after = layer_spec(&doc, &path);
    assert_eq!(after.applied_paragraph_style.as_deref(), Some("Body"));
    assert_eq!(after.character.size, 30.0, "the applied style still wins");
    assert_eq!(after.character.tracking, 120.0, "the default reaches it");
    assert_ne!(
        layer_channels(&doc, &path),
        channels,
        "the rendered content changed"
    );
}

#[test]
fn a_duplicate_target_name_is_refused() {
    let mut doc = Document::new(200, 120, ColorMode::Rgb, BitDepth::Eight);
    let path = crate::add_type_layer(&mut doc, "", &spec("Aa"));
    doc.text_styles
        .create_paragraph_style(
            "Body",
            CharacterOverrides {
                size: Some(30.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    doc.text_styles
        .create_paragraph_style(
            "Other",
            CharacterOverrides {
                size: Some(10.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    assert!(crate::apply_type_style(&mut doc, &path, "Body", true));

    let before = layer_spec(&doc, &path);
    let before_body = doc.text_styles.paragraph_style("Body").unwrap().clone();
    let before_other = doc.text_styles.paragraph_style("Other").unwrap().clone();
    let before_channels = layer_channels(&doc, &path);
    let before_composite = doc.composite.data.clone();

    assert_eq!(
        edit_named_style(
            &mut doc,
            NamedStyleEdit {
                from: "Body",
                to: "Other",
                attrs: paragraph(50.0),
            },
        ),
        Err(StyleError::Duplicate("Other".to_string()))
    );

    assert_eq!(doc.text_styles.paragraph_style("Body"), Some(&before_body));
    assert_eq!(
        doc.text_styles.paragraph_style("Other"),
        Some(&before_other)
    );
    let after = layer_spec(&doc, &path);
    assert_eq!(after.applied_paragraph_style.as_deref(), Some("Body"));
    assert_eq!(after.character.size, before.character.size);
    assert_eq!(layer_channels(&doc, &path), before_channels);
    assert_eq!(doc.composite.data, before_composite);
}

#[test]
fn a_missing_source_style_is_refused() {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    let before = doc.text_styles.clone();
    assert_eq!(
        edit_named_style(
            &mut doc,
            NamedStyleEdit {
                from: "Ghost",
                to: "Body",
                attrs: paragraph(20.0),
            },
        ),
        Err(StyleError::NotFound("Ghost".to_string()))
    );
    assert_eq!(doc.text_styles, before);
}

#[test]
fn a_character_style_rename_keeps_applied_references() {
    let mut doc = Document::new(200, 120, ColorMode::Rgb, BitDepth::Eight);
    let path = crate::add_type_layer(&mut doc, "", &spec("Aa"));
    doc.text_styles
        .create_character_style(
            "Quote",
            CharacterOverrides {
                size: Some(40.0),
                ..Default::default()
            },
        )
        .unwrap();
    doc.text_styles
        .create_character_style(
            "Caption",
            CharacterOverrides {
                size: Some(12.0),
                ..Default::default()
            },
        )
        .unwrap();
    doc.layers[0].type_overrides = pictura_core::StyleOverrides {
        character: pictura_core::CharacterOverrides {
            tracking: Some(120.0),
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(crate::apply_type_style(&mut doc, &path, "Quote", false));
    let channels = layer_channels(&doc, &path);

    edit_named_style(
        &mut doc,
        NamedStyleEdit {
            from: "Quote",
            to: "Quote 2",
            attrs: character(60.0),
        },
    )
    .unwrap();
    assert!(doc.text_styles.character_style("Quote").is_none());
    let spec = layer_spec(&doc, &path);
    assert_eq!(spec.applied_character_style.as_deref(), Some("Quote 2"));
    assert_eq!(spec.character.size, 60.0);
    assert_eq!(spec.overrides.character.tracking, Some(120.0));
    assert_eq!(
        spec.character.tracking, 120.0,
        "the manual override still wins"
    );
    assert_eq!(
        doc.layers[0].type_overrides.character.tracking,
        Some(120.0),
        "the layer keeps its manual override"
    );
    assert_ne!(
        layer_channels(&doc, &path),
        channels,
        "the rendered content changed"
    );

    assert_eq!(
        edit_named_style(
            &mut doc,
            NamedStyleEdit {
                from: "Quote 2",
                to: "Caption",
                attrs: character(10.0),
            },
        ),
        Err(StyleError::Duplicate("Caption".to_string()))
    );
    assert_eq!(
        edit_named_style(
            &mut doc,
            NamedStyleEdit {
                from: "Ghost",
                to: "Ghost",
                attrs: character(10.0),
            },
        ),
        Err(StyleError::NotFound("Ghost".to_string()))
    );
    assert_eq!(layer_spec(&doc, &path).character.size, 60.0);
}

#[test]
fn renaming_in_a_non_editable_document_keeps_the_content() {
    let mut doc = Document::new(200, 120, ColorMode::Rgb, BitDepth::Eight);
    let path = crate::add_type_layer(&mut doc, "", &spec("Aa"));
    doc.text_styles
        .create_paragraph_style(
            "Body",
            CharacterOverrides {
                size: Some(30.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    assert!(crate::apply_type_style(&mut doc, &path, "Body", true));
    let channels = layer_channels(&doc, &path);
    let composite = doc.composite.data.clone();

    doc.depth = BitDepth::Sixteen;
    assert!(!crate::type_layers_editable(&doc));

    edit_named_style(
        &mut doc,
        NamedStyleEdit {
            from: "Body",
            to: "Body 2",
            attrs: paragraph(50.0),
        },
    )
    .unwrap();

    assert!(doc.text_styles.paragraph_style("Body 2").is_some());
    let spec = layer_spec(&doc, &path);
    assert_eq!(spec.applied_paragraph_style.as_deref(), Some("Body 2"));
    assert_eq!(spec.character.size, 30.0, "content left as it was");
    assert_eq!(layer_channels(&doc, &path), channels);
    assert_eq!(doc.composite.data, composite);
}
