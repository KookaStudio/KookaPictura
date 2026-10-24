//! Bounded parser for PSD EngineData, the opaque style blob inside a
//! text layer's `Txt ` descriptor.
//!
//! Grammar follows psd-tools 1.19 `psd_tools/psd/engine_data.py`: `<<`/`>>`
//! dicts, `[`/`]` lists, `/Name` properties, integers, decimals, `true`/`false`,
//! and parenthesised strings (UTF-16BE when they begin with `\xfe\xff`, MacRoman
//! otherwise). Malformed input returns [`PsdError::Invalid`]; the caller keeps
//! the document readable.

use pictura_core::{
    AntiAlias, CharacterAttrs, Composer, Justify, KerningMode, Leading, ParagraphAttrs,
    TextDirection, TextStyle, DEFAULT_LANGUAGE,
};

use crate::error::PsdError;

const MAX_DEPTH: u32 = 64;
const MAX_TOKENS: usize = 100_000;
const MAX_BYTES: usize = 16 * 1024 * 1024;

/// A parsed EngineData value. A dict keeps duplicate keys in order; lookup
/// takes the first match.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineValue {
    Dict(Vec<(String, EngineValue)>),
    List(Vec<EngineValue>),
    Int(i64),
    Double(f64),
    Bool(bool),
    String(String),
}

/// Parse an EngineData byte stream.
pub fn parse_engine_data(data: &[u8]) -> Result<EngineValue, PsdError> {
    if data.len() > MAX_BYTES {
        return Err(invalid("exceeds 16 MiB"));
    }
    let mut parser = Parser {
        data,
        pos: 0,
        tokens: 0,
    };
    parser.value(0)
}

/// Font-set names from the first `FontSet` list under `ResourceDict` (else
/// `DocumentResources`).
pub fn extract_fonts(root: &EngineValue) -> Vec<String> {
    let font_set = get(root, "ResourceDict")
        .and_then(|r| find_key(r, "FontSet"))
        .or_else(|| get(root, "DocumentResources").and_then(|r| find_key(r, "FontSet")));
    let Some(value) = font_set else {
        return Vec::new();
    };
    as_list(value)
        .iter()
        .filter_map(|entry| get(entry, "Name").and_then(as_str).map(str::to_owned))
        .collect()
}

