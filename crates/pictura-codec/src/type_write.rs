//! Author a new type layer's `TySh` block from a [`TypeSpec`].
//!
//! The read side ([`crate::type_tool`]) keeps an opened file's descriptors
//! verbatim; a layer the Type tools create has none, so this writes them. The
//! text descriptor carries the string and the layer-level choices (orientation,
//! anti-aliasing, bounds); its `EngineData` is the text engine's own dump. That
//! dump has to be complete enough for Photoshop to build a live text object —
//! the run lengths must add up to the string, the run must name a font in the
//! font set — or it falls back to the pixels. `/Rendered` carries only the
//! shape (point or paragraph box, [`crate::type_shape`]); its line cache is
//! left for Photoshop to rebuild. Not written: warping (the warp descriptor
//! says none) and kinsoku / mojikumi sets (empty). Ported from photorust's `core/src/psd/text_write.rs`.
//!
//! When the layer already has a text descriptor, its EngineData is parsed and
//! the modelled keys of the single style run, paragraph sheet, and text are
//! overwritten in place; every other key survives. A block with more than one
//! style run collapses to the single modelled run (`ponytail:` ceiling named
//! below). With no parseable block, the from-scratch skeleton is emitted.

use pictura_core::{Composer, KerningMode, Leading, TypeSpec, TypeTool};

use crate::descriptor::{get_object_item, write_descriptor, DescValue};
use crate::encode_engine_data;
use crate::engine_data::{as_str, get, parse_engine_data, EngineValue};
use crate::type_tool::{ant_alias_spelling, decode_type_tool, engine_data_payload};

/// The `TySh` view for `spec`, decoded back from the bytes it encodes to so the
/// fonts and style match what a reopened file reports. `bounds` is the text box
/// relative to the origin: left, top, right, bottom. `existing` is the layer's
/// current text descriptor bytes, whose EngineData is merged over.
/// `font_name` is the face's PostScript name when the caller read one (a
/// reopened layer's `TextStyle::font`), so its `/FontSet` entry is matched
/// rather than duplicated.
pub fn author_type_tool(
    spec: &TypeSpec,
    bounds: [f64; 4],
    existing: Option<&[u8]>,
    font_name: Option<&str>,
) -> TypeTool {
    let mut tool = TypeTool {
        transform: [
            spec.matrix[0],
            spec.matrix[1],
            spec.matrix[2],
            spec.matrix[3],
            spec.origin.0,
            spec.origin.1,
        ],
        text: spec.text.clone(),
        // ponytail: the trailing four integers are written as the rounded text
        // box; photorust writes zeros. Settle against a CS6-authored corpus.
        bounds: bounds.map(|v| v.round() as i32),
        text_desc: text_descriptor(spec, bounds, existing, font_name),
        warp_desc: warp_descriptor(),
        fonts: Vec::new(),
        style: None,
        vertical: spec.vertical,
        box_bounds: spec.box_bounds,
    };
    if let Some(decoded) = decode_type_tool(&crate::encode_type_tool(&tool)) {
        tool.fonts = decoded.fonts;
        tool.style = decoded.style;
    }
    tool
}

fn key(name: &str) -> Vec<u8> {
    name.as_bytes().to_vec()
}

fn enumerated(kind: &str, value: &str) -> DescValue {
    DescValue::Enum {
        kind: key(kind),
        value: key(value),
    }
}

fn object(class: &str, items: Vec<(Vec<u8>, DescValue)>) -> DescValue {
    DescValue::Object {
        name: String::new(),
        class_id: key(class),
        items,
    }
}

fn bounds_object(class: &str, bounds: [f64; 4]) -> DescValue {
    let items = ["Left", "Top ", "Rght", "Btom"]
        .iter()
        .zip(bounds)
        .map(|(name, value)| {
            (
                key(name),
                DescValue::UnitFloat {
                    unit: *b"#Pnt",
                    value,
                },
            )
        })
        .collect();
    object(class, items)
}

