//! `.joboptions` (Distiller parameter) files: a tolerant reader for Acrobat's
//! own files and the canonical writer the presets, the session, and the app
//! bridge share (design D3). Distiller key names are used where one exists;
//! Photoshop-only settings use `Pictura*` keys.

use super::object::real;
use super::options::{
    Changes, ColorConversion, Compatibility, Compression, Destination, Downsample, PdfOptions,
    Printing, ProfileInclusion, Quality, Standard,
};

/// A parsed PostScript-style value.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Num(f64),
    Bool(bool),
    Name(String),
    Str(String),
    Array(Vec<Value>),
    Dict(Vec<(String, Value)>),
    /// A bare operator or other word (`setdistillerparams`, `null`, …).
    Word(String),
}

impl Value {
    fn num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    fn flag(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Value::Name(n) => Some(n),
            _ => None,
        }
    }

    /// A string, or a localized-string dictionary's English (else first)
    /// entry, as Acrobat writes `/Description`.
    fn text(&self) -> Option<String> {
        match self {
            Value::Str(s) => Some(s.clone()),
            Value::Dict(entries) => entries
                .iter()
                .find(|(k, _)| k == "ENU")
                .or_else(|| entries.first())
                .and_then(|(_, v)| v.text()),
            _ => None,
        }
    }
}

struct Lexer<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Lexer<'_> {
    fn skip_space(&mut self) {
        while let Some(&b) = self.bytes.get(self.at) {
            if b == b'%' {
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|&c| c != b'\n' && c != b'\r')
                {
                    self.at += 1;
                }
            } else if b.is_ascii_whitespace() || b == 0 {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn starts(&self, text: &[u8]) -> bool {
        self.bytes[self.at..].starts_with(text)
    }

    fn is_delimiter(b: u8) -> bool {
        b.is_ascii_whitespace() || b"()<>[]{}/%".contains(&b)
    }

    fn word(&mut self) -> String {
        let start = self.at;
        while self.peek().is_some_and(|b| !Self::is_delimiter(b)) {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.bytes[start..self.at]).into_owned()
    }

    /// The next value, or `None` at the end or on a closing delimiter.
    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > 64 {
            return None;
        }
        self.skip_space();
        let b = self.peek()?;
        if self.starts(b"<<") {
            self.at += 2;
            let mut entries = Vec::new();
            loop {
                self.skip_space();
                if self.starts(b">>") || self.peek().is_none() {
                    self.at = (self.at + 2).min(self.bytes.len());
                    return Some(Value::Dict(entries));
                }
                match self.value(depth + 1) {
                    Some(Value::Name(key)) => {
                        if let Some(value) = self.value(depth + 1) {
                            entries.push((key, value));
                        }
                    }
                    Some(_) => {}
                    None => self.at += 1,
                }
            }
        }
        match b {
            b'[' | b'{' => {
                let close = if b == b'[' { b']' } else { b'}' };
                self.at += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    match self.peek() {
                        None => return Some(Value::Array(items)),
                        Some(c) if c == close => {
                            self.at += 1;
                            return Some(Value::Array(items));
                        }
                        _ => match self.value(depth + 1) {
                            Some(v) => items.push(v),
                            None => self.at += 1,
                        },
                    }
                }
            }
            b'(' => Some(Value::Str(decode_text(&self.literal()))),
            b'<' => Some(Value::Str(decode_text(&self.hex()))),
            b'/' => {
                self.at += 1;
                Some(Value::Name(self.word()))
            }
            b')' | b'>' | b']' | b'}' => None,
            _ => {
                let word = self.word();
                if word.is_empty() {
                    self.at += 1;
                    return Some(Value::Word(String::new()));
                }
                Some(match word.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => word
                        .parse::<f64>()
                        .map(Value::Num)
                        .unwrap_or(Value::Word(word)),
                })
            }
        }
    }

    fn literal(&mut self) -> Vec<u8> {
        self.at += 1;
        let mut out = Vec::new();
        let mut nesting = 0;
        while let Some(b) = self.peek() {
            self.at += 1;
            match b {
                b'\\' => {
                    let Some(next) = self.peek() else { break };
                    self.at += 1;
                    match next {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'0'..=b'7' => {
                            let mut code = u32::from(next - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        code = code * 8 + u32::from(d - b'0');
                                        self.at += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(code as u8);
                        }
                        b'\n' | b'\r' => {}
                        other => out.push(other),
                    }
                }
                b'(' => {
                    nesting += 1;
                    out.push(b);
                }
                b')' if nesting == 0 => break,
                b')' => {
                    nesting -= 1;
                    out.push(b);
                }
                _ => out.push(b),
            }
        }
        out
    }

    fn hex(&mut self) -> Vec<u8> {
        self.at += 1;
        let mut digits = Vec::new();
        while let Some(b) = self.peek() {
            self.at += 1;
            if b == b'>' {
                break;
            }
            if let Some(d) = (b as char).to_digit(16) {
                digits.push(d as u8);
            }
        }
        if digits.len() % 2 == 1 {
            digits.push(0);
        }
        digits.chunks(2).map(|p| p[0] << 4 | p[1]).collect()
    }
}

