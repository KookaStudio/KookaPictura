use super::*;
use crate::descriptor::read_descriptor;
use crate::engine_data::{as_list, extract_fonts, extract_style, path};
use pictura_core::{AntiAlias, Justify};

fn spec(text: &str, vertical: bool) -> TypeSpec {
    let mut spec = TypeSpec::new(text, "Liberation Sans", 36.0);
    spec.vertical = vertical;
    spec.origin = (40.0, 60.0);
    spec.character.fill_color = [1.0, 1.0, 0.0, 0.0];
    spec.paragraph.justify = Justify::Center;
    spec
}

fn read_style(tool: &TypeTool) -> pictura_core::TextStyle {
    decode_type_tool(&crate::encode_type_tool(tool))
        .expect("decodes")
        .style
        .expect("style")
}

/// A version-16 text descriptor carrying `engine` as its EngineData.
fn descriptor_with_engine(engine: &[u8]) -> Vec<u8> {
    let mut raw = b"tdta".to_vec();
    raw.extend_from_slice(&(engine.len() as u32).to_be_bytes());
    raw.extend_from_slice(engine);
    write_descriptor(&object(
        "TxLr",
        vec![
            (key("Txt "), DescValue::Text("Hi\0".into())),
            (key("AntA"), enumerated("Annt", "antiAliasNone")),
            (key("EngineData"), DescValue::Raw(raw)),
        ],
    ))
}

#[test]
fn authored_type_tool_reads_back_as_written() {
    let tool = author_type_tool(
        &spec("Hi (there)\rtwo", false),
        [-50.0, -30.0, 50.0, 40.0],
        None,
        None,
    );
    let back = decode_type_tool(&crate::encode_type_tool(&tool)).expect("decodes");
    assert_eq!(back, tool);
    assert_eq!(back.text, "Hi (there)\rtwo");
    assert_eq!(back.transform, [1.0, 0.0, 0.0, 1.0, 40.0, 60.0]);
    assert_eq!(back.bounds, [-50, -30, 50, 40]);
    assert!(!back.vertical);
    assert_eq!(back.fonts, ["LiberationSans"]);
    let style = back.style.expect("style");
    assert_eq!(style.font.as_deref(), Some("LiberationSans"));
    assert_eq!(style.character.font_family, "Liberation Sans");
    assert_eq!(style.character.size, 36.0);
    assert_eq!(style.rgba(), [255, 0, 0, 255]);
    assert_eq!(style.paragraph.justify.index(), 2);
}

#[test]
fn vertical_orientation_round_trips() {
    let tool = author_type_tool(&spec("V", true), [0.0, 0.0, 10.0, 10.0], None, None);
    assert!(tool.vertical);
    let back = decode_type_tool(&crate::encode_type_tool(&tool)).expect("decodes");
    assert!(back.vertical);
}

#[test]
fn run_lengths_cover_the_engine_text() {
    let engine = skeleton_engine_data(&spec("ab\rcd", false), None);
    assert_eq!(engine.len() % 2, 0);
    let root = parse_engine_data(&engine).expect("parses");
    let text = format!("{root:?}");
    assert!(text.contains("String(\"ab\\rcd\\r\")"), "{text}");
    assert!(text.contains("(\"RunLengthArray\", List([Int(3), Int(3)]))"));
    assert!(text.contains("(\"RunLengthArray\", List([Int(6)]))"));
}