fn text_descriptor(
    spec: &TypeSpec,
    bounds: [f64; 4],
    existing: Option<&[u8]>,
    font_name: Option<&str>,
) -> Vec<u8> {
    let engine = engine_data(spec, existing, font_name);
    let mut raw = b"tdta".to_vec();
    raw.extend_from_slice(&(engine.len() as u32).to_be_bytes());
    raw.extend_from_slice(&engine);
    write_descriptor(&object(
        "TxLr",
        vec![
            (
                key("Txt "),
                DescValue::Text(format!("{}\0", engine_text(&spec.text))),
            ),
            (key("textGridding"), enumerated("textGridding", "None")),
            (
                key("Ornt"),
                enumerated("Ornt", if spec.vertical { "Vrtc" } else { "Hrzn" }),
            ),
            // ponytail: the Crisp/Strong/Smooth `AntA` spellings are inferred;
            // validated against a CS6-authored PSD in the type-style-model
            // follow-up.
            (
                key("AntA"),
                enumerated("Annt", ant_alias_spelling(spec.character.anti_alias)),
            ),
            // Unwarped type occupies exactly its own box: the text's for
            // point type, the paragraph box for paragraph type.
            (key("bounds"), bounds_object("bounds", bounds)),
            (key("boundingBox"), bounds_object("boundingBox", bounds)),
            (key("TextIndex"), DescValue::Long(0)),
            (key("EngineData"), DescValue::Raw(raw)),
        ],
    ))
}

fn warp_descriptor() -> Vec<u8> {
    write_descriptor(&object(
        "warp",
        vec![
            (key("warpStyle"), enumerated("warpStyle", "warpNone")),
            (key("warpValue"), DescValue::Double(0.0)),
            (key("warpPerspective"), DescValue::Double(0.0)),
            (key("warpPerspectiveOther"), DescValue::Double(0.0)),
            (key("warpRotate"), enumerated("Ornt", "Hrzn")),
        ],
    ))
}

/// Line breaks as the engine wants them: carriage returns only. A `\n` left in
/// the string shows in Photoshop as a missing-glyph box.
fn engine_text(text: &str) -> String {
    text.replace("\r\n", "\r").replace('\n', "\r")
}

/// EngineData's `/Text`, which always ends in a paragraph-closing `\r` that the
/// descriptor's `Txt ` does not carry; the run lengths count it.
fn engine_body(text: &str) -> String {
    let mut body = engine_text(text);
    if !body.ends_with('\r') {
        body.push('\r');
    }
    body
}

/// The EngineData for a re-set layer: merge over the existing tree when one
/// parses, else the complete skeleton.
fn engine_data(spec: &TypeSpec, existing: Option<&[u8]>, font_name: Option<&str>) -> Vec<u8> {
    if let Some(root) = existing.and_then(existing_engine_tree) {
        let mut root = root;
        merge_engine_data(&mut root, spec, font_name);
        // A parsed tree is bounded by the parser's depth cap, so encoding it
        // cannot exceed the encoder's cap; fall back to the skeleton only if a
        // programmatic tree ever did.
        if let Ok(bytes) = encode_engine_data(&root) {
            return bytes;
        }
    }
    skeleton_engine_data(spec, font_name)
}

/// The parsed EngineData of a text descriptor, or `None` when the descriptor or
/// its EngineData is missing or unparseable.
pub(crate) fn existing_engine_tree(text_desc: &[u8]) -> Option<EngineValue> {
    let mut reader = crate::common::Reader::new(text_desc);
    let value = crate::descriptor::read_descriptor(&mut reader).ok()?;
    let DescValue::Object { items, .. } = value else {
        return None;
    };
    let DescValue::Raw(raw) = get_object_item(&items, b"EngineData")? else {
        return None;
    };
    parse_engine_data(engine_data_payload(raw)).ok()
}