/// UTF-16BE behind a byte-order mark, else Latin-1.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xfe, 0xff]) {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|p| u16::from_be_bytes([p[0], *p.get(1).unwrap_or(&0)]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// The Distiller parameters of a `.joboptions` file: every dictionary passed
/// to `setdistillerparams`, merged in order (or the first top-level
/// dictionary when there is no such call). `None` when there is none.
fn distiller_params(text: &str) -> Option<Vec<(String, Value)>> {
    let mut lexer = Lexer {
        bytes: text.as_bytes(),
        at: 0,
    };
    let mut values = Vec::new();
    while lexer.peek().is_some() {
        match lexer.value(0) {
            Some(v) => values.push(v),
            None => lexer.at += 1,
        }
        lexer.skip_space();
    }
    let mut merged = Vec::new();
    let mut found = false;
    for pair in values.windows(2) {
        if let [Value::Dict(entries), Value::Word(op)] = pair {
            if op == "setdistillerparams" {
                merged.extend(entries.iter().cloned());
                found = true;
            }
        }
    }
    if !found {
        merged = values.into_iter().find_map(|v| match v {
            Value::Dict(entries) => Some(entries),
            _ => None,
        })?;
    }
    Some(merged)
}

/// Options from a `.joboptions` file, starting from `[Photoshop Default]` and
/// overriding every key the file sets; unknown keys are ignored. `None` when
/// the text holds no parameter dictionary.
pub fn parse_joboptions(text: &str) -> Option<PdfOptions> {
    let params = distiller_params(text)?;
    let mut o = PdfOptions::default();
    let mut downsample_on = None;
    let mut downsample_kind = None;
    let mut threshold = None;
    let mut encode = None;
    let mut filter = None;
    let mut strategy = None;
    let mut destination_name = None;
    for (key, value) in &params {
        let s = &mut o.security;
        match key.as_str() {
            "Description" => o.description = value.text().unwrap_or_default(),
            "CompatibilityLevel" => {
                if let Some(level) = value.num() {
                    o.compatibility = Compatibility::from_level(level);
                }
            }
            "CheckCompliance" => {
                let first = match value {
                    Value::Array(items) => items.first().and_then(Value::name),
                    other => other.name(),
                };
                o.standard = Standard::ALL
                    .into_iter()
                    .find(|st| Some(st.compliance()) == first)
                    .unwrap_or(Standard::None);
            }
            "DownsampleColorImages" => downsample_on = value.flag(),
            "ColorImageDownsampleType" => downsample_kind = value.name().map(str::to_string),
            "ColorImageResolution" => {
                if let Some(ppi) = value.num() {
                    o.downsample_ppi = ppi;
                }
            }
            "ColorImageDownsampleThreshold" => threshold = value.num(),
            "EncodeColorImages" => encode = value.flag(),
            "ColorImageFilter" => filter = value.name().map(str::to_string),
            "ColorImageDict" => {
                if let Value::Dict(entries) = value {
                    if let Some(q) = entries
                        .iter()
                        .find(|(k, _)| k == "QFactor")
                        .and_then(|(_, v)| v.num())
                    {
                        o.quality = Quality::from_qfactor(q);
                    }
                }
            }
            "JPEG2000ColorImageDict" => {
                if let Value::Dict(entries) = value {
                    if let Some(tile) = entries
                        .iter()
                        .find(|(k, _)| k == "TileWidth")
                        .and_then(|(_, v)| v.num())
                    {
                        o.tile_size = tile as u32;
                    }
                }
            }
            "ColorConversionStrategy" => strategy = value.name().map(str::to_string),
            "DestinationProfileName" => destination_name = value.text(),
            "PDFXOutputIntentProfile" => {
                o.output_intent.profile_name = value.text().unwrap_or_default()
            }
            "PDFXOutputCondition" => o.output_intent.condition = value.text().unwrap_or_default(),
            "PDFXOutputConditionIdentifier" => {
                o.output_intent.condition_identifier = value.text().unwrap_or_default()
            }
            "PDFXRegistryName" => o.output_intent.registry_name = value.text().unwrap_or_default(),
            "Optimize" => o.fast_web_view = value.flag().unwrap_or(o.fast_web_view),
            "PicturaPreserveEditing" => {
                o.preserve_editing = value.flag().unwrap_or(o.preserve_editing)
            }
            "PicturaEmbedThumbnails" => {
                o.embed_thumbnails = value.flag().unwrap_or(o.embed_thumbnails)
            }
            "PicturaViewPdf" => o.view_after_saving = value.flag().unwrap_or(o.view_after_saving),
            "PicturaConvert16To8" => o.convert_16_to_8 = value.flag().unwrap_or(o.convert_16_to_8),
            "PicturaProfileInclusion" => {
                o.profile_inclusion = if value.name() == Some("None") {
                    ProfileInclusion::None
                } else {
                    ProfileInclusion::IncludeDestination
                }
            }
            "PicturaLayers" => o.layers = value.flag().unwrap_or(o.layers),
            "PicturaNotes" => o.notes = value.flag().unwrap_or(o.notes),
            "PicturaAlphaChannels" => o.alpha_channels = value.flag().unwrap_or(o.alpha_channels),
            "PicturaIccProfile" => o.icc_profile = value.flag().unwrap_or(o.icc_profile),
            "PicturaOpenPassword" => s.open_password = value.text().unwrap_or_default(),
            "PicturaPermissionsPassword" => {
                s.permissions_password = value.text().unwrap_or_default()
            }
            "PicturaPrinting" => {
                s.printing = match value.name() {
                    Some("None") => Printing::None,
                    Some("LowResolution") => Printing::LowResolution,
                    _ => Printing::HighResolution,
                }
            }
            "PicturaChanges" => {
                s.changes = match value.name() {
                    Some("None") => Changes::None,
                    Some("PageAssembly") => Changes::PageAssembly,
                    Some("FormFill") => Changes::FormFill,
                    Some("Commenting") => Changes::Commenting,
                    _ => Changes::AnyExceptExtract,
                }
            }
            "PicturaEnableCopying" => s.copying = value.flag().unwrap_or(s.copying),
            "PicturaEnableAccessibility" => {
                s.accessibility = value.flag().unwrap_or(s.accessibility)
            }
            "PicturaPlaintextMetadata" => {
                s.plaintext_metadata = value.flag().unwrap_or(s.plaintext_metadata)
            }
            _ => {}
        }
    }
    o.downsample = match (downsample_on, downsample_kind.as_deref()) {
        (Some(false), _) => Downsample::None,
        (_, Some("Average")) => Downsample::Average,
        (_, Some("Subsample")) => Downsample::Subsample,
        (Some(true), _) | (_, Some("Bicubic")) => Downsample::Bicubic,
        _ => o.downsample,
    };
    if let Some(ratio) = threshold {
        // The file stores a four-decimal ratio; round back to the ppi shown.
        o.downsample_above_ppi = (o.downsample_ppi * ratio * 100.0).round() / 100.0;
    }
    o.compression = match (encode, filter.as_deref()) {
        (Some(false), _) => Compression::None,
        (_, Some("DCTEncode")) => Compression::Jpeg,
        (_, Some("JPXEncode")) => Compression::Jpeg2000,
        (_, Some("FlateEncode")) => Compression::Zip,
        _ => o.compression,
    };
    if let Some(strategy) = strategy {
        if strategy == "LeaveColorUnchanged" {
            o.color_conversion = ColorConversion::None;
        } else {
            o.color_conversion = ColorConversion::ToDestination;
            o.destination = destination_name
                .as_deref()
                .and_then(|name| Destination::ALL.into_iter().find(|d| d.label() == name))
                .unwrap_or(match strategy.as_str() {
                    "CMYK" => Destination::CoatedCmyk,
                    "Gray" => Destination::GrayGamma22,
                    _ => Destination::Srgb,
                });
        }
    }
    o.normalize();
    Some(o)
}