#[test]
fn every_modelled_character_and_paragraph_key_reads_back() {
    let mut s = spec("Track", false);
    s.character.tracking = 120.0;
    s.character.horizontal_scale = 85.0;
    s.character.vertical_scale = 110.0;
    s.character.leading = Leading::Fixed(26.0);
    s.character.kerning = KerningMode::Manual(-25);
    s.character.baseline_shift = -4.0;
    s.character.all_caps = true;
    s.character.underline = true;
    s.character.strikethrough = true;
    s.character.fractional_widths = false;
    s.paragraph.first_line_indent = 24.0;
    s.paragraph.start_indent = 6.0;
    s.paragraph.end_indent = 3.0;
    s.paragraph.space_before = 5.0;
    s.paragraph.space_after = 12.0;
    s.paragraph.hanging = true;
    s.paragraph.hyphenate = true;
    s.paragraph.composer = Composer::EveryLine;
    s.paragraph.word_spacing = [0.8, 1.1, 1.33];
    s.paragraph.letter_spacing = [-0.1, 0.0, 0.2];
    s.paragraph.glyph_spacing = [0.9, 1.0, 1.1];

    let style = read_style(&author_type_tool(&s, [0.0, 0.0, 10.0, 10.0], None, None));
    let c = &style.character;
    assert_eq!(c.tracking, 120.0);
    assert!((c.horizontal_scale - 85.0).abs() < 1e-6);
    assert!((c.vertical_scale - 110.0).abs() < 1e-6);
    assert_eq!(c.leading, Leading::Fixed(26.0));
    assert_eq!(c.kerning, KerningMode::Manual(-25));
    assert_eq!(c.baseline_shift, -4.0);
    assert!(c.all_caps && !c.small_caps);
    assert!(c.underline && c.strikethrough);
    assert!(!c.fractional_widths);
    let p = &style.paragraph;
    assert_eq!(p.first_line_indent, 24.0);
    assert_eq!(p.start_indent, 6.0);
    assert_eq!(p.end_indent, 3.0);
    assert_eq!(p.space_before, 5.0);
    assert_eq!(p.space_after, 12.0);
    assert!(p.hanging && p.hyphenate);
    assert_eq!(p.composer, Composer::EveryLine);
    assert_eq!(p.word_spacing, [0.8, 1.1, 1.33]);
    assert_eq!(p.letter_spacing, [-0.1, 0.0, 0.2]);
    assert_eq!(p.glyph_spacing, [0.9, 1.0, 1.1]);
}

#[test]
fn each_anti_alias_method_authors_to_descriptor_and_flag() {
    for method in [
        AntiAlias::None,
        AntiAlias::Sharp,
        AntiAlias::Crisp,
        AntiAlias::Strong,
        AntiAlias::Smooth,
    ] {
        let mut s = spec("A", false);
        s.character.anti_alias = method;
        let tool = author_type_tool(&s, [0.0, 0.0, 10.0, 10.0], None, None);
        let value =
            read_descriptor(&mut crate::common::Reader::new(&tool.text_desc)).expect("descriptor");
        let DescValue::Object { items, .. } = &value else {
            panic!("object");
        };
        let Some(DescValue::Enum { value, .. }) = get_object_item(items, b"AntA") else {
            panic!("AntA");
        };
        assert_eq!(value.as_slice(), ant_alias_spelling(method).as_bytes());

        let root = parse_engine_data(&skeleton_engine_data(&s, None)).expect("parses");
        assert_eq!(
            get(&root, "EngineDict").and_then(|e| get(e, "AntiAlias")),
            Some(&EngineValue::Int(method.index() as i64))
        );
        assert_eq!(read_style(&tool).character.anti_alias, method);
    }
}