/// Overwrite the modelled keys of the parsed tree, preserving every unmodeled
/// one; collapse style runs to one and rebuild the paragraph runs to match the
/// text.
fn merge_engine_data(root: &mut EngineValue, spec: &TypeSpec, font_name: Option<&str>) {
    let font = font_name
        .map(str::to_owned)
        .unwrap_or_else(|| postscript_name(&spec.character.font_family));
    let font_index = ensure_font(
        root,
        &font,
        &spec.character.font_family,
        &spec.character.font_style,
    );
    let body = engine_body(&spec.text);
    let style_total = body.encode_utf16().count() as i64;
    let para_lengths: Vec<i64> = paragraph_lengths(&body)
        .into_iter()
        .map(|n| n as i64)
        .collect();

    let engine = child(root, "EngineDict");
    set_child(
        child(engine, "Editor"),
        "Text",
        EngineValue::String(body.clone()),
    );
    set_child(
        engine,
        "AntiAlias",
        EngineValue::Int(spec.character.anti_alias.index() as i64),
    );
    set_child(
        engine,
        "UseFractionalGlyphWidths",
        EngineValue::Bool(spec.character.fractional_widths),
    );
    set_child(
        engine,
        "Rendered",
        crate::type_shape::rendered(spec.box_bounds),
    );

    let style_run = child(engine, "StyleRun");
    let style_template = {
        let entry = first_entry(style_run);
        merge_character(style_data_of(entry), spec, font_index);
        entry.clone()
    };
    if let Some(default) = dict_get_mut(style_run, "DefaultRunData") {
        merge_character(style_data_of(default), spec, font_index);
    }
    set_child(
        style_run,
        "RunArray",
        EngineValue::List(vec![style_template]),
    );
    set_child(
        style_run,
        "RunLengthArray",
        EngineValue::List(vec![EngineValue::Int(style_total)]),
    );

    let para_run = child(engine, "ParagraphRun");
    let para_template = {
        let entry = first_entry(para_run);
        merge_paragraph(paragraph_props_of(entry), spec);
        entry.clone()
    };
    if let Some(default) = dict_get_mut(para_run, "DefaultRunData") {
        merge_paragraph(paragraph_props_of(default), spec);
    }
    let count = para_lengths.len().max(1);
    set_child(
        para_run,
        "RunArray",
        EngineValue::List(vec![para_template; count]),
    );
    set_child(
        para_run,
        "RunLengthArray",
        EngineValue::List(para_lengths.iter().map(|n| EngineValue::Int(*n)).collect()),
    );

    for container in ["ResourceDict", "DocumentResources"] {
        let Some(res) = dict_get_mut(root, container) else {
            continue;
        };
        if let Some(data) = set_style_data(res, "StyleSheetSet") {
            merge_character(data, spec, font_index);
        }
        if let Some(props) = set_paragraph_props(res, "ParagraphSheetSet") {
            merge_paragraph(props, spec);
        }
    }
}

/// Overwrite the modelled character keys of one `StyleSheetData`, preserving
/// the rest.
fn merge_character(data: &mut EngineValue, spec: &TypeSpec, font_index: i64) {
    let c = &spec.character;
    set_child(data, "Font", EngineValue::Int(font_index));
    set_child(data, "FontSize", EngineValue::Double(c.size));
    match c.leading {
        Leading::Auto => {
            set_child(data, "AutoLeading", EngineValue::Bool(true));
            set_child(data, "Leading", EngineValue::Double(c.size * 1.2));
        }
        Leading::Fixed(value) => {
            set_child(data, "AutoLeading", EngineValue::Bool(false));
            set_child(data, "Leading", EngineValue::Double(value));
        }
    }
    // Tracking is 1/1000 em, an integer in EngineData (psd-tools `tracking`).
    set_child(
        data,
        "Tracking",
        EngineValue::Int(c.tracking.round() as i64),
    );
    let (auto_kerning, kerning) = match c.kerning {
        KerningMode::Manual(value) => (false, value),
        KerningMode::Metrics => (true, 0),
    };
    set_child(data, "Kerning", EngineValue::Int(kerning as i64));
    set_child(data, "AutoKerning", EngineValue::Bool(auto_kerning));
    set_child(
        data,
        "HorizontalScale",
        EngineValue::Double(c.horizontal_scale / 100.0),
    );
    set_child(
        data,
        "VerticalScale",
        EngineValue::Double(c.vertical_scale / 100.0),
    );
    set_child(data, "BaselineShift", EngineValue::Double(c.baseline_shift));
    set_child(data, "FontCaps", EngineValue::Int(font_caps(c)));
    set_child(data, "FontBaseline", EngineValue::Int(font_baseline(c)));
    set_child(data, "Underline", EngineValue::Bool(c.underline));
    set_child(data, "Strikethrough", EngineValue::Bool(c.strikethrough));
    set_child(data, "FauxBold", EngineValue::Bool(c.faux_bold));
    set_child(data, "FauxItalic", EngineValue::Bool(c.faux_italic));
    // ponytail: the OpenType keys are the CS6 names; `language` and
    // `vertical_roman_alignment` stay in the model (their EngineData encoding —
    // a script index — is not modelled).
    set_child(data, "Ligatures", EngineValue::Bool(c.standard_ligatures));
    set_child(
        data,
        "ContextualLigatures",
        EngineValue::Bool(c.contextual_alternates),
    );
    set_child(
        data,
        "DiscretionaryLigatures",
        EngineValue::Bool(c.discretionary_ligatures),
    );
    set_child(data, "Swash", EngineValue::Bool(c.swash));
    set_child(data, "OldStyle", EngineValue::Bool(c.oldstyle));
    set_child(
        data,
        "StylisticAlternates",
        EngineValue::Bool(c.stylistic_alternates),
    );
    set_child(
        data,
        "TitlingAlternates",
        EngineValue::Bool(c.titling_alternates),
    );
    set_child(data, "Ornaments", EngineValue::Bool(c.ornaments));
    set_child(data, "Ordinals", EngineValue::Bool(c.ordinals));
    set_child(data, "Fractions", EngineValue::Bool(c.fractions));
    let mut fill = EngineValue::Dict(Vec::new());
    set_child(&mut fill, "Type", EngineValue::Int(1));
    set_child(
        &mut fill,
        "Values",
        EngineValue::List(
            c.fill_color
                .iter()
                .map(|v| EngineValue::Double(*v))
                .collect(),
        ),
    );
    set_child(data, "FillColor", fill);
}