/// First style run's effective character attributes and first paragraph run's
/// effective paragraph attributes, resolved against the style-sheet and
/// paragraph-sheet defaults.
pub fn extract_style(root: &EngineValue, fonts: &[String]) -> TextStyle {
    let run = path(root, &["EngineDict", "StyleRun", "RunArray"])
        .map(as_list)
        .and_then(|list| list.first())
        .and_then(|entry| path(entry, &["StyleSheet", "StyleSheetData"]))
        .filter(|v| is_dict(v));
    let char_default =
        style_sheet(root, "ResourceDict").or_else(|| style_sheet(root, "DocumentResources"));
    let paragraph = path(root, &["EngineDict", "ParagraphRun", "RunArray"])
        .map(as_list)
        .and_then(|list| list.first())
        .and_then(|entry| path(entry, &["ParagraphSheet", "Properties"]))
        .filter(|v| is_dict(v));
    let paragraph_default = paragraph_sheet(root, "ResourceDict")
        .or_else(|| paragraph_sheet(root, "DocumentResources"))
        .or_else(|| {
            path(
                root,
                &[
                    "EngineDict",
                    "ParagraphRun",
                    "DefaultRunData",
                    "ParagraphSheet",
                    "Properties",
                ],
            )
            .filter(|v| is_dict(v))
        });

    let pick_int = |key: &str| {
        run.and_then(|v| get(v, key))
            .and_then(as_i64)
            .or_else(|| char_default.and_then(|v| get(v, key)).and_then(as_i64))
    };
    let pick_num = |key: &str| {
        run.and_then(|v| get(v, key))
            .and_then(as_f64)
            .or_else(|| char_default.and_then(|v| get(v, key)).and_then(as_f64))
    };
    let pick_bool = |key: &str| {
        run.and_then(|v| get(v, key))
            .and_then(as_bool)
            .or_else(|| char_default.and_then(|v| get(v, key)).and_then(as_bool))
    };

    let font_index = pick_int("Font").filter(|index| *index >= 0);
    let font = font_index.and_then(|index| fonts.get(index as usize).cloned());
    let font_entry = font_index.and_then(|index| font_set_entry(root, index as usize));
    let font_family = font_entry
        .and_then(|entry| get(entry, "FontFamily"))
        .and_then(as_str)
        .map(str::to_owned)
        .or_else(|| font.clone())
        .unwrap_or_default();
    let font_style = font_entry
        .and_then(|entry| get(entry, "FontStyle"))
        .and_then(as_str)
        .unwrap_or_default()
        .to_string();

    let leading = if pick_bool("AutoLeading").unwrap_or(true) {
        Leading::Auto
    } else {
        Leading::Fixed(pick_num("Leading").unwrap_or(0.0))
    };
    let kerning = {
        let auto = pick_bool("AutoKerning")
            .or_else(|| pick_bool("AutoKern"))
            .unwrap_or(true);
        let manual = pick_int("Kerning").unwrap_or(0);
        if !auto && manual != 0 {
            KerningMode::Manual(manual as i32)
        } else {
            KerningMode::Metrics
        }
    };
    let font_caps = pick_int("FontCaps").unwrap_or(0);
    let font_baseline = pick_int("FontBaseline").unwrap_or(0);
    let fill_color = run
        .and_then(|v| get(v, "FillColor"))
        .and_then(|v| get(v, "Values"))
        .or_else(|| {
            char_default
                .and_then(|v| get(v, "FillColor"))
                .and_then(|v| get(v, "Values"))
        })
        .map(as_list)
        .filter(|values| values.len() >= 4)
        .map(|values| {
            [
                as_f64(&values[0]).unwrap_or(0.0),
                as_f64(&values[1]).unwrap_or(0.0),
                as_f64(&values[2]).unwrap_or(0.0),
                as_f64(&values[3]).unwrap_or(0.0),
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let anti_alias = path(root, &["EngineDict", "AntiAlias"])
        .and_then(as_bool)
        .map(|enabled| {
            if enabled {
                AntiAlias::Sharp
            } else {
                AntiAlias::None
            }
        })
        .unwrap_or_default();
    let fractional_widths = path(root, &["EngineDict", "UseFractionalGlyphWidths"])
        .and_then(as_bool)
        .unwrap_or(true);
    let direction = path(
        root,
        &["EngineDict", "Rendered", "Shapes", "WritingDirection"],
    )
    .and_then(as_i64)
    .map(|value| TextDirection::from_index(value as i32))
    .unwrap_or_default();

    let character = CharacterAttrs {
        font_family,
        font_style,
        size: pick_num("FontSize").unwrap_or(0.0),
        leading,
        kerning,
        tracking: pick_num("Tracking").unwrap_or(0.0),
        horizontal_scale: pick_num("HorizontalScale").unwrap_or(1.0) * 100.0,
        vertical_scale: pick_num("VerticalScale").unwrap_or(1.0) * 100.0,
        baseline_shift: pick_num("BaselineShift").unwrap_or(0.0),
        tsume: pick_num("Tsume").unwrap_or(0.0),
        anti_alias,
        fill_color,
        all_caps: font_caps == 2,
        small_caps: font_caps == 1,
        superscript: font_baseline == 1,
        subscript: font_baseline == 2,
        underline: pick_bool("Underline").unwrap_or(false),
        strikethrough: pick_bool("Strikethrough").unwrap_or(false),
        fractional_widths,
        faux_bold: pick_bool("FauxBold").unwrap_or(false),
        faux_italic: pick_bool("FauxItalic").unwrap_or(false),
        standard_ligatures: pick_bool("Ligatures").unwrap_or(true),
        contextual_alternates: pick_bool("ContextualLigatures").unwrap_or(true),
        discretionary_ligatures: pick_bool("DiscretionaryLigatures").unwrap_or(false),
        swash: pick_bool("Swash").unwrap_or(false),
        oldstyle: pick_bool("OldStyle").unwrap_or(false),
        stylistic_alternates: pick_bool("StylisticAlternates").unwrap_or(false),
        titling_alternates: pick_bool("TitlingAlternates").unwrap_or(false),
        ornaments: pick_bool("Ornaments").unwrap_or(false),
        ordinals: pick_bool("Ordinals").unwrap_or(false),
        fractions: pick_bool("Fractions").unwrap_or(false),
        language: run
            .and_then(|v| get(v, "Language"))
            .or_else(|| char_default.and_then(|v| get(v, "Language")))
            .and_then(as_str)
            .unwrap_or(DEFAULT_LANGUAGE)
            .to_string(),
        vertical_roman_alignment: pick_bool("VerticalRomanAlignment").unwrap_or(true),
    };

    let p_int = |key: &str| {
        paragraph
            .and_then(|v| get(v, key))
            .and_then(as_i64)
            .or_else(|| paragraph_default.and_then(|v| get(v, key)).and_then(as_i64))
    };
    let p_num = |key: &str| {
        paragraph
            .and_then(|v| get(v, key))
            .and_then(as_f64)
            .or_else(|| paragraph_default.and_then(|v| get(v, key)).and_then(as_f64))
    };
    let p_bool = |key: &str| {
        paragraph
            .and_then(|v| get(v, key))
            .and_then(as_bool)
            .or_else(|| {
                paragraph_default
                    .and_then(|v| get(v, key))
                    .and_then(as_bool)
            })
    };
    let p_triplet = |key: &str, default: [f64; 3]| {
        paragraph
            .and_then(|v| get(v, key))
            .or_else(|| paragraph_default.and_then(|v| get(v, key)))
            .map(as_list)
            .filter(|values| values.len() >= 3)
            .map(|values| {
                [
                    as_f64(&values[0]).unwrap_or(default[0]),
                    as_f64(&values[1]).unwrap_or(default[1]),
                    as_f64(&values[2]).unwrap_or(default[2]),
                ]
            })
            .unwrap_or(default)
    };

    let paragraph = ParagraphAttrs {
        justify: Justify::from_index(p_int("Justification").unwrap_or(0) as u8).unwrap_or_default(),
        direction,
        word_spacing: p_triplet("WordSpacing", [0.8, 1.0, 1.33]),
        letter_spacing: p_triplet("LetterSpacing", [0.0, 0.0, 0.0]),
        glyph_spacing: p_triplet("GlyphSpacing", [1.0, 1.0, 1.0]),
        start_indent: p_num("StartIndent").unwrap_or(0.0),
        end_indent: p_num("EndIndent").unwrap_or(0.0),
        first_line_indent: p_num("FirstLineIndent").unwrap_or(0.0),
        space_before: p_num("SpaceBefore").unwrap_or(0.0),
        space_after: p_num("SpaceAfter").unwrap_or(0.0),
        hanging: p_bool("Hanging").unwrap_or(false),
        hyphenate: p_bool("AutoHyphenate").unwrap_or(false),
        composer: if p_bool("EveryLineComposer").unwrap_or(false) {
            Composer::EveryLine
        } else {
            Composer::SingleLine
        },
        auto_leading: p_num("AutoLeading").map(|v| v * 100.0).unwrap_or(120.0),
        hyphenate_word_size: p_int("HyphenatedWordSize").unwrap_or(5) as i32,
        hyphenate_pre: p_int("PreHyphen").unwrap_or(2) as i32,
        hyphenate_post: p_int("PostHyphen").unwrap_or(2) as i32,
        hyphen_limit: p_int("ConsecutiveHyphens").unwrap_or(2) as i32,
        hyphenation_zone: p_num("Zone").unwrap_or(36.0),
        hyphenate_caps: p_bool("HyphenateCaps").unwrap_or(true),
    };

    TextStyle {
        font,
        character,
        paragraph,
        applied_character_style: None,
        applied_paragraph_style: None,
    }
}

/// The `FontSet` entry at `index`, from `ResourceDict` else `DocumentResources`.
fn font_set_entry(root: &EngineValue, index: usize) -> Option<&EngineValue> {
    ["ResourceDict", "DocumentResources"]
        .iter()
        .find_map(|container| {
            get(root, container)
                .and_then(|c| get(c, "FontSet"))
                .and_then(|set| as_list(set).get(index))
        })
}

fn style_sheet<'a>(root: &'a EngineValue, container: &str) -> Option<&'a EngineValue> {
    get(root, container)
        .and_then(|c| get(c, "StyleSheetSet"))
        .map(as_list)
        .and_then(|list| list.first())
        .and_then(|entry| {
            get(entry, "StyleSheetData").or_else(|| path(entry, &["StyleSheet", "StyleSheetData"]))
        })
        .filter(|v| is_dict(v))
}

fn paragraph_sheet<'a>(root: &'a EngineValue, container: &str) -> Option<&'a EngineValue> {
    get(root, container)
        .and_then(|c| get(c, "ParagraphSheetSet"))
        .map(as_list)
        .and_then(|list| list.first())
        .and_then(|entry| {
            get(entry, "Properties").or_else(|| path(entry, &["ParagraphSheet", "Properties"]))
        })
        .filter(|v| is_dict(v))
}

