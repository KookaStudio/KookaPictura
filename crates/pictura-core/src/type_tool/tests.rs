use super::*;

#[test]
fn new_type_defaults_are_cs6() {
    let character = CharacterAttrs::default();
    assert_eq!(character.horizontal_scale, 100.0);
    assert_eq!(character.vertical_scale, 100.0);
    assert_eq!(character.kerning, KerningMode::Metrics);
    assert_eq!(character.anti_alias, AntiAlias::Sharp);
    assert_eq!(character.tracking, 0.0);
    assert_eq!(character.baseline_shift, 0.0);
    assert_eq!(character.tsume, 0.0);
    assert!(!character.all_caps);
    assert!(!character.small_caps);
    assert!(!character.superscript);
    assert!(!character.subscript);
    assert!(!character.underline);
    assert!(!character.strikethrough);
    assert!(character.fractional_widths);
    assert!(!character.faux_bold);
    assert!(!character.faux_italic);
    assert!(character.standard_ligatures);
    assert!(character.contextual_alternates);
    assert!(!character.discretionary_ligatures);
    assert!(!character.swash);
    assert!(!character.oldstyle);
    assert!(!character.stylistic_alternates);
    assert!(!character.titling_alternates);
    assert!(!character.ornaments);
    assert!(!character.ordinals);
    assert!(!character.fractions);
    assert_eq!(character.language, DEFAULT_LANGUAGE);
    assert!(character.vertical_roman_alignment);

    let paragraph = ParagraphAttrs::default();
    assert_eq!(paragraph.justify, Justify::Left);
    assert_eq!(paragraph.direction, TextDirection::Ltr);
    assert_eq!(paragraph.word_spacing, [0.8, 1.0, 1.33]);
    assert_eq!(paragraph.letter_spacing, [0.0, 0.0, 0.0]);
    assert_eq!(paragraph.glyph_spacing, [1.0, 1.0, 1.0]);
    assert_eq!(paragraph.start_indent, 0.0);
    assert_eq!(paragraph.end_indent, 0.0);
    assert_eq!(paragraph.first_line_indent, 0.0);
    assert_eq!(paragraph.space_before, 0.0);
    assert_eq!(paragraph.space_after, 0.0);
    assert!(!paragraph.hanging);
    assert!(!paragraph.hyphenate);
    assert_eq!(paragraph.composer, Composer::SingleLine);
    assert_eq!(paragraph.auto_leading, 120.0);
    assert_eq!(paragraph.hyphenate_word_size, 5);
    assert_eq!(paragraph.hyphenate_pre, 2);
    assert_eq!(paragraph.hyphenate_post, 2);
    assert_eq!(paragraph.hyphen_limit, 2);
    assert_eq!(paragraph.hyphenation_zone, 36.0);
    assert!(paragraph.hyphenate_caps);

    let sheet = TextStyleSheet::default();
    assert!(sheet
        .paragraph_styles
        .iter()
        .any(|style| style.name == "Basic Paragraph"));
}

#[test]
fn character_defaults_include_language_and_tsume() {
    let character = CharacterAttrs::default();
    assert_eq!(character.language, DEFAULT_LANGUAGE);
    assert_eq!(character.tsume, 0.0);
}

#[test]
fn paragraph_defaults_include_direction() {
    assert_eq!(ParagraphAttrs::default().direction, TextDirection::Ltr);
    assert_eq!(TextDirection::from_index(0), TextDirection::Ltr);
    assert_eq!(TextDirection::from_index(1), TextDirection::Rtl);
    assert_eq!(TextDirection::from_index(99), TextDirection::Ltr);
    assert_eq!(TextDirection::Rtl.index(), 1);
}

#[test]
fn every_character_field_round_trips() {
    let over = CharacterOverrides {
        tsume: Some(60.0),
        language: Some("French".to_string()),
        ..CharacterOverrides::default()
    };
    let mut attrs = CharacterAttrs::default();
    merge_character(&mut attrs, &over);
    assert_eq!(attrs.tsume, 60.0);
    assert_eq!(attrs.language, "French");
}

#[test]
fn every_paragraph_field_round_trips() {
    let over = ParagraphOverrides {
        direction: Some(TextDirection::Rtl),
        ..ParagraphOverrides::default()
    };
    let mut attrs = ParagraphAttrs::default();
    merge_paragraph(&mut attrs, &over);
    assert_eq!(attrs.direction, TextDirection::Rtl);
}