/// Overwrite the modelled paragraph keys of one `Properties`, preserving the
/// rest.
fn merge_paragraph(props: &mut EngineValue, spec: &TypeSpec) {
    let p = &spec.paragraph;
    set_child(
        props,
        "Justification",
        EngineValue::Int(p.justify.index() as i64),
    );
    set_child(
        props,
        "FirstLineIndent",
        EngineValue::Double(p.first_line_indent),
    );
    set_child(props, "StartIndent", EngineValue::Double(p.start_indent));
    set_child(props, "EndIndent", EngineValue::Double(p.end_indent));
    set_child(props, "SpaceBefore", EngineValue::Double(p.space_before));
    set_child(props, "SpaceAfter", EngineValue::Double(p.space_after));
    set_child(props, "AutoHyphenate", EngineValue::Bool(p.hyphenate));
    set_child(props, "Hanging", EngineValue::Bool(p.hanging));
    // ponytail: `hyphenate_caps` stays in the model (its EngineData key is not
    // confirmed); the rest of the hyphenation dictionary is authored.
    set_child(
        props,
        "HyphenatedWordSize",
        EngineValue::Int(i64::from(p.hyphenate_word_size)),
    );
    set_child(
        props,
        "PreHyphen",
        EngineValue::Int(i64::from(p.hyphenate_pre)),
    );
    set_child(
        props,
        "PostHyphen",
        EngineValue::Int(i64::from(p.hyphenate_post)),
    );
    set_child(
        props,
        "ConsecutiveHyphens",
        EngineValue::Int(i64::from(p.hyphen_limit)),
    );
    set_child(props, "Zone", EngineValue::Double(p.hyphenation_zone));
    set_child(
        props,
        "AutoLeading",
        EngineValue::Double(p.auto_leading / 100.0),
    );
    for (key, values) in [
        ("WordSpacing", p.word_spacing),
        ("LetterSpacing", p.letter_spacing),
        ("GlyphSpacing", p.glyph_spacing),
    ] {
        set_child(
            props,
            key,
            EngineValue::List(values.iter().map(|v| EngineValue::Double(*v)).collect()),
        );
    }
    // `/LeadingType` has no model field: a merge must preserve the existing
    // value. The from-scratch skeleton writes the constant 0.
    set_child(
        props,
        "EveryLineComposer",
        EngineValue::Bool(p.composer == Composer::EveryLine),
    );
}

fn font_caps(c: &pictura_core::CharacterAttrs) -> i64 {
    if c.all_caps {
        2
    } else if c.small_caps {
        1
    } else {
        0
    }
}

fn font_baseline(c: &pictura_core::CharacterAttrs) -> i64 {
    if c.superscript {
        1
    } else if c.subscript {
        2
    } else {
        0
    }
}

/// The `FontSet` the reader resolves `/Font` against: the first of
/// `ResourceDict`, then `DocumentResources`, that carries one; created under
/// `ResourceDict` when neither does. `extract_fonts` makes the same choice, so
/// an index into this list names the face the reader sees.
fn reader_font_set(root: &mut EngineValue) -> &mut EngineValue {
    let container = ["ResourceDict", "DocumentResources"]
        .into_iter()
        .find(|name| get(root, name).and_then(|c| get(c, "FontSet")).is_some())
        .unwrap_or("ResourceDict");
    let set = child(child(root, container), "FontSet");
    if !matches!(set, EngineValue::List(_)) {
        *set = EngineValue::List(Vec::new());
    }
    set
}