fn invalid(message: &str) -> PsdError {
    PsdError::Invalid(format!("EngineData: {message}"))
}

fn is_dict(value: &EngineValue) -> bool {
    matches!(value, EngineValue::Dict(_))
}

pub(crate) fn get<'a>(value: &'a EngineValue, key: &str) -> Option<&'a EngineValue> {
    match value {
        EngineValue::Dict(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

pub(crate) fn path<'a>(value: &'a EngineValue, keys: &[&str]) -> Option<&'a EngineValue> {
    keys.iter()
        .try_fold(value, |current, key| get(current, key))
}

fn find_key<'a>(value: &'a EngineValue, key: &str) -> Option<&'a EngineValue> {
    match value {
        EngineValue::Dict(items) => items
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .or_else(|| items.iter().find_map(|(_, v)| find_key(v, key))),
        EngineValue::List(items) => items.iter().find_map(|v| find_key(v, key)),
        _ => None,
    }
}

pub(crate) fn as_list(value: &EngineValue) -> &[EngineValue] {
    match value {
        EngineValue::List(items) => items,
        _ => &[],
    }
}

pub(crate) fn as_str(value: &EngineValue) -> Option<&str> {
    match value {
        EngineValue::String(s) => Some(s),
        _ => None,
    }
}

fn as_bool(value: &EngineValue) -> Option<bool> {
    match value {
        EngineValue::Bool(b) => Some(*b),
        EngineValue::Int(i) => Some(*i != 0),
        _ => None,
    }
}

