//! Batch File Naming: CS6's ordered name fields evaluated per input.

use serde::{Deserialize, Serialize};

/// The CS6 File Naming menu labels, in menu order. A field whose text is not
/// one of these (and not empty) is custom text.
pub const TOKEN_LABELS: &[&str] = &[
    "Document Name",
    "document name",
    "DOCUMENT NAME",
    "1 Digit Serial Number",
    "2 Digit Serial Number",
    "3 Digit Serial Number",
    "4 Digit Serial Number",
    "Serial Letter (a, b, c...)",
    "Serial Letter (A, B, C...)",
    "mmddyy (date)",
    "mmdd (date)",
    "yyyymmdd (date)",
    "yymmdd (date)",
    "yyddmm (date)",
    "ddmmyy (date)",
    "ddmm (date)",
    "extension",
    "EXTENSION",
];

/// The number of field slots the dialog shows.
pub const MAX_FIELDS: usize = 6;

/// The letter case a document-name token writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    AsIs,
    Lower,
    Upper,
}

/// One evaluated name field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamingField {
    Empty,
    Text(String),
    DocumentName(Case),
    Serial(usize),
    SerialLetter { upper: bool },
    Date(&'static str),
    Extension { upper: bool },
}

impl NamingField {
    /// The field for a combo-box text: an exact token label or custom text.
    pub fn parse(text: &str) -> Self {
        match text {
            "" => Self::Empty,
            "Document Name" => Self::DocumentName(Case::AsIs),
            "document name" => Self::DocumentName(Case::Lower),
            "DOCUMENT NAME" => Self::DocumentName(Case::Upper),
            "1 Digit Serial Number" => Self::Serial(1),
            "2 Digit Serial Number" => Self::Serial(2),
            "3 Digit Serial Number" => Self::Serial(3),
            "4 Digit Serial Number" => Self::Serial(4),
            "Serial Letter (a, b, c...)" => Self::SerialLetter { upper: false },
            "Serial Letter (A, B, C...)" => Self::SerialLetter { upper: true },
            "extension" => Self::Extension { upper: false },
            "EXTENSION" => Self::Extension { upper: true },
            other => other
                .strip_suffix(" (date)")
                .and_then(|pattern| DATE_PATTERNS.iter().find(|p| **p == pattern))
                .map_or_else(
                    || Self::Text(other.to_string()),
                    |pattern| Self::Date(pattern),
                ),
        }
    }

    /// Whether the field distinguishes inputs (the unique-field rule).
    pub fn is_unique(&self) -> bool {
        matches!(
            self,
            Self::DocumentName(_) | Self::Serial(_) | Self::SerialLetter { .. }
        )
    }
}

const DATE_PATTERNS: &[&str] = &[
    "mmddyy", "mmdd", "yyyymmdd", "yymmdd", "yyddmm", "ddmmyy", "ddmm",
];

/// A calendar date in local time, supplied by the caller so evaluation is
/// deterministic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// The File Naming scheme. `fields` holds the combo-box texts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileNaming {
    pub fields: Vec<String>,
    pub starting_serial: u32,
    pub windows: bool,
    pub mac: bool,
}

impl Default for FileNaming {
    fn default() -> Self {
        Self {
            fields: vec!["Document Name".into(), "extension".into()],
            starting_serial: 1,
            windows: false,
            mac: false,
        }
    }
}

impl FileNaming {
    fn parsed(&self) -> Vec<NamingField> {
        self.fields
            .iter()
            .take(MAX_FIELDS)
            .map(|text| NamingField::parse(text))
            .collect()
    }

    /// At least one document-name, serial-number, or serial-letter field.
    pub fn has_unique_field(&self) -> bool {
        self.parsed().iter().any(NamingField::is_unique)
    }

    /// The output file name for the input `input_name` (a file name, its final
    /// extension stripped for the document-name tokens) at zero-based
    /// `position`, saved with extension `ext` (no dot).
    pub fn render(&self, input_name: &str, ext: &str, position: usize, date: BatchDate) -> String {
        let stem = match input_name.rsplit_once('.') {
            Some((stem, _)) if !stem.is_empty() => stem,
            _ => input_name,
        };
        let serial = u64::from(self.starting_serial.max(1)) + position as u64;
        let fields = self.parsed();
        let mut name = String::new();
        for field in &fields {
            match field {
                NamingField::Empty => {}
                NamingField::Text(text) => name.push_str(text),
                NamingField::DocumentName(Case::AsIs) => name.push_str(stem),
                NamingField::DocumentName(Case::Lower) => name.push_str(&stem.to_lowercase()),
                NamingField::DocumentName(Case::Upper) => name.push_str(&stem.to_uppercase()),
                NamingField::Serial(width) => name.push_str(&format!("{serial:0width$}")),
                NamingField::SerialLetter { upper } => {
                    let letters = serial_letters(position);
                    name.push_str(&if *upper {
                        letters.to_uppercase()
                    } else {
                        letters
                    });
                }
                NamingField::Date(pattern) => name.push_str(&format_date(pattern, date)),
                NamingField::Extension { upper } => {
                    name.push('.');
                    name.push_str(&if *upper {
                        ext.to_uppercase()
                    } else {
                        ext.to_lowercase()
                    });
                }
            }
        }
        let has_extension = fields
            .iter()
            .any(|field| matches!(field, NamingField::Extension { .. }));
        let (base, suffix) = if has_extension {
            let dotted = format!(".{ext}");
            match name.len().checked_sub(dotted.len()) {
                Some(cut)
                    if name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(&dotted) =>
                {
                    let suffix = name[cut..].to_string();
                    name.truncate(cut);
                    (name, suffix)
                }
                _ => (name, String::new()),
            }
        } else {
            (name, format!(".{}", ext.to_lowercase()))
        };
        format!("{}{suffix}", sanitize(&base, self.windows, self.mac))
    }
}

/// Zero-based position as a bijective base-26 sequence: a … z, aa, ab, …
pub fn serial_letters(position: usize) -> String {
    let mut n = position + 1;
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(b'a' + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn format_date(pattern: &str, date: BatchDate) -> String {
    let yyyy = format!("{:04}", date.year);
    let yy = format!("{:02}", date.year.rem_euclid(100));
    let mm = format!("{:02}", date.month);
    let dd = format!("{:02}", date.day);
    match pattern {
        "mmddyy" => format!("{mm}{dd}{yy}"),
        "mmdd" => format!("{mm}{dd}"),
        "yyyymmdd" => format!("{yyyy}{mm}{dd}"),
        "yymmdd" => format!("{yy}{mm}{dd}"),
        "yyddmm" => format!("{yy}{dd}{mm}"),
        "ddmmyy" => format!("{dd}{mm}{yy}"),
        "ddmm" => format!("{dd}{mm}"),
        _ => String::new(),
    }
}

/// Make a base name (no extension) safe: Unix always, plus the optional
/// Windows and Mac OS rules. `_` replaces each unsafe character.
pub fn sanitize(base: &str, windows: bool, mac: bool) -> String {
    let mut out: String = base
        .chars()
        .map(|c| {
            let unsafe_char = c == '/'
                || c == '\0'
                || (mac && c == ':')
                || (windows && (c.is_control() || "\\/:*?\"<>|".contains(c)));
            if unsafe_char {
                '_'
            } else {
                c
            }
        })
        .collect();
    if windows {
        let trimmed = out.trim_end_matches([' ', '.']).len();
        out.truncate(trimmed);
    }
    out
}