/// The index of the face in the reader's `FontSet`, appending a new entry when
/// it is absent so the run can always name its font. An entry matches on either
/// its `/Name` (the PostScript name) or its `/FontFamily`, so a reopened
/// `Name=ArialMT / FontFamily=Arial` set is reused for family `Arial` rather
/// than duplicated.
fn ensure_font(root: &mut EngineValue, name: &str, family: &str, style: &str) -> i64 {
    let set = reader_font_set(root);
    let EngineValue::List(items) = set else {
        unreachable!()
    };
    let matches = |entry: &EngineValue| {
        get(entry, "Name").and_then(as_str) == Some(name)
            || (!family.is_empty() && get(entry, "FontFamily").and_then(as_str) == Some(family))
    };
    if let Some(index) = items.iter().position(matches) {
        return index as i64;
    }
    let mut entry = EngineValue::Dict(Vec::new());
    set_child(&mut entry, "Name", EngineValue::String(name.to_string()));
    if !family.is_empty() {
        set_child(
            &mut entry,
            "FontFamily",
            EngineValue::String(family.to_string()),
        );
    }
    if !style.is_empty() {
        set_child(
            &mut entry,
            "FontStyle",
            EngineValue::String(style.to_string()),
        );
    }
    set_child(&mut entry, "Script", EngineValue::Int(0));
    set_child(&mut entry, "FontType", EngineValue::Int(1));
    set_child(&mut entry, "Synthetic", EngineValue::Int(0));
    let index = items.len() as i64;
    items.push(entry);
    index
}

fn dict_get_mut<'a>(value: &'a mut EngineValue, key: &str) -> Option<&'a mut EngineValue> {
    match value {
        EngineValue::Dict(items) => items
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

/// Get `parent[key]`, creating a dict entry when it is absent.
fn child<'a>(parent: &'a mut EngineValue, key: &str) -> &'a mut EngineValue {
    if !matches!(parent, EngineValue::Dict(_)) {
        *parent = EngineValue::Dict(Vec::new());
    }
    if dict_get_mut(parent, key).is_none() {
        if let EngineValue::Dict(items) = parent {
            items.push((key.to_string(), EngineValue::Dict(Vec::new())));
        }
    }
    dict_get_mut(parent, key).unwrap()
}

fn set_child(parent: &mut EngineValue, key: &str, value: EngineValue) {
    if let EngineValue::Dict(items) = parent {
        match items.iter_mut().find(|(k, _)| k == key) {
            Some((_, slot)) => *slot = value,
            None => items.push((key.to_string(), value)),
        }
    }
}

/// The first `RunArray` entry, creating the run when the array is absent.
fn first_entry(run: &mut EngineValue) -> &mut EngineValue {
    let list = child(run, "RunArray");
    if !matches!(list, EngineValue::List(_)) {
        *list = EngineValue::List(Vec::new());
    }
    let EngineValue::List(items) = list else {
        unreachable!()
    };
    if items.is_empty() {
        items.push(EngineValue::Dict(Vec::new()));
    }
    &mut items[0]
}

fn style_data_of(entry: &mut EngineValue) -> &mut EngineValue {
    child(child(entry, "StyleSheet"), "StyleSheetData")
}

fn paragraph_props_of(entry: &mut EngineValue) -> &mut EngineValue {
    child(child(entry, "ParagraphSheet"), "Properties")
}

fn first_existing<'a>(res: &'a mut EngineValue, key: &str) -> Option<&'a mut EngineValue> {
    match dict_get_mut(res, key) {
        Some(EngineValue::List(items)) => items.first_mut(),
        _ => None,
    }
}

/// The `StyleSheetData` of a `StyleSheetSet` entry, `None` when absent. The
/// entry either holds `/StyleSheetData` directly or nests it under
/// `/StyleSheet`.
fn set_style_data<'a>(res: &'a mut EngineValue, key: &str) -> Option<&'a mut EngineValue> {
    let entry = first_existing(res, key)?;
    if get(entry, "StyleSheetData").is_some() {
        return dict_get_mut(entry, "StyleSheetData");
    }
    dict_get_mut(entry, "StyleSheet").and_then(|sheet| dict_get_mut(sheet, "StyleSheetData"))
}

/// The `Properties` of a `ParagraphSheetSet` entry, `None` when absent; the
/// entry either holds `/Properties` directly or nests it under
/// `/ParagraphSheet`.
fn set_paragraph_props<'a>(res: &'a mut EngineValue, key: &str) -> Option<&'a mut EngineValue> {
    let entry = first_existing(res, key)?;
    if get(entry, "Properties").is_some() {
        return dict_get_mut(entry, "Properties");
    }
    dict_get_mut(entry, "ParagraphSheet").and_then(|sheet| dict_get_mut(sheet, "Properties"))
}