fn as_f64(value: &EngineValue) -> Option<f64> {
    match value {
        EngineValue::Double(d) => Some(*d),
        EngineValue::Int(i) => Some(*i as f64),
        _ => None,
    }
}

fn as_i64(value: &EngineValue) -> Option<i64> {
    match value {
        EngineValue::Int(i) => Some(*i),
        EngineValue::Double(d) => Some(*d as i64),
        _ => None,
    }
}

struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
    tokens: usize,
}

impl<'a> Parser<'a> {
    fn value(&mut self, depth: u32) -> Result<EngineValue, PsdError> {
        if depth > MAX_DEPTH {
            return Err(invalid("nesting too deep"));
        }
        self.skip_ws();
        self.count()?;
        if self.starts_with(b"<<") {
            self.pos += 2;
            return self.dict(depth);
        }
        if self.starts_with(b"[") {
            self.pos += 1;
            return self.list(depth);
        }
        match self.peek() {
            Some(b'(') => self.string(),
            Some(b'/') => Err(invalid("unexpected property")),
            Some(_) => {
                let word = self.word();
                self.classify(word)
            }
            None => Err(invalid("truncated")),
        }
    }

    fn dict(&mut self, depth: u32) -> Result<EngineValue, PsdError> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => return Err(invalid("truncated dict")),
                Some(b'>') if self.starts_with(b">>") => {
                    self.pos += 2;
                    self.count()?;
                    return Ok(EngineValue::Dict(items));
                }
                Some(b'/') => {
                    self.count()?;
                    self.pos += 1;
                    let name = self.name();
                    let value = self.value(depth + 1)?;
                    items.push((name, value));
                }
                Some(_) => return Err(invalid("expected property")),
            }
        }
    }

    fn list(&mut self, depth: u32) -> Result<EngineValue, PsdError> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => return Err(invalid("truncated list")),
                Some(b']') => {
                    self.pos += 1;
                    self.count()?;
                    return Ok(EngineValue::List(items));
                }
                Some(_) => items.push(self.value(depth + 1)?),
            }
        }
    }

    fn string(&mut self) -> Result<EngineValue, PsdError> {
        let start = self.pos + 1;
        let mut i = start;
        let end = loop {
            match self.data.get(i) {
                None => return Err(invalid("unterminated string")),
                Some(b'\\') if i + 1 < self.data.len() => i += 2,
                Some(b'\\') => return Err(invalid("dangling escape")),
                Some(b')') => break i,
                Some(_) => i += 1,
            }
        };
        self.pos = end + 1;
        Ok(EngineValue::String(decode_string(&self.data[start..end])))
    }

    fn name(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        decode_macroman(&self.data[start..self.pos])
    }

    fn word(&mut self) -> &'a [u8] {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if matches!(b, b' ' | b'\n' | b'\t' | b'[' | b']' | b'<' | b'>') {
                break;
            }
            self.pos += 1;
        }
        &self.data[start..self.pos]
    }

    fn classify(&mut self, word: &[u8]) -> Result<EngineValue, PsdError> {
        match word {
            b"true" => Ok(EngineValue::Bool(true)),
            b"false" => Ok(EngineValue::Bool(false)),
            _ => {
                let text = std::str::from_utf8(word).map_err(|_| invalid("unknown token"))?;
                if is_integer(text) {
                    text.parse::<i64>()
                        .map(EngineValue::Int)
                        .map_err(|_| invalid("integer overflow"))
                } else if is_decimal(text) {
                    text.parse::<f64>()
                        .map(EngineValue::Double)
                        .map_err(|_| invalid("bad decimal"))
                } else {
                    Err(invalid("unknown token"))
                }
            }
        }
    }

    fn count(&mut self) -> Result<(), PsdError> {
        self.tokens += 1;
        if self.tokens > MAX_TOKENS {
            return Err(invalid("token cap"));
        }
        Ok(())
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\t')) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        self.data[self.pos..].starts_with(prefix)
    }
}