#[test]
fn re_author_preserves_unmodeled_keys_and_merges_the_model() {
    let source = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00H\x00i\x00\r) >> \
            /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << \
            /Font 0 /FontSize 20.0 /Tracking 5 /NoBreak true /Ligatures false \
            /FillColor << /Values [ 1.0 0.0 0.0 1.0 ] >> >> >> >> ] >> \
            /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> \
            >> /ResourceDict << /FontSet [ << /Name (LiberationSans) >> ] \
            /StyleSheetSet [ << /StyleSheetData << /Font 0 /Tracking 9 >> >> ] >> >>";
    let existing = descriptor_with_engine(source);
    let mut s = spec("Hi", false);
    s.character.tracking = 77.0;

    let merged = engine_data(&s, Some(&existing), None);
    let root = parse_engine_data(&merged).expect("parses");
    let fonts = extract_fonts(&root);
    let style = extract_style(&root, &fonts);
    assert_eq!(style.character.tracking, 77.0, "the model is merged");
    let data = path(&root, &["EngineDict", "StyleRun", "RunArray"])
        .and_then(|list| as_list(list).first())
        .and_then(|entry| path(entry, &["StyleSheet", "StyleSheetData"]))
        .expect("style data");
    assert_eq!(get(data, "NoBreak"), Some(&EngineValue::Bool(true)));
    // `Ligatures` is modelled now, so the model's default (true) wins over the
    // source's false; `NoBreak` above proves unmodelled keys survive.
    assert_eq!(get(data, "Ligatures"), Some(&EngineValue::Bool(true)));
}

#[test]
fn no_existing_engine_data_authors_a_complete_skeleton() {
    let tool = author_type_tool(&spec("New", false), [0.0, 0.0, 10.0, 10.0], None, None);
    let root = parse_engine_data(&engine_data(&spec("New", false), None, None)).expect("parses");
    let fonts = extract_fonts(&root);
    let style = extract_style(&root, &fonts);
    assert_eq!(style.character.size, 36.0);
    assert_eq!(style.font.as_deref(), Some("LiberationSans"));
    assert_eq!(style.paragraph.justify, Justify::Center);
    assert!(style.character.standard_ligatures);
    assert!(style.character.contextual_alternates);
    assert!(!style.character.faux_bold);
    assert_eq!(style.paragraph.auto_leading, 120.0);
    assert_eq!(style.paragraph.hyphenate_word_size, 5);
    assert_eq!(style.paragraph.hyphen_limit, 2);
    assert_eq!(tool.text, "New");
}

#[test]
fn multi_run_engine_data_collapses_to_the_single_run() {
    let source = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00a\x00\r) >> \
            /StyleRun << /RunArray [ \
            << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 10.0 >> >> >> \
            << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 40.0 >> >> >> ] \
            /RunLengthArray [ 1 1 ] >> \
            /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> \
            >> /ResourceDict << /FontSet [ << /Name (X) >> ] >> >>";
    let existing = descriptor_with_engine(source);
    let merged = engine_data(&spec("a", false), Some(&existing), None);
    let root = parse_engine_data(&merged).expect("parses");
    let runs = path(&root, &["EngineDict", "StyleRun", "RunArray"])
        .map(as_list)
        .expect("run array");
    assert_eq!(runs.len(), 1);
    let lengths = path(&root, &["EngineDict", "StyleRun", "RunLengthArray"])
        .map(as_list)
        .expect("lengths");
    assert_eq!(lengths, [EngineValue::Int(2)]);
}

#[test]
fn merge_preserves_the_unmodeled_leading_type() {
    let source = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00H\x00i\x00\r) >> \
            /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 20.0 \
            /FillColor << /Values [ 1.0 0.0 0.0 1.0 ] >> >> >> >> ] >> \
            /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << \
            /Justification 0 /LeadingType 2 >> >> >> ] >> \
            >> /ResourceDict << /FontSet [ << /Name (LiberationSans) >> ] >> >>";
    let existing = descriptor_with_engine(source);
    let merged = engine_data(&spec("Hi", false), Some(&existing), None);
    let root = parse_engine_data(&merged).expect("parses");
    let props = path(&root, &["EngineDict", "ParagraphRun", "RunArray"])
        .and_then(|list| as_list(list).first())
        .and_then(|entry| path(entry, &["ParagraphSheet", "Properties"]))
        .expect("paragraph properties");
    assert_eq!(
        get(props, "LeadingType"),
        Some(&EngineValue::Int(2)),
        "a merge must not clobber the unmodeled /LeadingType"
    );
}