/// Assembles the from-scratch dump as raw bytes: string values are binary
/// UTF-16.
struct Dump {
    out: Vec<u8>,
}

impl Dump {
    fn line(&mut self, depth: usize, text: &str) {
        self.out.extend(std::iter::repeat_n(b'\t', depth));
        self.out.extend_from_slice(text.as_bytes());
        self.out.push(b'\n');
    }

    fn open(&mut self, depth: usize, name: &str) {
        self.line(depth, name);
        self.line(depth, "<<");
    }

    fn close(&mut self, depth: usize) {
        self.line(depth, ">>");
    }

    /// `key (…)` with the value as BOM-led UTF-16BE, `(`, `)` and `\` escaped.
    fn string(&mut self, depth: usize, key: &str, text: &str) {
        self.out.extend(std::iter::repeat_n(b'\t', depth));
        self.out.extend_from_slice(key.as_bytes());
        self.out.extend_from_slice(b" (\xFE\xFF");
        for unit in text.encode_utf16() {
            for byte in unit.to_be_bytes() {
                if matches!(byte, b'(' | b')' | b'\\') {
                    self.out.push(b'\\');
                }
                self.out.push(byte);
            }
        }
        self.out.extend_from_slice(b")\n");
    }
}

fn skeleton_engine_data(spec: &TypeSpec, font_name: Option<&str>) -> Vec<u8> {
    let body = engine_body(&spec.text);
    let font = font_name
        .map(str::to_owned)
        .unwrap_or_else(|| postscript_name(&spec.character.font_family));
    let mut d = Dump {
        out: b"\n\n".to_vec(),
    };
    d.line(0, "<<");
    d.open(1, "/EngineDict");
    d.open(2, "/Editor");
    d.string(3, "/Text", &body);
    d.close(2);
    paragraph_run(&mut d, spec, &body);
    style_run(&mut d, spec, &body);
    grid_info(&mut d);
    d.line(
        2,
        &format!("/AntiAlias {}", spec.character.anti_alias.index()),
    );
    d.line(
        2,
        &format!(
            "/UseFractionalGlyphWidths {}",
            spec.character.fractional_widths
        ),
    );
    if let Ok(shape) = encode_engine_data(&crate::type_shape::rendered(spec.box_bounds)) {
        d.line(2, &format!("/Rendered {}", String::from_utf8_lossy(&shape)));
    }
    d.close(1);
    // `ResourceDict` is what this layer uses, `DocumentResources` what the
    // document offers; identical is right for a lone text layer.
    for name in ["/ResourceDict", "/DocumentResources"] {
        d.open(1, name);
        resources(&mut d, spec, &font);
        d.close(1);
    }
    d.close(0);
    // An even length keeps the `TySh` block aligned without a padding byte.
    if d.out.len() % 2 == 1 {
        d.out.push(b'\n');
    }
    d.out
}

fn paragraph_run(d: &mut Dump, spec: &TypeSpec, body: &str) {
    d.open(2, "/ParagraphRun");
    d.open(3, "/DefaultRunData");
    paragraph_sheet(d, 4, spec);
    d.close(3);
    let lengths = paragraph_lengths(body);
    d.line(3, "/RunArray [");
    for _ in &lengths {
        d.line(4, "<<");
        paragraph_sheet(d, 5, spec);
        d.line(4, ">>");
    }
    d.line(3, "]");
    d.line(3, &format!("/RunLengthArray [ {} ]", join(&lengths)));
    d.line(3, "/IsJoinable 1");
    d.close(2);
}