/// A PostScript string literal for `text` (UTF-16BE hex beyond ASCII).
fn ps_string(text: &str) -> String {
    if text.bytes().all(|b| (0x20..0x7f).contains(&b)) {
        let mut out = String::from("(");
        for c in text.chars() {
            if matches!(c, '(' | ')' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push(')');
        out
    } else {
        let mut out = String::from("<FEFF");
        for unit in text.encode_utf16() {
            out.push_str(&format!("{unit:04X}"));
        }
        out.push('>');
        out
    }
}

fn ps_bool(on: bool) -> &'static str {
    if on {
        "true"
    } else {
        "false"
    }
}

/// The canonical `.joboptions` text for `options`. With `preset` the Save
/// options and the Security page are left out: presets never store passwords.
pub fn write_joboptions(options: &PdfOptions, preset: bool) -> String {
    let o = options;
    let downsample_type = match o.downsample {
        Downsample::Average => "Average",
        Downsample::Subsample => "Subsample",
        _ => "Bicubic",
    };
    let filter = match o.compression {
        Compression::Jpeg => "DCTEncode",
        Compression::Jpeg2000 => "JPXEncode",
        _ => "FlateEncode",
    };
    let strategy = match (o.color_conversion, o.destination) {
        (ColorConversion::None, _) => "LeaveColorUnchanged",
        (_, Destination::CoatedCmyk) => "CMYK",
        (_, Destination::Srgb) => "sRGB",
        (_, Destination::GrayGamma22) => "Gray",
        _ => "RGB",
    };
    let samples = if matches!(o.quality, Quality::Maximum | Quality::High) {
        "[1 1 1 1]"
    } else {
        "[2 1 1 2]"
    };
    let mut lines = vec![
        "%!PS-Adobe-3.0".to_string(),
        "%%Creator: Kooka Pictura".to_string(),
        "<<".to_string(),
        format!("/Description << /ENU {} >>", ps_string(&o.description)),
        format!("/CompatibilityLevel 1.{}", o.compatibility.minor()),
        format!("/CheckCompliance [/{}]", o.standard.compliance()),
        format!(
            "/DownsampleColorImages {}",
            ps_bool(o.downsample != Downsample::None)
        ),
        format!("/ColorImageDownsampleType /{downsample_type}"),
        format!("/ColorImageResolution {}", real(o.downsample_ppi)),
        format!(
            "/ColorImageDownsampleThreshold {}",
            real(o.downsample_above_ppi / o.downsample_ppi.max(1.0))
        ),
        format!(
            "/EncodeColorImages {}",
            ps_bool(o.compression != Compression::None)
        ),
        format!("/ColorImageFilter /{filter}"),
        "/AutoFilterColorImages false".to_string(),
        format!(
            "/ColorImageDict << /QFactor {} /HSamples {samples} /VSamples {samples} >>",
            real(o.quality.qfactor())
        ),
        format!(
            "/JPEG2000ColorImageDict << /TileWidth {0} /TileHeight {0} /Quality 30 >>",
            o.tile_size
        ),
        format!("/ColorConversionStrategy /{strategy}"),
        format!(
            "/DestinationProfileName {}",
            ps_string(o.destination.label())
        ),
        format!(
            "/PicturaProfileInclusion /{}",
            match o.profile_inclusion {
                ProfileInclusion::IncludeDestination => "IncludeDestination",
                ProfileInclusion::None => "None",
            }
        ),
        format!(
            "/PDFXOutputIntentProfile {}",
            ps_string(&o.output_intent.profile_name)
        ),
        format!(
            "/PDFXOutputCondition {}",
            ps_string(&o.output_intent.condition)
        ),
        format!(
            "/PDFXOutputConditionIdentifier {}",
            ps_string(&o.output_intent.condition_identifier)
        ),
        format!(
            "/PDFXRegistryName {}",
            ps_string(&o.output_intent.registry_name)
        ),
        "/PDFXTrapped /False".to_string(),
        format!("/Optimize {}", ps_bool(o.fast_web_view)),
        format!("/PicturaPreserveEditing {}", ps_bool(o.preserve_editing)),
        format!("/PicturaEmbedThumbnails {}", ps_bool(o.embed_thumbnails)),
        format!("/PicturaViewPdf {}", ps_bool(o.view_after_saving)),
        format!("/PicturaConvert16To8 {}", ps_bool(o.convert_16_to_8)),
    ];
    if !preset {
        let s = &o.security;
        lines.extend([
            format!("/PicturaLayers {}", ps_bool(o.layers)),
            format!("/PicturaNotes {}", ps_bool(o.notes)),
            format!("/PicturaAlphaChannels {}", ps_bool(o.alpha_channels)),
            format!("/PicturaIccProfile {}", ps_bool(o.icc_profile)),
            format!("/PicturaOpenPassword {}", ps_string(&s.open_password)),
            format!(
                "/PicturaPermissionsPassword {}",
                ps_string(&s.permissions_password)
            ),
            format!("/PicturaPrinting /{:?}", s.printing),
            format!(
                "/PicturaChanges /{}",
                match s.changes {
                    Changes::None => "None",
                    Changes::PageAssembly => "PageAssembly",
                    Changes::FormFill => "FormFill",
                    Changes::Commenting => "Commenting",
                    Changes::AnyExceptExtract => "AnyExceptExtract",
                }
            ),
            format!("/PicturaEnableCopying {}", ps_bool(s.copying)),
            format!("/PicturaEnableAccessibility {}", ps_bool(s.accessibility)),
            format!(
                "/PicturaPlaintextMetadata {}",
                ps_bool(s.plaintext_metadata)
            ),
        ]);
    }
    lines.push(">> setdistillerparams".to_string());
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Whether two option sets agree on every preset setting (the Save options
/// and Security are not part of a preset).
pub fn same_preset(a: &PdfOptions, b: &PdfOptions) -> bool {
    write_joboptions(a, true) == write_joboptions(b, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joboptions_round_trip() {
        let mut options = PdfOptions::default();
        for (field, value) in [
            ("description", "Proof — für Druck (v2)"),
            ("standard", "3"),
            ("downsample", "2"),
            ("downsamplePpi", "150"),
            ("downsampleAbovePpi", "225"),
            ("compression", "2"),
            ("quality", "1"),
            ("colorConversion", "1"),
            ("destination", "1"),
            ("profileInclusion", "1"),
            ("outputCondition", "My press (proof)"),
            ("fastWebView", "1"),
            ("preserveEditing", "0"),
            ("embedThumbnails", "0"),
            ("notes", "0"),
        ] {
            assert!(options.set(field, value), "{field}");
        }
        let text = write_joboptions(&options, false);
        assert_eq!(parse_joboptions(&text), Some(options.clone()));
        let preset = parse_joboptions(&write_joboptions(&options, true)).unwrap();
        assert!(same_preset(&preset, &options));
    }

    #[test]
    fn joboptions_preset_drops_security() {
        let mut options = PdfOptions::default();
        options.set("openPassword", "kooka");
        options.set("permissionsPassword", "owner");
        let text = write_joboptions(&options, true);
        assert!(!text.contains("kooka") && !text.contains("owner"));
        assert!(!text.contains("Password") && !text.contains("PicturaPrinting"));
        let full = write_joboptions(&options, false);
        assert!(full.contains("/PicturaOpenPassword (kooka)"));
    }

    #[test]
    fn joboptions_parses_acrobat_file() {
        let text = r"%!PS-Adobe-3.0
%%Title: a preset (with a comment)
<<
  /ASCII85EncodePages false
  /AllowTransparency false
  /ColorImageDict <<
    /QFactor 0.76
    /HSamples [2 1 1 2] /VSamples [2 1 1 2]
  >>
  /ColorImageDownsampleThreshold 1.50000
  /ColorImageDownsampleType /Bicubic
  /ColorImageResolution 150
  /CompatibilityLevel 1.4
  /DownsampleColorImages true
  /ColorImageFilter /DCTEncode
  /ColorConversionStrategy /sRGB
  /Description <<
    /DEU <FEFF0044006500750074007300630068>
    /ENU (Standard \(web\) settings)
  >>
  /OtherNamespaces [ << /AsReaderSpreads false /Namespace [ (Adobe) (InDesign) (4.0) ] >> ]
  /UnknownKey { 1 2 add }
>> setdistillerparams
<< /HWResolution [600 600] /PageSize [612 792] >> setpagedevice
";
        let options = parse_joboptions(text).expect("parses");
        assert_eq!(options.description, "Standard (web) settings");
        assert_eq!(options.compatibility, Compatibility::Acrobat5);
        assert_eq!(options.downsample, Downsample::Bicubic);
        assert_eq!(options.downsample_ppi, 150.0);
        assert_eq!(options.downsample_above_ppi, 225.0);
        assert_eq!(options.compression, Compression::Jpeg);
        assert_eq!(options.quality, Quality::Medium);
        assert_eq!(options.color_conversion, ColorConversion::ToDestination);
        assert_eq!(options.destination, Destination::Srgb);
        assert_eq!(parse_joboptions("% only a comment\n"), None);
    }
}
