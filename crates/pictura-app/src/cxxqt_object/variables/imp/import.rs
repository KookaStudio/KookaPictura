use pictura_core::variables::{valid_xml_text, DataSet, Value, VariableKind};
use pictura_core::Document;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Copy, Default)]
pub(super) enum Encoding {
    #[default]
    Automatic,
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
}

#[derive(Clone, Copy, Default)]
pub(super) struct ImportOptions {
    pub(super) encoding: Encoding,
    pub(super) first_names: bool,
    pub(super) replace: bool,
}

fn utf16(bytes: &[u8], little: bool) -> Result<String, String> {
    let units = bytes.chunks_exact(2);
    if !units.remainder().is_empty() {
        return Err("Invalid UTF-16 byte count".into());
    }
    char::decode_utf16(units.map(|b| {
        if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        }
    }))
    .collect::<Result<String, _>>()
    .map_err(|_| "Invalid UTF-16 text".into())
}

fn utf32(bytes: &[u8], little: bool) -> Result<String, String> {
    let units = bytes.chunks_exact(4);
    if !units.remainder().is_empty() {
        return Err("Invalid UTF-32 byte count".into());
    }
    units
        .map(|b| {
            let bytes = [b[0], b[1], b[2], b[3]];
            char::from_u32(if little {
                u32::from_le_bytes(bytes)
            } else {
                u32::from_be_bytes(bytes)
            })
            .ok_or_else(|| "Invalid UTF-32 text".into())
        })
        .collect()
}

fn windows1252(bytes: &[u8]) -> String {
    const EXTENDED: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    bytes
        .iter()
        .map(|&b| {
            if (0x80..0xa0).contains(&b) {
                EXTENDED[usize::from(b - 0x80)]
            } else {
                b.into()
            }
        })
        .collect()
}

fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, String> {
    match encoding {
        Encoding::Automatic => {
            for (bom, little) in [
                (&[0xff, 0xfe, 0, 0][..], true),
                (&[0, 0, 0xfe, 0xff][..], false),
            ] {
                if let Some(data) = bytes.strip_prefix(bom) {
                    return utf32(data, little);
                }
            }
            if bytes.starts_with(&[0xff, 0xfe]) {
                return decode(bytes, Encoding::Utf16Le);
            }
            if bytes.starts_with(&[0xfe, 0xff]) {
                return decode(bytes, Encoding::Utf16Be);
            }
            if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                return decode(bytes, Encoding::Utf8);
            }
            Ok(std::str::from_utf8(bytes).map_or_else(|_| windows1252(bytes), str::to_owned))
        }
        Encoding::Utf8 => {
            std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
                .map(str::to_owned)
                .map_err(|_| "Invalid UTF-8 text".into())
        }
        Encoding::Utf16Le => utf16(bytes.strip_prefix(&[0xff, 0xfe]).unwrap_or(bytes), true),
        Encoding::Utf16Be => utf16(bytes.strip_prefix(&[0xfe, 0xff]).unwrap_or(bytes), false),
        Encoding::Windows1252 => Ok(windows1252(bytes)),
    }
}

fn delimiter(source: &str, path: &Path) -> Result<char, String> {
    let (mut commas, mut tabs, mut quoted) = (false, false, false);
    for ch in source.chars() {
        match ch {
            '"' => quoted = !quoted,
            '\r' | '\n' if !quoted => break,
            ',' if !quoted => commas = true,
            '\t' if !quoted => tabs = true,
            _ => {}
        }
    }
    match (commas, tabs) {
        (true, false) => Ok(','),
        (false, true) => Ok('\t'),
        (true, true) => Err("The header contains both commas and tabs".into()),
        _ => match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "csv" => Ok(','),
            "tsv" | "txt" => Ok('\t'),
            _ => Err("Use a CSV, TSV, or TXT extension for a single-column data file".into()),
        },
    }
}

struct Rows<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    delimiter: char,
    index: usize,
}