fn paragraph_sheet(d: &mut Dump, depth: usize, spec: &TypeSpec) {
    let p = &spec.paragraph;
    let triplet = |v: [f64; 3]| format!("[ {} {} {} ]", number(v[0]), number(v[1]), number(v[2]));
    d.open(depth, "/ParagraphSheet");
    d.line(depth + 1, "/DefaultStyleSheet 0");
    d.open(depth + 1, "/Properties");
    for line in [
        format!("/Justification {}", p.justify.index()),
        format!("/FirstLineIndent {}", number(p.first_line_indent)),
        format!("/StartIndent {}", number(p.start_indent)),
        format!("/EndIndent {}", number(p.end_indent)),
        format!("/SpaceBefore {}", number(p.space_before)),
        format!("/SpaceAfter {}", number(p.space_after)),
        format!("/AutoHyphenate {}", p.hyphenate),
        format!("/HyphenatedWordSize {}", p.hyphenate_word_size),
        format!("/PreHyphen {}", p.hyphenate_pre),
        format!("/PostHyphen {}", p.hyphenate_post),
        format!("/ConsecutiveHyphens {}", p.hyphen_limit),
        format!("/Zone {}", number(p.hyphenation_zone)),
        format!("/WordSpacing {}", triplet(p.word_spacing)),
        format!("/LetterSpacing {}", triplet(p.letter_spacing)),
        format!("/GlyphSpacing {}", triplet(p.glyph_spacing)),
        format!("/AutoLeading {}", number(p.auto_leading / 100.0)),
        "/LeadingType 0".into(),
        format!("/Hanging {}", p.hanging),
        "/Burasagari false".into(),
        "/KinsokuOrder 0".into(),
        format!("/EveryLineComposer {}", p.composer == Composer::EveryLine),
    ] {
        d.line(depth + 2, &line);
    }
    d.close(depth + 1);
    d.close(depth);
    d.open(depth, "/Adjustments");
    d.line(depth + 1, "/Axis [ 1.0 0.0 1.0 ]");
    d.line(depth + 1, "/XY [ 0.0 0.0 ]");
    d.close(depth);
}

fn style_run(d: &mut Dump, spec: &TypeSpec, body: &str) {
    d.open(2, "/StyleRun");
    d.open(3, "/DefaultRunData");
    style_sheet(d, 4, spec, 0);
    d.close(3);
    d.line(3, "/RunArray [");
    d.line(4, "<<");
    style_sheet(d, 5, spec, 0);
    d.line(4, ">>");
    d.line(3, "]");
    d.line(
        3,
        &format!("/RunLengthArray [ {} ]", body.encode_utf16().count()),
    );
    d.line(3, "/IsJoinable 2");
    d.close(2);
}

/// One style sheet; the font set holds the spec's single font, so `/Font` is 0.
fn style_sheet(d: &mut Dump, depth: usize, spec: &TypeSpec, font_index: usize) {
    let c = &spec.character;
    let (auto_kerning, kerning) = match c.kerning {
        KerningMode::Manual(value) => (false, value),
        KerningMode::Metrics => (true, 0),
    };
    let mut lines = vec![
        format!("/Font {font_index}"),
        format!("/FontSize {}", number(c.size)),
    ];
    match c.leading {
        Leading::Auto => {
            lines.push("/AutoLeading true".into());
            lines.push(format!("/Leading {}", number(c.size * 1.2)));
        }
        Leading::Fixed(value) => {
            lines.push("/AutoLeading false".into());
            lines.push(format!("/Leading {}", number(value)));
        }
    }
    lines.push(format!(
        "/HorizontalScale {}",
        number(c.horizontal_scale / 100.0)
    ));
    lines.push(format!(
        "/VerticalScale {}",
        number(c.vertical_scale / 100.0)
    ));
    lines.push(format!("/Tracking {}", c.tracking.round() as i64));
    lines.push(format!("/BaselineShift {}", number(c.baseline_shift)));
    lines.push(format!("/AutoKerning {auto_kerning}"));
    lines.push(format!("/Kerning {kerning}"));
    lines.push(format!("/FontCaps {}", font_caps(c)));
    lines.push(format!("/FontBaseline {}", font_baseline(c)));
    lines.push(format!("/Underline {}", c.underline));
    lines.push(format!("/Strikethrough {}", c.strikethrough));
    lines.push(format!("/FauxBold {}", c.faux_bold));
    lines.push(format!("/FauxItalic {}", c.faux_italic));
    lines.push(format!("/Ligatures {}", c.standard_ligatures));
    lines.push(format!("/ContextualLigatures {}", c.contextual_alternates));
    lines.push(format!(
        "/DiscretionaryLigatures {}",
        c.discretionary_ligatures
    ));
    lines.push(format!("/Swash {}", c.swash));
    lines.push(format!("/OldStyle {}", c.oldstyle));
    lines.push(format!("/StylisticAlternates {}", c.stylistic_alternates));
    lines.push(format!("/TitlingAlternates {}", c.titling_alternates));
    lines.push(format!("/Ornaments {}", c.ornaments));
    lines.push(format!("/Ordinals {}", c.ordinals));
    lines.push(format!("/Fractions {}", c.fractions));
    lines.push("/StyleRunAlignment 2".into());
    lines.push("/NoBreak false".into());
    d.open(depth, "/StyleSheet");
    d.open(depth + 1, "/StyleSheetData");
    for line in lines {
        d.line(depth + 2, &line);
    }
    d.open(depth + 2, "/FillColor");
    // Type 1 is RGB; the values run alpha first, as fractions.
    d.line(depth + 3, "/Type 1");
    let [a, r, g, b] = c.fill_color.map(number);
    d.line(depth + 3, &format!("/Values [ {a} {r} {g} {b} ]"));
    d.close(depth + 2);
    d.line(depth + 2, "/FillFlag true");
    d.line(depth + 2, "/StrokeFlag false");
    d.close(depth + 1);
    d.close(depth);
}