#[test]
fn language_tsume_and_direction_round_trip() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .create_character_style(
            "Quote",
            CharacterOverrides {
                tsume: Some(50.0),
                language: Some("German".to_string()),
                ..CharacterOverrides::default()
            },
        )
        .unwrap();
    let resolved = sheet.resolve(
        &StyleOverrides {
            paragraph: ParagraphOverrides {
                direction: Some(TextDirection::Rtl),
                ..ParagraphOverrides::default()
            },
            ..StyleOverrides::default()
        },
        Some("Quote"),
        None,
    );
    assert_eq!(resolved.character.tsume, 50.0);
    assert_eq!(resolved.character.language, "German");
    assert_eq!(resolved.paragraph.direction, TextDirection::Rtl);
}

#[test]
fn justify_index_round_trips_every_modelled_value() {
    for value in [
        Justify::Left,
        Justify::Right,
        Justify::Center,
        Justify::JustifyLastLeft,
        Justify::JustifyLastRight,
        Justify::JustifyLastCenter,
        Justify::JustifyAll,
    ] {
        assert_eq!(Justify::from_index(value.index()), Some(value));
    }
    assert_eq!(Justify::from_index(7), None);
    assert_eq!(Justify::from_index(255), None);
}

#[test]
fn resolve_prefers_manual_then_character_then_paragraph_then_default() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .edit_paragraph_style(
            BASIC_PARAGRAPH,
            CharacterOverrides {
                size: Some(11.0),
                ..Default::default()
            },
            ParagraphOverrides {
                justify: Some(Justify::Right),
                ..Default::default()
            },
        )
        .unwrap();
    sheet
        .create_paragraph_style(
            "Para",
            CharacterOverrides {
                size: Some(22.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    sheet
        .create_character_style(
            "Char",
            CharacterOverrides {
                size: Some(33.0),
                ..Default::default()
            },
        )
        .unwrap();

    let manual = StyleOverrides {
        character: CharacterOverrides {
            size: Some(44.0),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        sheet
            .resolve(&manual, Some("Char"), Some("Para"))
            .character
            .size,
        44.0
    );
    assert_eq!(
        sheet
            .resolve(&StyleOverrides::default(), Some("Char"), Some("Para"))
            .character
            .size,
        33.0
    );
    assert_eq!(
        sheet
            .resolve(&StyleOverrides::default(), None, Some("Para"))
            .character
            .size,
        22.0
    );
    let default = sheet.resolve(&StyleOverrides::default(), None, None);
    assert_eq!(default.character.size, 11.0);
    assert_eq!(default.paragraph.justify, Justify::Right);
}

#[test]
fn applying_a_style_keeps_a_manual_override_on_another_attribute() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .create_character_style(
            "Heading",
            CharacterOverrides {
                size: Some(33.0),
                ..Default::default()
            },
        )
        .unwrap();
    let manual = StyleOverrides {
        character: CharacterOverrides {
            tracking: Some(120.0),
            ..Default::default()
        },
        ..Default::default()
    };
    let resolved = sheet.resolve(&manual, Some("Heading"), None);
    assert_eq!(resolved.character.tracking, 120.0);
    assert_eq!(resolved.character.size, 33.0);
}

#[test]
fn a_paragraph_style_resolves_its_extended_attributes() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .create_paragraph_style(
            "Heading",
            CharacterOverrides {
                faux_bold: Some(true),
                fractions: Some(true),
                language: Some("French".to_string()),
                ..Default::default()
            },
            ParagraphOverrides {
                auto_leading: Some(150.0),
                hyphenate: Some(true),
                hyphen_limit: Some(3),
                hyphenate_caps: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    let resolved = sheet
        .apply_paragraph_style("Heading", &StyleOverrides::default())
        .unwrap();
    assert!(resolved.character.faux_bold);
    assert!(resolved.character.fractions);
    assert_eq!(resolved.character.language, "French");
    assert!(
        resolved.character.standard_ligatures,
        "a field the style leaves unset keeps the default"
    );
    assert_eq!(resolved.paragraph.auto_leading, 150.0);
    assert!(resolved.paragraph.hyphenate);
    assert_eq!(resolved.paragraph.hyphen_limit, 3);
    assert!(!resolved.paragraph.hyphenate_caps);
}

#[test]
fn applying_a_style_resolves_its_attributes_or_reports_a_missing_name() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .create_character_style(
            "Heading",
            CharacterOverrides {
                size: Some(20.0),
                ..Default::default()
            },
        )
        .unwrap();
    let attrs = sheet
        .apply_character_style("Heading", &CharacterOverrides::default())
        .unwrap();
    assert_eq!(attrs.size, 20.0);
    assert_eq!(
        sheet.apply_character_style("Ghost", &CharacterOverrides::default()),
        Err(StyleError::NotFound("Ghost".to_string()))
    );

    let resolved = sheet
        .apply_paragraph_style(BASIC_PARAGRAPH, &StyleOverrides::default())
        .unwrap();
    assert_eq!(resolved.paragraph.justify, Justify::Left);
    assert_eq!(
        sheet.apply_paragraph_style("Ghost", &StyleOverrides::default()),
        Err(StyleError::NotFound("Ghost".to_string()))
    );
}

#[test]
fn basic_paragraph_refuses_rename_and_delete_but_accepts_edits() {
    let mut sheet = TextStyleSheet::default();
    assert_eq!(
        sheet.rename_paragraph_style(BASIC_PARAGRAPH, "Body"),
        Err(StyleError::Protected(BASIC_PARAGRAPH.to_string()))
    );
    assert_eq!(
        sheet.delete_paragraph_style(BASIC_PARAGRAPH),
        Err(StyleError::Protected(BASIC_PARAGRAPH.to_string()))
    );
    let paragraph = ParagraphOverrides {
        justify: Some(Justify::Center),
        ..Default::default()
    };
    sheet
        .edit_paragraph_style(
            BASIC_PARAGRAPH,
            CharacterOverrides::default(),
            paragraph.clone(),
        )
        .unwrap();
    assert_eq!(
        sheet.paragraph_style(BASIC_PARAGRAPH).unwrap().paragraph,
        paragraph
    );
    assert_eq!(sheet.paragraph_styles.len(), 1);
}

#[test]
fn renaming_a_custom_style_onto_basic_paragraph_names_the_protected_target() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .create_paragraph_style(
            "Body",
            CharacterOverrides::default(),
            ParagraphOverrides::default(),
        )
        .unwrap();
    assert_eq!(
        sheet.rename_paragraph_style("Body", BASIC_PARAGRAPH),
        Err(StyleError::Protected(BASIC_PARAGRAPH.to_string()))
    );
    assert!(sheet.paragraph_style("Body").is_some());
}

#[test]
fn a_character_style_leaves_the_paragraph_styles_character_defaults_alone() {
    let mut sheet = TextStyleSheet::default();
    sheet
        .create_paragraph_style(
            "Body",
            CharacterOverrides {
                size: Some(18.0),
                tracking: Some(50.0),
                ..Default::default()
            },
            ParagraphOverrides::default(),
        )
        .unwrap();
    sheet
        .create_character_style(
            "Bold",
            CharacterOverrides {
                size: Some(33.0),
                ..Default::default()
            },
        )
        .unwrap();
    let resolved = sheet.resolve(&StyleOverrides::default(), Some("Bold"), Some("Body"));
    assert_eq!(
        resolved.character.size, 33.0,
        "the character style wins for the field it sets"
    );
    assert_eq!(
        resolved.character.tracking, 50.0,
        "the paragraph default survives a field the character style leaves unset"
    );
}

#[test]
fn create_delete_and_rename_reject_missing_and_duplicate_names() {
    let mut sheet = TextStyleSheet::default();
    assert_eq!(
        sheet.delete_character_style("Ghost"),
        Err(StyleError::NotFound("Ghost".to_string()))
    );
    sheet
        .create_character_style("One", CharacterOverrides::default())
        .unwrap();
    assert_eq!(
        sheet.create_character_style("One", CharacterOverrides::default()),
        Err(StyleError::Duplicate("One".to_string()))
    );
    sheet.rename_character_style("One", "Two").unwrap();
    assert!(sheet.character_style("One").is_none());
    assert!(sheet.character_style("Two").is_some());
    assert_eq!(
        sheet.rename_character_style("Ghost", "Three"),
        Err(StyleError::NotFound("Ghost".to_string()))
    );
    sheet.delete_character_style("Two").unwrap();
    assert!(sheet.character_styles.is_empty());
}