impl Rows<'_> {
    fn next(&mut self) -> Result<Option<Vec<String>>, String> {
        if self.chars.peek().is_none() {
            return Ok(None);
        }
        self.index += 1;
        let mut row = Vec::new();
        loop {
            while self.chars.peek() == Some(&' ') {
                self.chars.next();
            }
            let mut field = String::new();
            if self.chars.peek() == Some(&'"') {
                self.chars.next();
                loop {
                    match self.chars.next() {
                        Some('"') if self.chars.peek() == Some(&'"') => {
                            self.chars.next();
                            field.push('"');
                        }
                        Some('"') => break,
                        Some(ch) => field.push(ch),
                        None => {
                            return Err(format!(
                                "Row {}, column {} has an unclosed quote",
                                self.index,
                                row.len() + 1
                            ))
                        }
                    }
                }
                while self.chars.peek() == Some(&' ') {
                    self.chars.next();
                }
            } else {
                while let Some(ch) = self.chars.peek().copied() {
                    if ch == self.delimiter || matches!(ch, '\r' | '\n') {
                        break;
                    }
                    if ch == '"' {
                        return Err(format!(
                            "Row {}, column {} has an unexpected quote",
                            self.index,
                            row.len() + 1
                        ));
                    }
                    field.push(ch);
                    self.chars.next();
                }
                field.truncate(field.trim_end_matches(' ').len());
            }
            row.push(field);
            match self.chars.next() {
                Some(ch) if ch == self.delimiter => {}
                Some('\r') => {
                    if self.chars.peek() == Some(&'\n') {
                        self.chars.next();
                    }
                    break;
                }
                Some('\n') | None => break,
                _ => {
                    return Err(format!(
                        "Row {}, column {} has characters after a quoted value",
                        self.index,
                        row.len()
                    ))
                }
            }
        }
        Ok(Some(row))
    }
}

pub(super) fn import_sets(
    doc: &mut Document,
    path: &Path,
    options: ImportOptions,
) -> Result<bool, String> {
    if doc.variables.definitions().is_empty() {
        return Err("Define variables before importing data sets".into());
    }
    let bytes = crate::cxxqt_object::helpers::safe_io::read_capped(path, 16 * 1024 * 1024)
        .ok_or_else(|| format!("Cannot read data set file {}", path.display()))?;
    let source = decode(&bytes, options.encoding)?;
    let mut rows = Rows {
        chars: source.chars().peekable(),
        delimiter: delimiter(&source, path)?,
        index: 0,
    };
    let header = rows.next()?.ok_or("The data set file has no header")?;
    let header_names: BTreeSet<_> = header.iter().map(String::as_str).collect();
    let variable_names: BTreeSet<_> = doc
        .variables
        .definitions()
        .keys()
        .map(String::as_str)
        .collect();
    if header.len() != header_names.len() {
        return Err("Row 1 contains a duplicate variable name".into());
    }
    if header_names != variable_names {
        let missing: Vec<_> = variable_names.difference(&header_names).copied().collect();
        let extra: Vec<_> = header_names.difference(&variable_names).copied().collect();
        return Err(format!(
            "Row 1 does not match the template variables. Missing: {}. Extra: {}.",
            missing.join(", "),
            extra.join(", ")
        ));
    }
    let columns: Vec<_> = header
        .iter()
        .map(|name| &doc.variables.definitions()[name])
        .collect();
    let folder = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut used = BTreeSet::new();
    if !options.replace {
        used.extend(doc.variables.sets().iter().map(|set| set.name.clone()));
    }
    let mut output = Vec::new();
    let mut next = 1;
    while let Some(row) = rows.next()? {
        if row.len() != columns.len() {
            return Err(format!(
                "Row {} has {} columns, expected {}",
                rows.index,
                row.len(),
                columns.len()
            ));
        }
        let name = if options.first_names {
            row[0].clone()
        } else {
            loop {
                let name = format!("Data Set {next}");
                next += 1;
                if !used.contains(&name) {
                    break name;
                }
            }
        };
        if name.trim().is_empty() || !valid_xml_text(&name) || !used.insert(name.clone()) {
            return Err(format!(
                "Row {} has an invalid or duplicate data set name",
                rows.index
            ));
        }
        let mut values = BTreeMap::new();
        for (index, (field, variable)) in row.into_iter().zip(&columns).enumerate() {
            let value = match variable.kind {
                VariableKind::Visibility if field == "true" || field == "false" => {
                    Some(Value::Visibility(field == "true"))
                }
                VariableKind::Text if valid_xml_text(&field) => Some(Value::Text(field)),
                VariableKind::Pixels if !field.trim().is_empty() && valid_xml_text(&field) => {
                    let referenced = Path::new(&field);
                    let value = if referenced.is_absolute()
                        || field.as_bytes().get(1) == Some(&b':')
                        || field.starts_with("\\\\")
                    {
                        field
                    } else {
                        folder
                            .parent()
                            .ok_or("Data set file has no parent directory")?
                            .join(referenced)
                            .to_str()
                            .ok_or("Invalid image path encoding")?
                            .to_owned()
                    };
                    Some(Value::File(value))
                }
                _ => None,
            }
            .ok_or_else(|| {
                format!(
                    "Row {}, column {} has an invalid value",
                    rows.index,
                    index + 1
                )
            })?;
            values.insert(variable.id, value);
        }
        output.push(DataSet { name, values });
    }
    if output.is_empty() {
        return Err("The file contains no data sets".into());
    }
    doc.variables.import_sets(output, options.replace)
}