fn grid_info(d: &mut Dump) {
    d.open(2, "/GridInfo");
    d.line(3, "/GridIsOn false");
    d.line(3, "/ShowGrid false");
    d.line(3, "/GridSize 18.0");
    d.line(3, "/GridLeading 22.0");
    for name in ["/GridColor", "/GridLeadingFillColor"] {
        d.open(3, name);
        d.line(4, "/Type 1");
        d.line(4, "/Values [ 0.0 0.0 0.0 1.0 ]");
        d.close(3);
    }
    d.line(3, "/AlignLineHeightToGridFlags false");
    d.close(2);
}

fn resources(d: &mut Dump, spec: &TypeSpec, font: &str) {
    // Empty rather than absent: East Asian line breaking is not offered, but
    // the keys are part of the shape Photoshop reads.
    d.line(2, "/KinsokuSet [");
    d.line(2, "]");
    d.line(2, "/MojiKumiSet [");
    d.line(2, "]");
    d.line(2, "/TheNormalStyleSheet 0");
    d.line(2, "/TheNormalParagraphSheet 0");
    d.line(2, "/ParagraphSheetSet [");
    d.line(3, "<<");
    d.string(4, "/Name", "Normal RGB");
    d.line(4, "/DefaultStyleSheet 0");
    paragraph_sheet(d, 4, spec);
    d.line(3, ">>");
    d.line(2, "]");
    d.line(2, "/StyleSheetSet [");
    d.line(3, "<<");
    d.string(4, "/Name", "Normal RGB");
    style_sheet(d, 4, spec, 0);
    d.line(3, ">>");
    d.line(2, "]");
    d.line(2, "/FontSet [");
    d.line(3, "<<");
    d.string(4, "/Name", font);
    d.string(4, "/FontFamily", &spec.character.font_family);
    d.string(4, "/FontStyle", &spec.character.font_style);
    d.line(4, "/Script 0");
    d.line(4, "/FontType 1");
    d.line(4, "/Synthetic 0");
    d.line(3, ">>");
    d.line(2, "]");
    d.line(2, "/SuperscriptSize .583");
    d.line(2, "/SuperscriptPosition .333");
    d.line(2, "/SubscriptSize .583");
    d.line(2, "/SubscriptPosition .333");
    d.line(2, "/SmallCapSize .7");
}

/// A family as one PostScript name: whitespace dropped, so `Times New Roman`
/// is `TimesNewRoman`. ponytail: a heuristic — a family whose real PostScript
/// name differs (`TimesNewRomanPSMT`) is substituted on reopening in Photoshop.
fn postscript_name(family: &str) -> String {
    let name: String = family.chars().filter(|c| !c.is_whitespace()).collect();
    if name.is_empty() {
        "Helvetica".to_string()
    } else {
        name
    }
}

/// Paragraph lengths in UTF-16 units, each including its closing `\r`, so they
/// add up to the whole body — the property Photoshop checks.
fn paragraph_lengths(body: &str) -> Vec<usize> {
    let mut lengths = Vec::new();
    let mut current = 0;
    for c in body.chars() {
        current += c.len_utf16();
        if c == '\r' {
            lengths.push(current);
            current = 0;
        }
    }
    if current > 0 || lengths.is_empty() {
        lengths.push(current);
    }
    lengths
}

fn join(values: &[usize]) -> String {
    values
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A real with its decimal point: an integer without one is another type to
/// the engine.
fn number(value: f64) -> String {
    // Four decimals: a colour channel is a multiple of 1/255, which two
    // decimals cannot carry back to the same byte.
    let text = format!("{value:.4}");
    let trimmed = text.trim_end_matches('0');
    if trimmed.ends_with('.') {
        format!("{trimmed}0")
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
#[path = "type_write/tests.rs"]
mod tests;