#[test]
fn every_justification_value_authors_and_reads_back() {
    for value in [
        Justify::Left,
        Justify::Right,
        Justify::Center,
        Justify::JustifyLastLeft,
        Justify::JustifyLastRight,
        Justify::JustifyLastCenter,
        Justify::JustifyAll,
    ] {
        let mut s = spec("J", false);
        s.paragraph.justify = value;
        let style = read_style(&author_type_tool(&s, [0.0, 0.0, 10.0, 10.0], None, None));
        assert_eq!(style.paragraph.justify, value);
        assert_eq!(style.paragraph.justify.index(), value.index());
    }
}

#[test]
fn all_caps_wins_over_small_caps() {
    let mut s = spec("Caps", false);
    s.character.all_caps = true;
    s.character.small_caps = true;
    let tool = author_type_tool(&s, [0.0, 0.0, 10.0, 10.0], None, None);
    let style = read_style(&tool);
    assert!(style.character.all_caps);
    assert!(!style.character.small_caps);

    let root = parse_engine_data(&engine_data(&s, None, None)).expect("parses");
    let data = path(&root, &["EngineDict", "StyleRun", "RunArray"])
        .and_then(|list| as_list(list).first())
        .and_then(|entry| path(entry, &["StyleSheet", "StyleSheetData"]))
        .expect("style data");
    assert_eq!(get(data, "FontCaps"), Some(&EngineValue::Int(2)));
}

fn font_set(root: &EngineValue, container: &str) -> Vec<(String, EngineValue)> {
    path(root, &[container, "FontSet"])
        .map(as_list)
        .unwrap_or(&[])
        .iter()
        .map(|entry| {
            (
                get(entry, "Name")
                    .and_then(as_str)
                    .unwrap_or("")
                    .to_string(),
                entry.clone(),
            )
        })
        .collect()
}

fn run_font_index(root: &EngineValue) -> i64 {
    path(root, &["EngineDict", "StyleRun", "RunArray"])
        .and_then(|list| as_list(list).first())
        .and_then(|entry| path(entry, &["StyleSheet", "StyleSheetData"]))
        .and_then(|data| get(data, "Font"))
        .and_then(|value| match value {
            EngineValue::Int(index) => Some(*index),
            _ => None,
        })
        .expect("run font index")
}

#[test]
fn re_author_matches_font_by_family_and_keeps_its_style() {
    let source = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00H\x00i\x00\r) >> \
            /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 20.0 \
            /FillColor << /Values [ 1.0 0.0 0.0 1.0 ] >> >> >> >> ] >> \
            /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> \
            >> /ResourceDict << /FontSet [ << /Name (ArialMT) /FontFamily (Arial) /FontStyle (Bold) >> ] >> >>";
    let existing = descriptor_with_engine(source);
    let mut s = spec("Hi", false);
    s.character.font_family = "Arial".into();
    s.character.font_style = "Bold".into();
    let root = parse_engine_data(&engine_data(&s, Some(&existing), Some("ArialMT"))).unwrap();

    let fonts = font_set(&root, "ResourceDict");
    assert_eq!(fonts.len(), 1, "the family match reuses the entry");
    assert_eq!(
        get(&fonts[0].1, "FontStyle"),
        Some(&EngineValue::String("Bold".into()))
    );
    assert_eq!(run_font_index(&root), 0);
}

#[test]
fn appended_font_entry_carries_family_and_style() {
    let source = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00H\x00i\x00\r) >> \
            /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 20.0 \
            /FillColor << /Values [ 1.0 0.0 0.0 1.0 ] >> >> >> >> ] >> \
            /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> \
            >> /ResourceDict << /FontSet [ << /Name (Other) >> ] >> >>";
    let existing = descriptor_with_engine(source);
    let mut s = spec("Hi", false);
    s.character.font_family = "Arial".into();
    s.character.font_style = "Bold".into();
    let root = parse_engine_data(&engine_data(&s, Some(&existing), None)).unwrap();

    let fonts = font_set(&root, "ResourceDict");
    assert_eq!(fonts.len(), 2);
    assert_eq!(fonts[1].0, "Arial");
    assert_eq!(
        get(&fonts[1].1, "FontFamily"),
        Some(&EngineValue::String("Arial".into()))
    );
    assert_eq!(
        get(&fonts[1].1, "FontStyle"),
        Some(&EngineValue::String("Bold".into()))
    );
    assert_eq!(run_font_index(&root), 1);
}