fn is_integer(text: &str) -> bool {
    let digits = strip_sign(text.as_bytes());
    !digits.is_empty() && digits.iter().all(u8::is_ascii_digit)
}

fn is_decimal(text: &str) -> bool {
    let bytes = strip_sign(text.as_bytes());
    let Some(dot) = bytes.iter().position(|&b| b == b'.') else {
        return false;
    };
    let (whole, fraction) = (&bytes[..dot], &bytes[dot + 1..]);
    whole.iter().all(u8::is_ascii_digit)
        && !fraction.is_empty()
        && fraction.iter().all(u8::is_ascii_digit)
}

fn strip_sign(bytes: &[u8]) -> &[u8] {
    if bytes.first() == Some(&b'-') {
        &bytes[1..]
    } else {
        bytes
    }
}

fn decode_string(payload: &[u8]) -> String {
    let unescaped = unescape(payload);
    if let Some(body) = unescaped.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = body
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        decode_macroman(&unescaped)
    }
}

fn unescape(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len());
    let mut i = 0;
    while i < payload.len() {
        if payload[i] == b'\\' && matches!(payload.get(i + 1), Some(b'\\' | b'(' | b')')) {
            out.push(payload[i + 1]);
            i += 2;
        } else {
            out.push(payload[i]);
            i += 1;
        }
    }
    out
}

fn decode_macroman(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b < 0x80 {
                b as char
            } else {
                MAC_ROMAN_HIGH[(b - 0x80) as usize]
            }
        })
        .collect()
}

