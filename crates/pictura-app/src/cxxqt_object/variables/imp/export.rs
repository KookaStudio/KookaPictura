use super::{evaluate, ReplacementImage};
use pictura_core::Document;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct BatchResult {
    pub(super) written: Vec<PathBuf>,
    pub(super) skipped: Vec<String>,
    pub(super) failure: Option<String>,
    pub(super) cancelled: bool,
}

fn pattern_name(pattern: &str, base: &str, set: &str, index: usize) -> String {
    let mut out = String::new();
    let mut rest = if pattern.is_empty() {
        "{base}_{set}"
    } else {
        pattern
    };
    let number = index.to_string();
    while !rest.is_empty() {
        if let Some((token, text)) = [
            ("{base}", base),
            ("{set}", set),
            ("{index}", number.as_str()),
        ]
        .into_iter()
        .find(|(token, _)| rest.starts_with(token))
        {
            out.push_str(text);
            rest = &rest[token.len()..];
        } else if let Some(ch) = rest.chars().next() {
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    let stem: String = out
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let stem = stem.trim_matches([' ', '.']);
    if stem.is_empty() {
        "Data Set".into()
    } else {
        stem.into()
    }
}

fn unique_name(stem: &str, used: &mut BTreeSet<String>) -> String {
    for index in 1.. {
        let suffix = if index == 1 {
            ".psd".to_string()
        } else {
            format!(" {index}.psd")
        };
        // Reserve room for the sibling temporary file's suffix.
        let mut cut = stem.len().min(231 - suffix.len());
        while !stem.is_char_boundary(cut) {
            cut -= 1;
        }
        let name = format!("{}{suffix}", stem[..cut].trim_end_matches([' ', '.']));
        if used.insert(name.to_lowercase()) {
            return name;
        }
    }
    unreachable!()
}

pub(super) struct ExportBatch {
    doc: Document,
    pub(super) targets: Vec<(usize, PathBuf)>,
    pub(super) cursor: usize,
    pub(super) result: BatchResult,
}

impl ExportBatch {
    pub(super) fn new(
        doc: &Document,
        folder: &Path,
        selected: &[usize],
        base: &str,
        pattern: &str,
    ) -> Result<Self, String> {
        let mut used = BTreeSet::new();
        for entry in std::fs::read_dir(folder)
            .map_err(|e| format!("Cannot use output folder {}: {e}", folder.display()))?
        {
            let entry = entry
                .map_err(|e| format!("Cannot list output folder {}: {e}", folder.display()))?;
            used.insert(entry.file_name().to_string_lossy().to_lowercase());
        }
        let selected: BTreeSet<_> = selected.iter().copied().collect();
        let targets = doc
            .variables
            .sets()
            .iter()
            .enumerate()
            .filter(|(index, _)| selected.contains(index))
            .map(|(index, set)| {
                let stem = pattern_name(pattern, base, &set.name, index + 1);
                (index, folder.join(unique_name(&stem, &mut used)))
            })
            .collect::<Vec<_>>();
        if targets.len() != selected.len() || targets.is_empty() {
            return Err("Choose at least one existing data set".into());
        }
        Ok(Self {
            doc: doc.clone(),
            targets,
            cursor: 0,
            result: BatchResult::default(),
        })
    }

    pub(super) fn done(&self) -> bool {
        self.cursor == self.targets.len() || self.result.failure.is_some() || self.result.cancelled
    }

    pub(super) fn step(&mut self, load: impl Fn(&str) -> Result<ReplacementImage, String>) {
        self.step_with_writer(load, write_psd);
    }

    pub(super) fn step_with_writer(
        &mut self,
        load: impl Fn(&str) -> Result<ReplacementImage, String>,
        write: impl FnOnce(Document, &Path) -> Result<(), ExportError>,
    ) {
        if self.done() {
            return;
        }
        let (index, target) = &self.targets[self.cursor];
        let set = &self.doc.variables.sets()[*index].name;
        self.cursor += 1;
        match evaluate(&self.doc, *index, load) {
            Ok(result) => {
                let scratch = result.unwrap_or_else(|| self.doc.clone());
                match write(scratch, target) {
                    Ok(()) => self.result.written.push(target.clone()),
                    Err(ExportError::Resource(error)) => {
                        self.result.skipped.push(format!("{set}: {error}"));
                    }
                    Err(ExportError::Write(error)) => {
                        self.result.failure = Some(format!("{set} ({}): {error}", target.display()))
                    }
                }
            }
            Err(error) => self.result.skipped.push(format!("{set}: {error}")),
        }
    }
}

#[cfg(test)]
pub(super) fn export_sets(
    doc: &Document,
    folder: &Path,
    selected: &[usize],
    base: &str,
    pattern: &str,
    load: impl Fn(&str) -> Result<ReplacementImage, String>,
    mut progress: impl FnMut(usize) -> bool,
) -> BatchResult {
    let mut batch = match ExportBatch::new(doc, folder, selected, base, pattern) {
        Ok(batch) => batch,
        Err(error) => {
            return BatchResult {
                failure: Some(error),
                ..BatchResult::default()
            }
        }
    };
    while !batch.done() {
        if !progress(batch.cursor) {
            batch.result.cancelled = true;
            break;
        }
        batch.step(&load);
    }
    batch.result
}

pub(super) enum ExportError {
    Resource(String),
    Write(String),
}

fn write_psd(mut scratch: Document, path: &Path) -> Result<(), ExportError> {
    if scratch.width > 30_000 || scratch.height > 30_000 {
        return Err(ExportError::Resource(
            "The template exceeds PSD dimensions".into(),
        ));
    }
    let _reservation = pictura_core::memory::governor()
        .reserve(
            crate::cxxqt_object::helpers::save_gate::save_cost(&scratch).working,
            "a data set export",
        )
        .map_err(|_| ExportError::Resource("The data set exceeds the save memory budget".into()))?;
    scratch.is_psb = false;
    scratch.composite.set_merged_output(true);
    // The shared save path reconciles native color channels and any retained
    // composite with the new layer content before serializing.
    super::refresh_export_composite(&mut scratch).map_err(ExportError::Resource)?;
    crate::cxxqt_object::helpers::safe_io::write_new_atomically_with(path, |out| {
        pictura_codec::write_psd_to(out, &scratch, false, None)
            .map_err(|error| std::io::Error::other(error.to_string()))
    })
    .map_err(|error| ExportError::Write(error.to_string()))
}