#[test]
fn font_index_ignores_a_document_resources_set_the_reader_skips() {
    let source = b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00H\x00i\x00\r) >> \
            /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << /Font 0 /FontSize 20.0 \
            /FillColor << /Values [ 1.0 0.0 0.0 1.0 ] >> >> >> >> ] >> \
            /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << /Justification 0 >> >> >> ] >> \
            >> /ResourceDict << /FontSet [ << /Name (Other) >> ] >> \
            /DocumentResources << /FontSet [ << /Name (Arial) >> ] >> >>";
    let existing = descriptor_with_engine(source);
    let mut s = spec("Hi", false);
    s.character.font_family = "Arial".into();
    let root = parse_engine_data(&engine_data(&s, Some(&existing), None)).unwrap();

    assert_eq!(font_set(&root, "ResourceDict").len(), 2);
    assert_eq!(font_set(&root, "DocumentResources").len(), 1);
    assert_eq!(run_font_index(&root), 1, "the index names the reader's set");
}

fn shape_type(root: &EngineValue) -> Option<&EngineValue> {
    path(root, &["EngineDict", "Rendered", "Shapes", "Children"])
        .and_then(|list| as_list(list).first())
        .and_then(|shape| path(shape, &["Cookie", "Photoshop", "ShapeType"]))
}

#[test]
fn paragraph_box_round_trips() {
    let mut boxed = spec("a long caption", false);
    boxed.box_bounds = Some([0.0, 0.0, 120.5, 80.0]);
    let tool = author_type_tool(&boxed, [0.0, 0.0, 120.5, 80.0], None, None);
    let back = decode_type_tool(&crate::encode_type_tool(&tool)).expect("decodes");
    assert_eq!(back.box_bounds, Some([0.0, 0.0, 120.5, 80.0]));
    assert_eq!(back, tool);
    let root = parse_engine_data(&skeleton_engine_data(&boxed, None)).expect("parses");
    assert_eq!(shape_type(&root), Some(&EngineValue::Int(1)));

    let point = author_type_tool(&spec("Hi", false), [0.0, -30.0, 40.0, 10.0], None, None);
    let back = decode_type_tool(&crate::encode_type_tool(&point)).expect("decodes");
    assert_eq!(back.box_bounds, None);
    let root = parse_engine_data(&skeleton_engine_data(&spec("Hi", false), None)).expect("parses");
    assert_eq!(shape_type(&root), Some(&EngineValue::Int(0)));

    // A merge over existing EngineData (no shape, or the other kind) writes
    // the spec's shape, and a box with a non-zero corner survives.
    let existing = descriptor_with_engine(
        b"<< /EngineDict << /Editor << /Text (\xFE\xFF\x00H\x00i\x00\r) >> >> >>",
    );
    boxed.box_bounds = Some([-10.0, -20.0, 50.0, 30.0]);
    let merged = author_type_tool(&boxed, [-10.0, -20.0, 50.0, 30.0], Some(&existing), None);
    let back = decode_type_tool(&crate::encode_type_tool(&merged)).expect("decodes");
    assert_eq!(back.box_bounds, Some([-10.0, -20.0, 50.0, 30.0]));
    let to_point = author_type_tool(&spec("Hi", false), [0.0; 4], Some(&merged.text_desc), None);
    let back = decode_type_tool(&crate::encode_type_tool(&to_point)).expect("decodes");
    assert_eq!(back.box_bounds, None, "re-set as point type drops the box");
}