const MAC_ROMAN_HIGH: [char; 128] = [
    '\u{00C4}', '\u{00C5}', '\u{00C7}', '\u{00C9}', '\u{00D1}', '\u{00D6}', '\u{00DC}', '\u{00E1}',
    '\u{00E0}', '\u{00E2}', '\u{00E4}', '\u{00E3}', '\u{00E5}', '\u{00E7}', '\u{00E9}', '\u{00E8}',
    '\u{00EA}', '\u{00EB}', '\u{00ED}', '\u{00EC}', '\u{00EE}', '\u{00EF}', '\u{00F1}', '\u{00F3}',
    '\u{00F2}', '\u{00F4}', '\u{00F6}', '\u{00F5}', '\u{00FA}', '\u{00F9}', '\u{00FB}', '\u{00FC}',
    '\u{2020}', '\u{00B0}', '\u{00A2}', '\u{00A3}', '\u{00A7}', '\u{2022}', '\u{00B6}', '\u{00DF}',
    '\u{00AE}', '\u{00A9}', '\u{2122}', '\u{00B4}', '\u{00A8}', '\u{2260}', '\u{00C6}', '\u{00D8}',
    '\u{221E}', '\u{00B1}', '\u{2264}', '\u{2265}', '\u{00A5}', '\u{00B5}', '\u{2202}', '\u{2211}',
    '\u{220F}', '\u{03C0}', '\u{222B}', '\u{00AA}', '\u{00BA}', '\u{03A9}', '\u{00E6}', '\u{00F8}',
    '\u{00BF}', '\u{00A1}', '\u{00AC}', '\u{221A}', '\u{0192}', '\u{2248}', '\u{2206}', '\u{00AB}',
    '\u{00BB}', '\u{2026}', '\u{00A0}', '\u{00C0}', '\u{00C3}', '\u{00D5}', '\u{0152}', '\u{0153}',
    '\u{2013}', '\u{2014}', '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}', '\u{00F7}', '\u{25CA}',
    '\u{00FF}', '\u{0178}', '\u{2044}', '\u{20AC}', '\u{2039}', '\u{203A}', '\u{FB01}', '\u{FB02}',
    '\u{2021}', '\u{00B7}', '\u{201A}', '\u{201E}', '\u{2030}', '\u{00C2}', '\u{00CA}', '\u{00C1}',
    '\u{00CB}', '\u{00C8}', '\u{00CD}', '\u{00CE}', '\u{00CF}', '\u{00CC}', '\u{00D3}', '\u{00D4}',
    '\u{F8FF}', '\u{00D2}', '\u{00DA}', '\u{00DB}', '\u{00D9}', '\u{0131}', '\u{02C6}', '\u{02DC}',
    '\u{00AF}', '\u{02D8}', '\u{02D9}', '\u{02DA}', '\u{00B8}', '\u{02DD}', '\u{02DB}', '\u{02C7}',
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_tree_exposes_each_value() {
        let root =
            parse_engine_data(b"<< /n -7 /d 1.5 /b true /s (hi) /l [ 1 2.0 ] >>").expect("parses");
        let EngineValue::Dict(items) = &root else {
            panic!("dict");
        };
        let keys: Vec<&str> = items.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["n", "d", "b", "s", "l"]);
        assert_eq!(get(&root, "n"), Some(&EngineValue::Int(-7)));
        assert_eq!(get(&root, "d"), Some(&EngineValue::Double(1.5)));
        assert_eq!(get(&root, "b"), Some(&EngineValue::Bool(true)));
        assert_eq!(get(&root, "s"), Some(&EngineValue::String("hi".into())));
        let Some(EngineValue::List(list)) = get(&root, "l") else {
            panic!("list");
        };
        assert_eq!(list, &[EngineValue::Int(1), EngineValue::Double(2.0)]);
    }

    #[test]
    fn utf16_and_escapes_decode() {
        let mut data = b"<< /t (".to_vec();
        data.extend_from_slice(&[0xFE, 0xFF]);
        data.extend_from_slice(
            &"he\u{00e9}"
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect::<Vec<_>>(),
        );
        data.extend_from_slice(b") >>");
        let root = parse_engine_data(&data).expect("parses");
        assert_eq!(get(&root, "t"), Some(&EngineValue::String("heé".into())));

        let root = parse_engine_data(br"<< /t (a\(b\)c\\d) >>").expect("parses");
        assert_eq!(
            get(&root, "t"),
            Some(&EngineValue::String("a(b)c\\d".into()))
        );
    }

    #[test]
    fn unknown_parenthesised_tag_is_a_string() {
        let root = parse_engine_data(b"<< /t (hwid) >>").expect("parses");
        assert_eq!(get(&root, "t"), Some(&EngineValue::String("hwid".into())));
    }

    #[test]
    fn malformed_streams_error_without_panic() {
        assert!(parse_engine_data(b"").is_err(), "empty");
        assert!(parse_engine_data(b"<< /a").is_err(), "truncated dict");
        assert!(parse_engine_data(b"[ 1 2").is_err(), "truncated list");
        assert!(parse_engine_data(b"<< /a ? >>").is_err(), "unknown token");
        assert!(parse_engine_data(b"(unterminated").is_err(), "string");
        let deep = format!("{}{}", "[ ".repeat(80), "] ".repeat(80));
        assert!(parse_engine_data(deep.as_bytes()).is_err(), "depth cap");
    }

    #[test]
    fn extraction_resolves_run_against_defaults() {
        let root = parse_engine_data(
            b"<< /EngineDict << /StyleRun << /RunArray [ << /StyleSheet << \
              /StyleSheetData << /FontSize 150.0 /FillColor << /Values [ 1 1 1 1 ] >> >> \
              >> >> ] >> >> /ResourceDict << \
              /FontSet [ << /Name (Invis) >> << /Name (Myriad) >> ] \
              /StyleSheetSet [ << /StyleSheetData << /Font 1 /Tracking .25 >> >> ] >> >>",
        )
        .expect("parses");
        let fonts = extract_fonts(&root);
        assert_eq!(fonts, ["Invis", "Myriad"]);
        let style = extract_style(&root, &fonts);
        assert_eq!(style.font.as_deref(), Some("Myriad"));
        assert_eq!(style.character.size, 150.0);
        assert_eq!(style.character.fill_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(style.character.tracking, 0.25);
        assert_eq!(style.paragraph.justify, Justify::Left);
    }

    #[test]
    fn run_omissions_resolve_to_sheet_and_paragraph_defaults() {
        let root = parse_engine_data(
            b"<< /EngineDict << >> /ResourceDict << \
              /StyleSheetSet [ << /StyleSheetData << /Tracking 40 >> >> ] \
              /ParagraphSheetSet [ << /Properties << /SpaceAfter 12.0 /FirstLineIndent 24.0 >> >> ] >> >>",
        )
        .expect("parses");
        let style = extract_style(&root, &[]);
        assert_eq!(style.character.tracking, 40.0);
        assert_eq!(style.paragraph.space_after, 12.0);
        assert_eq!(style.paragraph.first_line_indent, 24.0);
    }

    #[test]
    fn anti_alias_flag_is_the_fallback() {
        let root = parse_engine_data(b"<< /EngineDict << /AntiAlias 1 >> >>").expect("parses");
        assert_eq!(
            extract_style(&root, &[]).character.anti_alias,
            AntiAlias::Sharp
        );
        let root = parse_engine_data(b"<< /EngineDict << /AntiAlias 0 >> >>").expect("parses");
        assert_eq!(
            extract_style(&root, &[]).character.anti_alias,
            AntiAlias::None
        );
        let root = parse_engine_data(b"<< /EngineDict << /AntiAlias false >> >>").expect("parses");
        assert_eq!(
            extract_style(&root, &[]).character.anti_alias,
            AntiAlias::None
        );
        let root = parse_engine_data(b"<< /EngineDict << /AntiAlias true >> >>").expect("parses");
        assert_eq!(
            extract_style(&root, &[]).character.anti_alias,
            AntiAlias::Sharp
        );
    }

    #[test]
    fn missing_style_data_uses_fallbacks() {
        let root = parse_engine_data(b"<< >>").expect("parses");
        assert!(extract_fonts(&root).is_empty());
        let style = extract_style(&root, &[]);
        assert_eq!(style.font, None);
        assert_eq!(style.character.size, 0.0);
        assert_eq!(style.character.fill_color, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(style.character.tracking, 0.0);
        assert_eq!(style.paragraph.justify, Justify::Left);
        assert_eq!(style.paragraph.word_spacing, [0.8, 1.0, 1.33]);
    }

    #[test]
    fn paragraph_justification_reads_every_modelled_index() {
        for (index, expected) in [
            (0, Justify::Left),
            (1, Justify::Right),
            (2, Justify::Center),
            (3, Justify::JustifyLastLeft),
            (4, Justify::JustifyLastRight),
            (5, Justify::JustifyLastCenter),
            (6, Justify::JustifyAll),
            (7, Justify::Left),
        ] {
            let source = format!(
                "<< /EngineDict << /ParagraphRun << /RunArray [ << /ParagraphSheet << \
                 /Properties << /Justification {index} >> >> >> ] >> >> >>"
            );
            let root = parse_engine_data(source.as_bytes()).expect("parses");
            assert_eq!(extract_style(&root, &[]).paragraph.justify, expected);
        }
    }

    #[test]
    fn extended_character_and_paragraph_keys_are_read() {
        let root = parse_engine_data(
            b"<< /EngineDict << /StyleRun << /RunArray [ << /StyleSheet << /StyleSheetData << \
              /FauxBold true /FauxItalic true /Ligatures false /ContextualLigatures false \
              /Fractions true /Language (French) >> >> >> ] >> \
              /ParagraphRun << /RunArray [ << /ParagraphSheet << /Properties << \
              /AutoLeading 1.5 /HyphenatedWordSize 8 /PreHyphen 3 /PostHyphen 4 \
              /ConsecutiveHyphens 5 /Zone 24.0 >> >> >> ] >> >> >>",
        )
        .expect("parses");
        let style = extract_style(&root, &[]);
        assert!(style.character.faux_bold);
        assert!(style.character.faux_italic);
        assert!(!style.character.standard_ligatures);
        assert!(!style.character.contextual_alternates);
        assert!(style.character.fractions);
        assert_eq!(style.character.language, "French");
        assert_eq!(style.paragraph.auto_leading, 150.0);
        assert_eq!(style.paragraph.hyphenate_word_size, 8);
        assert_eq!(style.paragraph.hyphenate_pre, 3);
        assert_eq!(style.paragraph.hyphenate_post, 4);
        assert_eq!(style.paragraph.hyphen_limit, 5);
        assert_eq!(style.paragraph.hyphenation_zone, 24.0);
    }

    #[test]
    fn caps_are_enforced() {
        let many = format!("<< {} >>", "/a 0 ".repeat(100_001));
        assert!(parse_engine_data(many.as_bytes()).is_err(), "token cap");
        let huge = vec![b' '; MAX_BYTES + 1];
        assert!(parse_engine_data(&huge).is_err(), "byte cap");
    }

    #[test]
    fn macroman_high_bytes_decode() {
        assert_eq!(decode_macroman(&[0x80, 0xA9]), "Ä©");
    }

    #[test]
    fn language_falls_back_to_sheet_default() {
        let root = parse_engine_data(
            b"<< /EngineDict << /StyleRun << /RunArray [ << /StyleSheet << \
              /StyleSheetData << /FontSize 12.0 >> >> >> ] >> >> /ResourceDict << \
              /StyleSheetSet [ << /StyleSheetData << /Language (German) >> >> ] >> >>",
        )
        .expect("parses");
        assert_eq!(extract_style(&root, &[]).character.language, "German");
    }

    #[test]
    fn tsume_language_and_direction_decode() {
        let root = parse_engine_data(
            b"<< /EngineDict << /StyleRun << /RunArray [ << /StyleSheet << \
              /StyleSheetData << /Tsume 60.0 /Language (French) >> >> >> ] >> \
              /Rendered << /Shapes << /WritingDirection 1 >> >> >> >>",
        )
        .expect("parses");
        let style = extract_style(&root, &[]);
        assert_eq!(style.character.tsume, 60.0);
        assert_eq!(style.character.language, "French");
        assert_eq!(style.paragraph.direction, TextDirection::Rtl);
    }

    #[test]
    fn missing_tsume_and_direction_take_defaults() {
        let root = parse_engine_data(b"<< /EngineDict << >> >>").expect("parses");
        let style = extract_style(&root, &[]);
        assert_eq!(style.character.tsume, 0.0);
        assert_eq!(style.paragraph.direction, TextDirection::Ltr);
    }
}
